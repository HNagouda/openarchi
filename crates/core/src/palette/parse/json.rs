//! JSON palettes: an array of codes, a map of names to codes, or an array of
//! objects with a "hex" and a "name".

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use serde_json::Value;

use super::{excerpt, hex_code, invalid, swatch};
use crate::palette::Swatch;
use crate::Result;

pub fn read(text: &str) -> Result<Vec<Swatch>> {
    if text.starts_with('{') {
        let named: OrderedObject = serde_json::from_str(text)
            .map_err(|error| invalid(format!("the JSON could not be read: {error}")))?;
        return named
            .0
            .into_iter()
            .map(|(name, value)| json_entry(&value, Some(name)))
            .collect();
    }

    let entries: Vec<Value> = serde_json::from_str(text)
        .map_err(|error| invalid(format!("the JSON could not be read: {error}")))?;
    entries
        .iter()
        .map(|entry| match entry {
            Value::Object(fields) => {
                let name = fields
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let code = ["hex", "color", "colour", "value"]
                    .iter()
                    .find_map(|key| fields.get(*key));
                match code {
                    Some(code) => json_entry(code, name),
                    None => Err(invalid(format!(
                        "a JSON entry has no \"hex\": {}",
                        excerpt(&entry.to_string())
                    ))),
                }
            }
            other => json_entry(other, None),
        })
        .collect()
}

fn json_entry(value: &Value, name: Option<String>) -> Result<Swatch> {
    let text = value.as_str().unwrap_or_default();
    match hex_code(text) {
        Some(hex) => Ok(swatch(hex, name)),
        None => Err(invalid(format!(
            "{} is not a hex colour like \"f79f79\"",
            excerpt(&value.to_string())
        ))),
    }
}

/// A JSON object read as pairs in the order written. serde_json's own map
/// sorts its keys, which would shuffle a named palette.
struct OrderedObject(Vec<(String, Value)>);

impl<'de> Deserialize<'de> for OrderedObject {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        deserializer.deserialize_map(OrderedVisitor)
    }
}

struct OrderedVisitor;

impl<'de> Visitor<'de> for OrderedVisitor {
    type Value = OrderedObject;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an object of names and hex codes")
    }

    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> std::result::Result<OrderedObject, A::Error> {
        let mut pairs = Vec::new();
        while let Some(pair) = map.next_entry::<String, Value>()? {
            pairs.push(pair);
        }
        Ok(OrderedObject(pairs))
    }
}
