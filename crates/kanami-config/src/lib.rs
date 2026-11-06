//! The config layer: parses TOML into [`Rules`], reporting errors with line and column numbers.
//! It runs only at startup and reload, never on the hot path. See `docs/config-spec.md` for the syntax.

mod expr;
mod raw;
mod schema;
mod testing;

use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use kanami_core::{
    Action, Activation, Chord, Combo, Condition, DeviceSpec, Engine, Key, Layer, Location, Remap,
    Rule, RuleKind, Rules, Settings, Strategy, TapHold,
};
use kanami_i18n::{Lang, tr};
use toml::Spanned;

pub use expr::{chord as parse_chord, modifier_names};
pub use schema::schema;
pub use testing::{Mismatch, TestCase};

use raw::{OneOrMany, Raw, RawCombo, RawGroup, RawLayer, RawRule, RawWhen};

/// A parsed config.
#[derive(Debug)]
pub struct Config {
    pub rules: Rules,
    pub tests: Vec<TestCase>,
    /// The scripts in `[script]`, in written order: `files` first, then `source`.
    pub scripts: Vec<Script>,
    /// Files other than the config file itself that need watching for changes (script files).
    pub watched: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct Script {
    /// The name shown in errors (a file path or "inline script").
    pub name: String,
    pub code: String,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(PathBuf, std::io::Error),
    Toml(toml::de::Error),
    Invalid {
        line: usize,
        column: usize,
        message: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(path, err) => f.write_str(&tr!(
                "Failed to read config {path}: {error}",
                path = path.display(),
                error = err
            )),
            ConfigError::Toml(err) => write!(f, "{err}"),
            ConfigError::Invalid {
                line,
                column,
                message,
            } => write!(f, "{line}:{column}: {message}"),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let src = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(path.to_owned(), e))?;
    parse_in(&src, path.parent())
}

/// Parses config text; relative paths in `[script] files` are relative to the current directory.
pub fn parse(src: &str) -> Result<Config, ConfigError> {
    parse_in(src, None)
}

/// Parses config text; relative paths in `[script] files` are relative to `base`.
pub fn parse_in(src: &str, base: Option<&Path>) -> Result<Config, ConfigError> {
    let raw: Raw = toml::from_str(src).map_err(ConfigError::Toml)?;
    Builder {
        src,
        base,
        spans: Spans::default(),
    }
    .build(raw)
}

/// Where each rule sits in the source file, used to map engine validation errors back to it.
#[derive(Default)]
struct Spans {
    remap: Vec<Range<usize>>,
    rule: Vec<Range<usize>>,
    layer: Vec<Range<usize>>,
    combo: Vec<Range<usize>>,
}

struct Builder<'a> {
    src: &'a str,
    base: Option<&'a Path>,
    spans: Spans,
}

