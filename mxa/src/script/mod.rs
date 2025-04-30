use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use log::debug;
use mlua::{FromLuaMulti, IntoLuaMulti, Lua, MultiValue, StdLib, Value};
use reqwest::Method;

mod value_type;
use self::value_type::ValueType;
pub struct Values(Vec<ValueType>);

impl FromLuaMulti for Values {
  fn from_lua_multi(mt: MultiValue, _: &Lua) -> mlua::Result<Self> {
    let mut vals = Vec::with_capacity(mt.len());
    for v in mt {
      vals.push(ValueType::try_from_lua(v)?);
    }
    Ok(Values(vals))
  }
}

impl IntoLuaMulti for Values {
  fn into_lua_multi(self, lua: &Lua) -> mlua::Result<MultiValue> {
    let mut vals = Vec::with_capacity(self.0.len());
    for v in self.0 {
      vals.push(v.try_into_lua(lua)?);
    }
    Ok(vals.into())
  }
}

pub trait Func {
  fn call(&self, lua: &Lua, args: Values) -> mlua::Result<Values>;
}
type FuncObj = Arc<Box<dyn Func + Send + Sync>>;

pub struct ExecutorContext {
  lua: Lua,
  func: HashMap<String, FuncObj>,
  inited: bool,
}

impl ExecutorContext {
  pub fn new() -> Self {
    let lua = Lua::new();
    Self {
      lua,
      func: HashMap::new(),
      inited: false,
    }
  }

  pub fn register_func(&mut self, name: &str, boxed: Box<dyn Func + Send + Sync>) -> Result<()> {
    if self.inited {
      return Err(anyhow::anyhow!("Cannot register function after initialization"));
    }
    if self.func.contains_key(name) {
      return Err(anyhow::anyhow!("Function {name} already registered"));
    }
    let f = Arc::new(boxed);
    self.func.insert(name.to_string(), f);
    Ok(())
  }

  pub fn init(&mut self) -> Result<()> {
    if self.inited {
      return Ok(());
    }
    self.lua.load_std_libs(StdLib::ALL_SAFE)?;
    let f_table = self.lua.create_table()?;
    for (f_name, f) in self.func.iter() {
      let f = f.clone();
      f_table.set(
        f_name.clone(),
        self.lua.create_function(move |lua, args: Values| f.call(lua, args))?,
      )?;
    }
    f_table.set("json", mod_json(&self.lua)?)?;
    f_table.set("fetch", self.lua.create_async_function(lua_fetch)?)?;
    self.lua.globals().set("mxa", f_table)?;
    self.inited = true;
    Ok(())
  }

  pub async fn eval_async(&mut self, script: &str) -> Result<()> {
    if !self.inited {
      return Err(anyhow::anyhow!("ExecutorContext not initialized"));
    }
    self.lua.load(script).exec_async().await?;
    Ok(())
  }
}

#[inline]
fn mod_json(lua: &Lua) -> mlua::Result<mlua::Table> {
  let json = lua.create_table()?;
  json.set("encode", lua.create_function(lua_json_encode)?)?;
  json.set("decode", lua.create_function(lua_json_decode)?)?;
  Ok(json)
}

fn lua_json_encode(_: &Lua, vt: Value) -> mlua::Result<String> {
  let v = ValueType::try_from_lua(vt)?;
  let json = match serde_json::to_string(&v) {
    Ok(v) => v,
    Err(e) => {
      return Err(mlua::Error::RuntimeError(format!("Failed to serialize to JSON: {}", e)));
    }
  };
  Ok(json)
}

fn lua_json_decode(lua: &Lua, s: String) -> mlua::Result<Value> {
  let v: ValueType = match serde_json::from_str(&s).ok() {
    Some(v) => v,
    None => {
      return Err(mlua::Error::RuntimeError(format!("Failed to parse JSON: {}", s)));
    }
  };
  let vt = v.try_into_lua(lua)?;
  Ok(vt)
}

#[derive(Debug, Clone)]
enum LuaFetchOutput {
  Text,
  Json,
}

#[derive(Debug, Clone)]
struct LuaFetchRequst {
  url: String,
  method: Option<String>,
  headers: Option<Vec<(String, String)>>,
  body: Option<String>,
  output: Option<LuaFetchOutput>,
}

impl TryInto<LuaFetchRequst> for ValueType {
  type Error = mlua::Error;

