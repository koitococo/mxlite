use std::collections::HashMap;

use log::debug;
use reqwest::Method;

use crate::script::value_type::ValueType;


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

impl From<LuaFetchResponse> for ValueType {
  fn from(val: LuaFetchResponse) -> Self {
    let mut table = HashMap::new();
    table.insert("ok".to_string(), ValueType::Boolean(val.ok));
    table.insert("status".to_string(), ValueType::Integer(val.status as i64));
    if let Some(v) = val.status_text {
      table.insert("statusText".to_string(), ValueType::String(v));
    }
    if let Some(v) = val.headers {
      let mut headers = HashMap::new();
      for (k, v) in v {
        headers.insert(k, ValueType::String(v));
      }
      table.insert("headers".to_string(), ValueType::Table(headers));
    }
    if let Some(v) = val.text {
      table.insert("text".to_string(), ValueType::String(v));
    }
    if let Some(v) = val.json {
      table.insert("json".to_string(), v);
    }
    if let Some(v) = val.length {
      table.insert("length".to_string(), ValueType::Integer(v as i64));
    }
    if let Some(v) = val.error {
      table.insert("error".to_string(), ValueType::String(v));
    }
    ValueType::Table(table)
  }
}

async fn lua_fetch(_: mlua::Lua, req: ValueType) -> mlua::Result<ValueType> {
  debug!("lua_fetch: {req:?}");
  let req: LuaFetchRequst = req.try_into()?;
  let client = reqwest::Client::new();
  let mut builder = client.request(
    {
      if let Some(v) = req.method {
        match v.to_uppercase().as_str() {
          "GET" => Method::GET,
          "POST" => Method::POST,
          "PUT" => Method::PUT,
          "DELETE" => Method::DELETE,
          "PATCH" => Method::PATCH,
          _ => return Err(mlua::Error::RuntimeError(format!("Unsupported HTTP method: {v}"))),
        }
      } else {
        Method::GET
      }
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
  debug!("lua_fetch: constructed request: {builder:?}");
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

pub(super) fn register(lua: &mlua::Lua, f_table: &mlua::Table) -> mlua::Result<()> {
  f_table.set("fetch", lua.create_async_function(lua_fetch)?)?;
  Ok(())
}