impl Builder<'_> {
    fn build(mut self, raw: Raw) -> Result<Config, ConfigError> {
        let defaults = Settings::default();
        let settings = Settings {
            tap_timeout: raw.settings.tap_timeout.unwrap_or(defaults.tap_timeout),
            combo_timeout: raw.settings.combo_timeout.unwrap_or(defaults.combo_timeout),
            typing_streak: raw.settings.typing_streak.unwrap_or(defaults.typing_streak),
            layer_hud: raw.settings.layer_hud.unwrap_or(defaults.layer_hud),
            language: match &raw.settings.language {
                Some(tag) => Some(self.at(tag, language)?),
                None => defaults.language,
            },
        };

        let mut devices = Vec::new();
        for (name, device) in &raw.device {
            let d = device.get_ref();
            if d.vendor_id.is_none() && d.product_id.is_none() && d.product.is_none() {
                let msg = tr!(
                    "Device {name} needs at least one of vendor_id, product_id and product; run `kanami devices` to see them",
                    name = name.get_ref()
                );
                return Err(self.error(name.span(), msg));
            }
            devices.push(DeviceSpec {
                name: name.get_ref().clone(),
                vendor_id: d.vendor_id,
                product_id: d.product_id,
                product: d.product.clone(),
                exclusive: d.inherit == Some(false),
            });
        }

        let mut remaps = Vec::new();
        let device_remaps = raw
            .device
            .values()
            .enumerate()
            .map(|(i, d)| (Some(i), &d.get_ref().remap));
        for (device, table) in std::iter::once((None, &raw.remap)).chain(device_remaps) {
            for (from, to) in table {
                remaps.push(Remap {
                    from: self.at(from, expr::key)?,
                    to: self.at(to, expr::remap_target)?,
                    device,
                });
                self.spans.remap.push(from.span());
            }
        }

        // A keyboard's own rules come before the rules for all keyboards; a keyboard that uses only its own rules skips the general ones
        let mut rules = Vec::new();
        for (name, device) in &raw.device {
            let d = device.get_ref();
            let scope = Condition {
                device: vec![name.get_ref().clone()],
                ..Condition::default()
            };
            self.rule_set(&d.rule, &d.group, &scope, Some(name.get_ref()), &mut rules)?;
        }
        let scope = Condition {
            device_not: devices
                .iter()
                .filter(|d| d.exclusive)
                .map(|d| d.name.clone())
                .collect(),
            ..Condition::default()
        };
        self.rule_set(&raw.rule, &raw.group, &scope, None, &mut rules)?;

        let mut layers = Vec::new();
        for (name, layer) in &raw.layer {
            layers.push(self.layer(name, layer)?);
            self.spans.layer.push(name.span());
        }

        let mut combos = Vec::new();
        for combo in &raw.combo {
            combos.push(self.combo(combo)?);
            self.spans.combo.push(combo.span());
        }

        let rules = Rules {
            settings,
            remaps,
            rules,
            layers,
            combos,
            devices,
        };
        if let Err(e) = Engine::new(&rules) {
            let spans = match e.at {
                Location::Remap(i) => self.spans.remap.get(i),
                Location::Rule(i) => self.spans.rule.get(i),
                Location::Layer(i) => self.spans.layer.get(i),
                Location::Combo(i) => self.spans.combo.get(i),
            };
            let span = spans.cloned().unwrap_or(0..0);
            return Err(self.error(span, e.to_string()));
        }

        let mut tests = Vec::new();
        for (i, test) in raw.test.iter().enumerate() {
            let t = test.get_ref();
            let line = self.line_col(test.span().start).0;
            let name = t
                .name
                .clone()
                .unwrap_or_else(|| tr!("Test {number}", number = i + 1));
            if let Some(device) = &t.device
                && !rules.devices.iter().any(|d| d.name == *device.get_ref())
            {
                let msg = tr!("There is no device named {name}", name = device.get_ref());
                return Err(self.error(device.span(), msg));
            }
            let case = TestCase::new(
                name,
                line,
                t.app.clone(),
                t.input_source.clone(),
                t.device.as_ref().map(|d| d.get_ref().clone()),
                t.input.get_ref(),
                t.expect.get_ref(),
            )
            .map_err(|(field, msg)| {
                let span = match field {
                    testing::Field::Input => t.input.span(),
                    testing::Field::Expect => t.expect.span(),
                };
                self.error(span, msg)
            })?;
            tests.push(case);
        }

        let mut scripts = Vec::new();
        let mut watched = Vec::new();
        for file in &raw.script.files {
            let path = self.resolve(file.get_ref());
            let code = std::fs::read_to_string(&path).map_err(|e| {
                self.error(
                    file.span(),
                    tr!(
                        "Failed to read script {path}: {error}",
                        path = path.display(),
                        error = e
                    ),
                )
            })?;
            scripts.push(Script {
                name: path.display().to_string(),
                code,
            });
            watched.push(path);
        }
        if let Some(code) = &raw.script.source {
            scripts.push(Script {
                name: tr!("inline script"),
                code: code.clone(),
            });
        }

        Ok(Config {
            rules,
            tests,
            scripts,
            watched,
        })
    }

    /// Script paths: `~/` is relative to HOME, and other relative paths are relative to the directory of the config file.
    fn resolve(&self, path: &str) -> PathBuf {
        if let Some(rest) = path.strip_prefix("~/")
            && let Some(home) = std::env::var_os("HOME")
        {
            return PathBuf::from(home).join(rest);
        }
        let path = Path::new(path);
        match self.base {
            Some(base) if path.is_relative() => base.join(path),
            _ => path.to_owned(),
        }
    }

    fn condition(&self, when: &RawWhen) -> Result<Condition, ConfigError> {
        let mut vars = Vec::new();
        for (name, value) in &when.var {
            let text = match value.get_ref() {
                toml::Value::String(s) => s.clone(),
                toml::Value::Boolean(b) => b.to_string(),
                toml::Value::Integer(i) => i.to_string(),
                _ => {
                    return Err(self.error(
                        value.span(),
                        tr!(
                            "The value of variable {name} must be a string, boolean or integer",
                            name = name
                        ),
                    ));
                }
            };
            vars.push((name.clone(), text));
        }
        Ok(Condition {
            app: when.app.clone(),
            app_not: when.app_not.clone(),
            input_source: when.input_source.clone(),
            input_source_not: when.input_source_not.clone(),
            vars,
            device: when.device.clone(),
            device_not: when.device_not.clone(),
        })
    }

    /// A set of rules and rule groups, sorted by their position in the file and added to `out`. `scope` is the condition of the outer level (a keyboard);
    /// `device` is the name of the enclosing `[device.*]`, in which case rules and groups cannot also write `when.device`.
    fn rule_set(
        &mut self,
        rules: &[Spanned<RawRule>],
        groups: &[Spanned<RawGroup>],
        scope: &Condition,
        device: Option<&str>,
        out: &mut Vec<Rule>,
    ) -> Result<(), ConfigError> {
        let mut items = Vec::new();
        for rule in rules {
            items.push((rule, scope.clone(), true));
        }
        for group in groups {
            let g = group.get_ref();
            self.check_device_free(&g.when, group, device)?;
            let mut scope = scope.clone();
            let when = self.condition(&g.when)?;
            scope.app = when.app;
            scope.app_not = when.app_not;
            scope.input_source = when.input_source;
            scope.input_source_not = when.input_source_not;
            scope.vars = when.vars;
            if device.is_none() {
                scope.device = when.device;
            }
            scope.device_not.extend(when.device_not);
            for rule in &g.rule {
                items.push((rule, scope.clone(), g.enabled != Some(false)));
            }
        }
        items.sort_by_key(|(rule, _, _)| rule.span().start);
        for (rule, scope, group_enabled) in items {
            self.check_device_free(&rule.get_ref().when, rule, device)?;
            // Disabled rules are checked for syntax too, but are not given to the engine
            let mut built = self.rule(rule)?;
            if group_enabled && rule.get_ref().enabled != Some(false) {
                built.scope = scope;
                out.push(built);
                self.spans.rule.push(rule.span());
            }
        }
        Ok(())
    }

    fn check_device_free<T>(
        &self,
        when: &RawWhen,
        at: &Spanned<T>,
        device: Option<&str>,
    ) -> Result<(), ConfigError> {
        match device {
            Some(name) if !when.device.is_empty() || !when.device_not.is_empty() => {
                let msg = tr!(
                    "This is already inside [device.{name}], so when.device cannot be used here",
                    name = name
                );
                Err(self.error(at.span(), msg))
            }
            _ => Ok(()),
        }
    }

    fn rule(&self, spanned: &Spanned<RawRule>) -> Result<Rule, ConfigError> {
        let r = spanned.get_ref();
        let from = self.at(&r.from, expr::trigger)?;
        let only = |msg: String| Err(self.error(spanned.span(), msg));
        if r.strategy.is_some() && r.hold.is_none() {
            return only(tr!("strategy can only be used with tap/hold"));
        }
        let tap_hold = r.tap.is_some() || r.hold.is_some();
        let kinds = [r.to.is_some(), r.run.is_some(), r.call.is_some(), tap_hold];
        match kinds.iter().filter(|&&k| k).count() {
            0 => return only(tr!("The rule is missing to, run, call or tap/hold")),
            1 => {}
            _ => {
                return only(tr!(
                    "A rule can have only one of to, run, call and tap/hold"
                ));
            }
        }
        let kind = if let Some(to) = &r.to {
            RuleKind::Action(self.at(to, expr::action)?)
        } else if let Some(cmd) = &r.run {
            RuleKind::Action(Action::Run(cmd.get_ref().clone()))
        } else if let Some(function) = &r.call {
            RuleKind::Action(Action::Call(function.get_ref().clone()))
        } else {
            let Some(hold) = &r.hold else {
                return only(tr!("tap requires hold as well"));
            };
            RuleKind::TapHold(TapHold {
                tap: match &r.tap {
                    Some(tap) => self.at(tap, expr::action)?,
                    None => default_tap(from.key),
                },
                hold: self.at(hold, expr::chord)?,
                strategy: self.strategy(r.strategy.as_ref())?,
            })
        };
        Ok(Rule {
            from,
            kind,
            when: self.condition(&r.when)?,
            scope: Condition::default(),
        })
    }

    fn layer(
        &self,
        name: &Spanned<String>,
        spanned: &Spanned<RawLayer>,
    ) -> Result<Layer, ConfigError> {
        let l = spanned.get_ref();
        let fail = |msg: String| Err(self.error(name.span(), msg));
        let activation = match (&l.hold, &l.toggle) {
            (Some(hold), None) => Activation::Hold {
                key: self.at(hold, expr::key)?,
                tap: l
                    .tap
                    .as_ref()
                    .map(|t| self.at(t, expr::action))
                    .transpose()?,
                strategy: self.strategy(l.strategy.as_ref())?,
            },
            (None, Some(toggle)) => {
                if l.tap.is_some() || l.strategy.is_some() {
                    return fail(tr!(
                        "The toggle layer {name} does not support tap or strategy",
                        name = name.get_ref()
                    ));
                }
                Activation::Toggle(self.at(toggle, expr::trigger)?)
            }
            _ => {
                return fail(tr!(
                    "Layer {name} needs either hold or toggle",
                    name = name.get_ref()
                ));
            }
        };
        let mut map = Vec::new();
        for (key, action) in &l.map {
            map.push((self.at(key, expr::key)?, self.at(action, expr::action)?));
        }
        Ok(Layer {
            name: name.get_ref().clone(),
            activation,
            map,
        })
    }

    fn combo(&self, spanned: &Spanned<RawCombo>) -> Result<Combo, ConfigError> {
        let c = spanned.get_ref();
        let keys = c
            .keys
            .get_ref()
            .iter()
            .map(|k| expr::key(k))
            .collect::<Result<Vec<Key>, _>>()
            .map_err(|e| self.error(c.keys.span(), e))?;
        let action = match (&c.to, &c.run, &c.call) {
            (Some(to), None, None) => self.at(to, expr::action)?,
            (None, Some(cmd), None) => Action::Run(cmd.get_ref().clone()),
            (None, None, Some(function)) => Action::Call(function.get_ref().clone()),
            _ => {
                let msg = tr!("A combo needs one of to, run and call");
                return Err(self.error(spanned.span(), msg));
            }
        };
        Ok(Combo {
            keys,
            action,
            timeout: c.timeout,
        })
    }

    fn strategy(&self, value: Option<&Spanned<String>>) -> Result<Strategy, ConfigError> {
        let Some(value) = value else {
            return Ok(Strategy::default());
        };
        match value.get_ref().as_str() {
            "timeout" => Ok(Strategy::Timeout),
            "permissive" => Ok(Strategy::Permissive),
            "eager" => Ok(Strategy::Eager),
            other => Err(self.error(
                value.span(),
                tr!(
                    "Unknown strategy \"{other}\"; the options are timeout, permissive and eager",
                    other = other
                ),
            )),
        }
    }

    /// Parses a located value with `parse`, pointing errors at that value.
    fn at<T: ?Sized, U>(
        &self,
        value: &Spanned<impl AsRef<T>>,
        parse: impl FnOnce(&T) -> Result<U, String>,
    ) -> Result<U, ConfigError> {
        parse(value.get_ref().as_ref()).map_err(|msg| self.error(value.span(), msg))
    }

    fn error(&self, span: Range<usize>, message: String) -> ConfigError {
        let (line, column) = self.line_col(span.start);
        ConfigError::Invalid {
            line,
            column,
            message,
        }
    }

    fn line_col(&self, offset: usize) -> (usize, usize) {
        let before = &self.src[..offset.min(self.src.len())];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        (line, column)
    }
}

impl AsRef<OneOrMany> for OneOrMany {
    fn as_ref(&self) -> &OneOrMany {
        self
    }
}

fn language(tag: &str) -> Result<Lang, String> {
    match tag {
        "en" => Ok(Lang::En),
        "zh-CN" => Ok(Lang::ZhCn),
        _ => Err(tr!(
            "Unknown language \"{tag}\"; the options are \"en\" and \"zh-CN\"",
            tag = tag
        )),
    }
}

/// When a tap/hold rule has no tap, a tap posts the trigger key itself (except Caps Lock).
fn default_tap(key: Key) -> Action {
    match key {
        Key::CapsLock => Action::Disable,
        key => Action::Keys(vec![Chord::key(key)]),
    }
}

#[cfg(test)]
mod tests;
