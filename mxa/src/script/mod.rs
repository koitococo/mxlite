use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use mlua::{FromLuaMulti, IntoLuaMulti, Lua, MultiValue, StdLib};
mod libs;
mod value_type;
use self::value_type::ValueType;
pub struct VecValue(Vec<ValueType>);

impl FromLuaMulti for VecValue {
  fn from_lua_multi(mt: MultiValue, _: &Lua) -> mlua::Result<Self> {
    let mut vals = Vec::with_capacity(mt.len());
    for v in mt {
      vals.push(ValueType::try_from_lua(v)?);
    }
    Ok(VecValue(vals))
  }
}

impl IntoLuaMulti for VecValue {
  fn into_lua_multi(self, lua: &Lua) -> mlua::Result<MultiValue> {
    let mut vals = Vec::with_capacity(self.0.len());
    for v in self.0 {
      vals.push(v.try_into_lua(lua)?);
    }
    Ok(vals.into())
  }
}

pub trait Func {
  fn call(&self, lua: &Lua, args: VecValue) -> mlua::Result<VecValue>;
}
type FuncObj = Arc<Box<dyn Func + Send + Sync>>;

pub struct ExecutorContext {
  lua: Lua,
  func: HashMap<String, FuncObj>,
}

impl ExecutorContext {
  pub fn try_new() -> Result<Self> {
    let lua = Lua::new();
    let ctx = Self {
      lua,
      func: HashMap::new(),
    };
    ctx.init()?;
    Ok(ctx)
  }

  pub fn try_new_with_func(func: HashMap<String, FuncObj>) -> Result<Self> {
    let lua = Lua::new();
    let ctx = Self { lua, func };
    ctx.init()?;
    Ok(ctx)
  }

  fn init(&self) -> Result<()> {
    self.lua.load_std_libs(StdLib::ALL_SAFE)?;
    let f_table = self.lua.create_table()?;
    for (f_name, f) in self.func.iter() {
      let f = f.clone();
      f_table.set(
        f_name.clone(),
        self.lua.create_function(move |lua, args: VecValue| f.call(lua, args))?,
      )?;
    }
    libs::register(&self.lua, &f_table)?;
    self.lua.globals().set("mxa", f_table)?;
    Ok(())
  }

  pub async fn exec_async(&self, script: &str) -> Result<()> {
    self.lua.load(script).exec_async().await?;
    Ok(())
  }

  pub async fn eval_async(&self, script: &str) -> Result<ValueType> {
    let result = self.lua.load(script).eval_async().await?;
    let vt = ValueType::try_from_lua(result)?;
    Ok(vt)
  }
}
