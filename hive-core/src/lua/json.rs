use super::error::check_arg;
use mlua::Value::Nil;
use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table};

pub fn create_preload_json(lua: &Lua) -> mlua::Result<Function> {
  lua.create_function(|lua, ()| {
    let json_table = lua.create_table()?;
    json_table.raw_set("parse", create_fn_json_parse(lua)?)?;
    json_table.raw_set("stringify", create_fn_json_stringify(lua)?)?;
    json_table.raw_set("array", create_fn_json_array(lua)?)?;
    json_table.raw_set("undo_array", create_fn_json_undo_array(lua)?)?;
    json_table.raw_set("array_metatable", lua.array_metatable())?;
    Ok(json_table)
  })
}

fn create_fn_json_parse(lua: &Lua) -> mlua::Result<Function> {
  lua.create_function(|lua, args: MultiValue| {
    let string: mlua::String = check_arg(lua, &args, 1, "string", 0)?;
    let result = serde_json::from_slice::<serde_json::Value>(string.as_bytes());
    match result {
      Ok(result) => lua.pack_multi(lua.to_value(&result)?),
      Err(error) => lua.pack_multi((Nil, error.to_string())),
    }
  })
}

fn create_fn_json_stringify(lua: &Lua) -> mlua::Result<Function> {
  lua.create_function(|lua, args: MultiValue| {
    let value: mlua::Value = check_arg(lua, &args, 1, "value", 0)?;
    let pretty = check_arg::<Option<bool>>(lua, &args, 1, "bool", 0)?.unwrap_or(false);
    let result = if pretty {
      serde_json::to_string_pretty(&value)
    } else {
      serde_json::to_string(&value)
    };
    match result {
      Ok(s) => lua.pack_multi(s),
      Err(error) => lua.pack_multi((Nil, error.to_string())),
    }
  })
}

fn create_fn_json_array(lua: &Lua) -> mlua::Result<Function> {
  lua.create_function(|lua, args: MultiValue| {
    let table: Table = check_arg(lua, &args, 1, "table", 0)?;
    table.set_metatable(Some(lua.array_metatable()));
    Ok(table)
  })
}

fn create_fn_json_undo_array(lua: &Lua) -> mlua::Result<Function> {
  lua.create_function(|lua, args: MultiValue| {
    let table: Table = check_arg(lua, &args, 1, "table", 0)?;
    if table
      .get_metatable()
      .map(|x| x == lua.array_metatable())
      .unwrap_or(false)
    {
      table.set_metatable(None);
    }
    Ok(table)
  })
}
