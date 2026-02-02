/// Transformer utilities
use bamboo_core::types::Content;
use serde_json::Value;

/// Convert content to a simple string representation
pub fn content_to_string(content: &Content) -> String {
    match content {
        Content::Text { text } => text.clone(),
        Content::Parts { parts } => {
            parts.iter()
                .filter_map(|p| match p {
                    bamboo_core::types::ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        }
    }
}

/// Create a JSON object from key-value pairs
pub fn json_object(pairs: Vec<(&str, Value)>) -> Value {
    let mut map = serde_json::Map::new();
    for (key, value) in pairs {
        map.insert(key.to_string(), value);
    }
    Value::Object(map)
}

/// Safe get from JSON value
pub fn safe_get<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;
    
    for part in parts {
        current = current.get(part)?;
    }
    
    Some(current)
}

/// Safe get string from JSON
pub fn safe_get_str<'a>(value: &'a Value, path: &str) -> Option<&'a str> {
    safe_get(value, path)?.as_str()
}
