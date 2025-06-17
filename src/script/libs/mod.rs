mod fetch;
mod json;
mod subprocess;
mod yaml;

pub(super) fn register(lua: &mlua::Lua, f_table: &mlua::Table) -> mlua::Result<()> {
  fetch::register(lua, f_table)?;
  json::register(lua, f_table)?;
  subprocess::register(lua, f_table)?;
  yaml::register(lua, f_table)?;
  Ok(())
}
