//! The raw structure of the config file; fields carry source positions for locating errors.

use std::collections::BTreeMap;

use serde::Deserialize;
use toml::Spanned;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Raw {
    #[serde(default)]
    pub settings: RawSettings,
    #[serde(default)]
    pub remap: BTreeMap<Spanned<String>, Spanned<String>>,
    #[serde(default)]
    pub rule: Vec<Spanned<RawRule>>,
    #[serde(default)]
    pub group: Vec<Spanned<RawGroup>>,
    #[serde(default)]
    pub layer: BTreeMap<Spanned<String>, Spanned<RawLayer>>,
    #[serde(default)]
    pub combo: Vec<Spanned<RawCombo>>,
    #[serde(default)]
    pub test: Vec<Spanned<RawTest>>,
    #[serde(default)]
    pub script: RawScript,
    #[serde(default)]
    pub device: BTreeMap<Spanned<String>, Spanned<RawDevice>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawDevice {
    pub vendor_id: Option<u32>,
    pub product_id: Option<u32>,
    pub product: Option<String>,
    /// Whether to also use the rules and remaps that apply to all keyboards; defaults to yes.
    pub inherit: Option<bool>,
    #[serde(default)]
    pub remap: BTreeMap<Spanned<String>, Spanned<String>>,
    #[serde(default)]
    pub rule: Vec<Spanned<RawRule>>,
    #[serde(default)]
    pub group: Vec<Spanned<RawGroup>>,
}

/// `[[group]]`: a set of rules sharing a condition and an on/off switch.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawGroup {
    /// The group name shown in the UI; the engine does not use it.
    #[allow(dead_code)]
    pub name: Option<String>,
    pub enabled: Option<bool>,
    #[serde(default)]
    pub when: RawWhen,
    #[serde(default)]
    pub rule: Vec<Spanned<RawRule>>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawScript {
    #[serde(default)]
    pub files: Vec<Spanned<String>>,
    pub source: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawSettings {
    pub tap_timeout: Option<u32>,
    pub combo_timeout: Option<u32>,
    pub typing_streak: Option<u32>,
    pub layer_hud: Option<bool>,
    pub language: Option<Spanned<String>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRule {
    pub from: Spanned<String>,
    pub to: Option<Spanned<OneOrMany>>,
    pub tap: Option<Spanned<OneOrMany>>,
    pub hold: Option<Spanned<String>>,
    pub strategy: Option<Spanned<String>>,
    pub run: Option<Spanned<String>>,
    pub call: Option<Spanned<String>>,
    pub enabled: Option<bool>,
    #[serde(default)]
    pub when: RawWhen,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawWhen {
    #[serde(default)]
    pub app: Vec<String>,
    #[serde(default)]
    pub app_not: Vec<String>,
    #[serde(default)]
    pub input_source: Vec<String>,
    #[serde(default)]
    pub input_source_not: Vec<String>,
    #[serde(default)]
    pub var: BTreeMap<String, Spanned<toml::Value>>,
    #[serde(default)]
    pub device: Vec<String>,
    #[serde(default)]
    pub device_not: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawLayer {
    pub hold: Option<Spanned<String>>,
    pub toggle: Option<Spanned<String>>,
    pub tap: Option<Spanned<OneOrMany>>,
    pub strategy: Option<Spanned<String>>,
    #[serde(default)]
    pub map: BTreeMap<Spanned<String>, Spanned<OneOrMany>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawCombo {
    pub keys: Spanned<Vec<String>>,
    pub to: Option<Spanned<OneOrMany>>,
    pub run: Option<Spanned<String>>,
    pub call: Option<Spanned<String>>,
    pub timeout: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawTest {
    pub name: Option<String>,
    pub app: Option<String>,
    pub input_source: Option<String>,
    pub device: Option<Spanned<String>>,
    pub input: Spanned<String>,
    pub expect: Spanned<String>,
}
