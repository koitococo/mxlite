use std::{collections::HashMap, marker::PhantomData};

use anyhow::Result;
use mlua::{FromLua, IntoLua, Lua, Table, Value};
use serde::{
  de::{Deserialize, MapAccess, SeqAccess, Visitor},
  ser::{Serialize, SerializeMap, SerializeSeq},
};

#[derive(Debug, Clone)]
pub enum ValueType {
  None,
  String(String),
  Integer(i64),
  Float(f64),
  Boolean(bool),
  Table(HashMap<String, ValueType>),
  Array(Vec<ValueType>),
}

impl Serialize for ValueType {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where S: serde::Serializer {
    match self {
      ValueType::None => serializer.serialize_none(),
      ValueType::String(v) => serializer.serialize_str(v),
      ValueType::Integer(v) => serializer.serialize_i64(*v),
      ValueType::Float(v) => serializer.serialize_f64(*v),
      ValueType::Boolean(v) => serializer.serialize_bool(*v),
      ValueType::Table(v) => {
        let mut map = serializer.serialize_map(Some(v.len()))?;
        for (k, v) in v {
          map.serialize_entry(k, v)?;
        }
        map.end()
      }
      ValueType::Array(v) => {
        let mut seq = serializer.serialize_seq(Some(v.len()))?;
        for v in v {
          seq.serialize_element(v)?;
        }
        seq.end()
      }
    }
  }
}

struct ValueTypeVisitor {
  marker: PhantomData<fn() -> ValueType>,
}

impl Default for ValueTypeVisitor {
  fn default() -> Self { ValueTypeVisitor { marker: PhantomData } }
}

impl<'de> Visitor<'de> for ValueTypeVisitor {
  type Value = ValueType;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result { formatter.write_str("a ValueType") }

  fn visit_bool<E>(self, v: bool) -> std::result::Result<Self::Value, E>
  where E: serde::de::Error {
    Ok(ValueType::Boolean(v))
  }

  fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
  where E: serde::de::Error {
    Ok(ValueType::Integer(v))
  }

  fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
  where E: serde::de::Error {
    self.visit_i64(v as i64)
  }

  fn visit_f64<E>(self, v: f64) -> std::result::Result<Self::Value, E>
  where E: serde::de::Error {
    Ok(ValueType::Float(v))
  }

  fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
  where E: serde::de::Error {
    Ok(ValueType::String(v.to_string()))
  }

  fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
  where E: serde::de::Error {
    let Ok(s) = std::str::from_utf8(v) else {
      return Err(E::custom("Invalid UTF-8"));
    };
    Ok(ValueType::String(s.to_string()))
  }

  fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
  where E: serde::de::Error {
    Ok(ValueType::None)
  }

  fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
  where E: serde::de::Error {
    self.visit_none()
  }

  fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
  where A: SeqAccess<'de> {
    let mut array = Vec::new();
    while let Some(v) = seq.next_element()? {
      array.push(v);
    }
    Ok(ValueType::Array(array))
  }

  fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
  where A: MapAccess<'de> {
    let mut table = HashMap::new();
    while let Some((k, v)) = map.next_entry::<String, ValueType>()? {
      table.insert(k, v);
    }
    Ok(ValueType::Table(table))
  }
}

impl<'de> Deserialize<'de> for ValueType {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where D: serde::Deserializer<'de> {
    deserializer.deserialize_any(ValueTypeVisitor::default())
  }
}

fn hashmap_to_table(lua: &Lua, map: HashMap<String, ValueType>) -> mlua::Result<Table> {
  let table = lua.create_table()?;
  for (k, v) in map {
    match v {
      ValueType::None => table.set(k, Value::Nil)?,
      ValueType::String(v) => table.set(k, lua.create_string(&v)?)?,
      ValueType::Boolean(v) => table.set(k, v)?,
      ValueType::Integer(v) => table.set(k, v)?,
      ValueType::Float(v) => table.set(k, v)?,
      ValueType::Table(v) => {
        table.set(k, hashmap_to_table(lua, v)?)?;
      }
      ValueType::Array(v) => {
        vec_to_table(lua, v)?;
      }
    }
  }
  Ok(table)
}

