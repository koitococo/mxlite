use anyhow::Result;
use mlua::{FromLuaMulti, IntoLuaMulti, Lua, MultiValue, StdLib};
mod libs;
mod value_type;

pub use self::value_type::ValueType;
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

impl From<Vec<ValueType>> for VecValue {
  fn from(vec: Vec<ValueType>) -> Self { VecValue(vec) }
}

impl From<VecValue> for Vec<ValueType> {
  fn from(vec_value: VecValue) -> Self { vec_value.0 }
}

impl<VT> FromIterator<VT> for VecValue
where VT: Into<ValueType>
{
  fn from_iter<I: IntoIterator<Item = VT>>(iter: I) -> Self { VecValue(iter.into_iter().map(Into::into).collect()) }
}

pub trait Invokable {
  fn call(&self, lua: &Lua, args: VecValue) -> mlua::Result<VecValue>;
}
pub type FuncObj = Box<dyn Invokable + Send + Sync>;

pub struct ExecutorContext {
  lua: Lua,
}

impl ExecutorContext {
  pub fn try_new_with_fn<T: IntoIterator<Item = (String, FuncObj)>>(fn_map: Option<T>) -> Result<Self> {
    let lua = Lua::new();
    lua.load_std_libs(StdLib::ALL_SAFE)?;

    let f_table = lua.create_table()?;
    f_table.set("version", crate::VERSION)?;
    libs::register(&lua, &f_table)?;

    if let Some(fn_map) = fn_map {
      for (name, func) in fn_map {
        let Ok(f) = lua.create_function(move |lua, args: VecValue| func.call(lua, args)) else {
          anyhow::bail!("Failed to create function for: {name}");
        };
        f_table.set(name, f)?;
      }
    }
    lua.globals().set("mx", f_table)?;

    Ok(ExecutorContext { lua })
  }

  pub fn try_new() -> Result<Self> { Self::try_new_with_fn::<Vec<(String, FuncObj)>>(None) }

  pub async fn exec_async(&self, script: &str) -> Result<()> {
    self.lua.load(script).exec_async().await?;
    Ok(())
  }

  pub async fn eval_async(&self, script: &str) -> Result<ValueType> {
    let result = self.lua.load(script).eval_async().await?;
    let vt = ValueType::try_from_lua(result)?;
    Ok(vt)
  }

  pub async fn invoke_async(&self, script: &str, args: VecValue) -> Result<ValueType> {
    let func = self.lua.load(script).into_function()?;
    let result: ValueType = func.call_async(args).await?;
    Ok(result)
  }
}
