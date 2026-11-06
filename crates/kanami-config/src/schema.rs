use kanami_core::Key;

use crate::expr;

/// The JSON Schema of the config file, for editor completion and validation. Key names are generated from code so they always match.
pub fn schema() -> String {
    let keys: Vec<String> = Key::ALL
        .iter()
        .map(|k| format!("\"{}\"", k.name()))
        .collect();
    let modifiers: Vec<&str> = expr::modifier_names().collect();
    include_str!("schema.template.json")
        .replace("__KEYS__", &format!("[{}]", keys.join(", ")))
        .replace("__MODIFIERS__", &modifiers.join(", "))
}