fn vec_to_table(lua: &Lua, vec: Vec<ValueType>) -> mlua::Result<Table> {
  let table = lua.create_table()?;
  for (i, v) in vec.into_iter().enumerate() {
    match v {
      ValueType::None => table.set(i + 1, Value::Nil)?,
      ValueType::String(v) => table.set(i + 1, lua.create_string(&v)?)?,
      ValueType::Boolean(v) => table.set(i + 1, v)?,
      ValueType::Integer(v) => table.set(i + 1, v)?,
      ValueType::Float(v) => table.set(i + 1, v)?,
      ValueType::Table(v) => {
        table.set(i + 1, hashmap_to_table(lua, v)?)?;
      }
      ValueType::Array(v) => {
        table.set(i + 1, vec_to_table(lua, v)?)?;
      }
    }
  }
  Ok(table)
}

impl ValueType {
  pub(super) fn try_into_lua(self, lua: &Lua) -> mlua::Result<Value> {
    let r = match self {
      ValueType::None => Value::Nil,
      ValueType::String(v) => Value::String(lua.create_string(v)?),
      ValueType::Integer(v) => Value::Integer(v),
      ValueType::Float(v) => Value::Number(v),
      ValueType::Boolean(v) => Value::Boolean(v),
      ValueType::Table(v) => Value::Table(hashmap_to_table(lua, v)?),
      ValueType::Array(v) => Value::Table(vec_to_table(lua, v)?),
    };
    Ok(r)
  }

  pub(super) fn try_from_lua(value: Value) -> mlua::Result<Self> {
    let r = match value {
      Value::Nil => ValueType::None,
      Value::String(v) => ValueType::String(v.to_string_lossy()),
      Value::Integer(v) => ValueType::Integer(v),
      Value::Number(v) => ValueType::Float(v),
      Value::Boolean(v) => ValueType::Boolean(v),
      Value::Table(t) => {
        let mut map = HashMap::new();
        for pair in t.pairs::<String, Value>() {
          let (k, v) = pair?;
          map.insert(k, Self::try_from_lua(v)?);
        }
        ValueType::Table(map)
      }
      _ => return Err(mlua::Error::RuntimeError("Unsupported value type".to_string())),
    };
    Ok(r)
  }

  fn try_table_val(&self, key: &str) -> Option<ValueType> {
    if let ValueType::Table(t) = self {
      if let Some(v) = t.get(key) {
        return Some(v.clone());
      }
    }
    None
  }

  pub(super) fn try_table_val_typed<T>(&self, key: &str) -> Option<T>
  where Self: Into<Option<T>> {
    if let Some(v) = self.try_table_val(key) {
      return v.into();
    }
    None
  }
}

impl FromLua for ValueType {
  fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> { Self::try_from_lua(value) }
}

impl IntoLua for ValueType {
  fn into_lua(self, lua: &Lua) -> mlua::Result<Value> { self.try_into_lua(lua) }
}

impl From<ValueType> for Option<String> {
  fn from(val: ValueType) -> Self { if let ValueType::String(v) = val { Some(v) } else { None } }
}

impl From<ValueType> for Option<i64> {
  fn from(val: ValueType) -> Self {
    if let ValueType::Integer(v) = val {
      Some(v)
    } else {
      None
    }
  }
}

impl From<ValueType> for Option<f64> {
  fn from(val: ValueType) -> Self { if let ValueType::Float(v) = val { Some(v) } else { None } }
}

impl From<ValueType> for Option<bool> {
  fn from(val: ValueType) -> Self {
    if let ValueType::Boolean(v) = val {
      Some(v)
    } else {
      None
    }
  }
}

impl From<ValueType> for Option<HashMap<String, ValueType>> {
  fn from(val: ValueType) -> Self { if let ValueType::Table(v) = val { Some(v) } else { None } }
}

impl From<ValueType> for Option<Vec<ValueType>> {
  fn from(val: ValueType) -> Self { if let ValueType::Array(v) = val { Some(v) } else { None } }
}
