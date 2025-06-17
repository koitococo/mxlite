use mlua::{Lua, Value};

use crate::script::value_type::ValueType;

fn lua_yaml_encode(_: &Lua, vt: Value) -> mlua::Result<String> {
  let v = ValueType::try_from_lua(vt)?;
  let yaml = match serde_yml::to_string(&v) {
    Ok(v) => v,
    Err(e) => {
      return Err(mlua::Error::RuntimeError(format!("Failed to serialize to YAML: {e}")));
    }
  };
  Ok(yaml)
}

fn lua_yaml_decode(lua: &Lua, s: String) -> mlua::Result<Value> {
  let v: ValueType = match serde_yml::from_str(&s) {
    Ok(v) => v,
    Err(e) => {
      return Err(mlua::Error::RuntimeError(format!("Failed to parse YAML: {e}")));
    }
  };
  let vt = v.try_into_lua(lua)?;
  Ok(vt)
}

#[inline]
fn mod_yaml(lua: &Lua) -> mlua::Result<mlua::Table> {
  let yaml = lua.create_table()?;
  yaml.set("encode", lua.create_function(lua_yaml_encode)?)?;
  yaml.set("decode", lua.create_function(lua_yaml_decode)?)?;
  Ok(yaml)
}

pub(super) fn register(lua: &Lua, f_table: &mlua::Table) -> mlua::Result<()> {
  let yaml = mod_yaml(lua)?;
  f_table.set("yaml", yaml)?;
  Ok(())
}
