use mlua::{Lua, Value};

use crate::script::value_type::ValueType;

fn lua_json_encode(_: &Lua, vt: Value) -> mlua::Result<String> {
  let v = ValueType::try_from_lua(vt)?;
  let json = match serde_json::to_string(&v) {
    Ok(v) => v,
    Err(e) => {
      return Err(mlua::Error::RuntimeError(format!("Failed to serialize to JSON: {e}")));
    }
  };
  Ok(json)
}

fn lua_json_decode(lua: &Lua, s: String) -> mlua::Result<Value> {
  let v: ValueType = match serde_json::from_str(&s) {
    Ok(v) => v,
    Err(e) => {
      return Err(mlua::Error::RuntimeError(format!("Failed to parse JSON: {e}")));
    }
  };
  let vt = v.try_into_lua(lua)?;
  Ok(vt)
}

#[inline]
fn mod_json(lua: &Lua) -> mlua::Result<mlua::Table> {
  let json = lua.create_table()?;
  json.set("encode", lua.create_function(lua_json_encode)?)?;
  json.set("decode", lua.create_function(lua_json_decode)?)?;
  Ok(json)
}

pub(super) fn register(lua: &Lua, f_table: &mlua::Table) -> mlua::Result<()> {
  let json = mod_json(lua)?;
  f_table.set("json", json)?;
  Ok(())
}