  fn try_into(self) -> mlua::Result<LuaFetchRequst> {
    let url: Option<String> = self.try_table_val_typed("url");
    let Some(url) = url else {
      return Err(mlua::Error::RuntimeError("url is required".to_string()));
    };
    let method: Option<String> = self.try_table_val_typed("method");
    let headers: Option<HashMap<String, ValueType>> = self.try_table_val_typed("headers");
    let headers: Option<Vec<(String, String)>> = match headers {
      Some(v) => {
        let r = v
          .into_iter()
          .filter_map(|(k, v)| match v {
            ValueType::String(v) => Some((k, v)),
            _ => None,
          })
          .collect::<Vec<(String, String)>>();
        Some(r)
      }
      None => None,
    };
    let body: Option<String> = self.try_table_val_typed("body");
    let output: Option<String> = self.try_table_val_typed("output");
    let output: Option<LuaFetchOutput> = match output {
      Some(v) => match v.as_str() {
        "text" => Some(LuaFetchOutput::Text),
        "json" => Some(LuaFetchOutput::Json),
        _ => None,
      },
      None => None,
    };

    Ok(LuaFetchRequst {
      url,
      method,
      headers,
      body,
      output,
    })
  }
}

#[allow(dead_code)] // Exception: this struct is exposed to Lua, and the fields are possibly used in Lua code
#[derive(Debug, Clone)]
struct LuaFetchResponse {
  ok: bool,
  status: u16,
  status_text: Option<String>,
  headers: Option<HashMap<String, String>>,
  text: Option<String>,
  json: Option<ValueType>,
  length: Option<u64>,
  output: LuaFetchOutput,
  error: Option<String>,
}

impl Into<ValueType> for LuaFetchResponse {
  fn into(self) -> ValueType {
    let mut table = HashMap::new();
    table.insert("ok".to_string(), ValueType::Boolean(self.ok));
    table.insert("status".to_string(), ValueType::Integer(self.status as i64));
    if let Some(v) = self.status_text {
      table.insert("statusText".to_string(), ValueType::String(v));
    }
    if let Some(v) = self.headers {
      let mut headers = HashMap::new();
      for (k, v) in v {
        headers.insert(k, ValueType::String(v));
      }
      table.insert("headers".to_string(), ValueType::Table(headers));
    }
    if let Some(v) = self.text {
      table.insert("text".to_string(), ValueType::String(v));
    }
    if let Some(v) = self.json {
      table.insert("json".to_string(), v);
    }
    if let Some(v) = self.length {
      table.insert("length".to_string(), ValueType::Integer(v as i64));
    }
    if let Some(v) = self.error {
      table.insert("error".to_string(), ValueType::String(v));
    }
    ValueType::Table(table)
  }
}

async fn lua_fetch(_: Lua, req: ValueType) -> mlua::Result<ValueType> {
  debug!("lua_fetch: {:?}", req);
  let req: LuaFetchRequst = req.try_into()?;
  let client = reqwest::Client::new();
  let mut builder = client.request(
    match req.method {
      _ => Method::GET,
    },
    req.url,
  );
  if let Some(headers) = req.headers {
    for (k, v) in headers {
      builder = builder.header(k, v);
    }
  }
  if let Some(body) = req.body {
    builder = builder.body(body);
  }
  let output = req.output.unwrap_or(LuaFetchOutput::Text);
  debug!("lua_fetch: constructed request: {:?}", builder);
  let response = match builder.send().await {
    Ok(r) => r,
    Err(e) => {
      return Ok(
        LuaFetchResponse {
          ok: false,
          status: 0,
          status_text: None,
          headers: None,
          text: None,
          json: None,
          length: None,
          output,
          error: Some(e.to_string()),
        }
        .into(),
      );
    }
  };
  debug!("lua_fetch: parsing response");
  let status = response.status().as_u16();
  let status_text = response.status().canonical_reason().map(|s| s.to_string());
  let mut header_map = HashMap::new();
  for (k, v) in response.headers() {
    header_map.insert(k.to_string(), v.to_str().unwrap_or("").to_string());
  }
  let mut r = LuaFetchResponse {
    ok: response.status().is_success(),
    status,
    status_text,
    headers: Some(header_map),
    text: None,
    json: None,
    length: None,
    output: output.clone(),
    error: None,
  };
  let text = match response.text().await {
    Ok(v) => v,
    Err(e) => {
      r.ok = false;
      r.error = Some(e.to_string());
      return Ok(r.into());
    }
  };
  match output {
    LuaFetchOutput::Text => {
      r.text = Some(text);
    }
    LuaFetchOutput::Json => {
      let json: ValueType = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
          r.ok = false;
          r.error = Some(e.to_string());
          return Ok(r.into());
        }
      };
      r.json = Some(json);
    }
  }
  Ok(r.into())
}
