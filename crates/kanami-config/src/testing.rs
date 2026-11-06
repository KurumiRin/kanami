//! `[[test]]`: replays key sequences offline and checks the engine's output.

use std::fmt::Write;

use kanami_core::{Engine, Hooks, Key, KeyEvent, KeyState, Out, Rules};
use kanami_i18n::tr;

use crate::expr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Key(Key, KeyState),
    Wait(u64),
}

/// One item of output: a key event or typed text.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Output {
    Key(Key, KeyState),
    Text(String),
}

#[derive(Clone, Debug)]
pub struct TestCase {
    pub name: String,
    /// The line number in the config file.
    pub line: usize,
    app: Option<String>,
    input_source: Option<String>,
    device: Option<String>,
    input: Vec<Step>,
    expect: Vec<Output>,
}

/// The expected and actual output of a failed test, written in sequence notation.
#[derive(Debug)]
pub struct Mismatch {
    pub expected: String,
    pub actual: String,
}

/// Which field failed to parse.
pub(crate) enum Field {
    Input,
    Expect,
}

impl TestCase {
    pub(crate) fn new(
        name: String,
        line: usize,
        app: Option<String>,
        input_source: Option<String>,
        device: Option<String>,
        input: &str,
        expect: &str,
    ) -> Result<Self, (Field, String)> {
        let input = parse(input, true)
            .map_err(|e| (Field::Input, e))?
            .into_iter()
            .map(|token| match token {
                Token::Step(step) => Ok(step),
                Token::Text(_) => Err((Field::Input, tr!("input cannot contain text"))),
            })
            .collect::<Result<_, _>>()?;
        let expect = parse(expect, false)
            .map_err(|e| (Field::Expect, e))?
            .into_iter()
            .filter_map(|token| match token {
                Token::Step(Step::Key(key, state)) => Some(Output::Key(key, state)),
                Token::Step(Step::Wait(_)) => None,
                Token::Text(text) => Some(Output::Text(text)),
            })
            .collect();
        Ok(TestCase {
            name,
            line,
            app,
            input_source,
            device,
            input,
            expect,
        })
    }

    /// Replays the test. When the config uses scripts, `hooks` is the extension with the scripts loaded.
    pub fn run(&self, rules: &Rules, hooks: Option<Box<dyn Hooks>>) -> Result<(), Mismatch> {
        let mut engine =
            Engine::new(rules).expect("rules were validated when the config was loaded");
        if let Some(hooks) = hooks {
            engine.set_hooks(hooks);
        }
        engine.set_app(self.app.as_deref());
        engine.set_input_source(self.input_source.as_deref());
        engine.assume_device(self.device.as_deref());
        let mut now = 0;
        let mut actual = Vec::new();
        for &step in &self.input {
            match step {
                Step::Key(key, state) => {
                    collect(&mut actual, engine.process(KeyEvent::new(key, state), now))
                }
                Step::Wait(ms) => {
                    now += ms;
                    collect(&mut actual, engine.tick(now));
                }
            }
        }
        // Let pending tap/hold keys, combos and script timers run out
        while let Some(deadline) = engine.next_deadline() {
            now = deadline.max(now);
            collect(&mut actual, engine.tick(now));
        }

        if actual == self.expect {
            Ok(())
        } else {
            Err(Mismatch {
                expected: format_outputs(&self.expect),
                actual: format_outputs(&actual),
            })
        }
    }
}

/// Inserts text between the key events by position; auto-repeat is ignored.
fn collect(actual: &mut Vec<Output>, out: &Out) {
    let mut texts = out.texts.iter().peekable();
    for (i, event) in out.keys.iter().enumerate() {
        while let Some((_, text)) = texts.next_if(|(at, _)| *at == i) {
            actual.push(Output::Text(text.to_string()));
        }
        if event.state != KeyState::Repeat {
            actual.push(Output::Key(event.key, event.state));
        }
    }
    actual.extend(texts.map(|(_, text)| Output::Text(text.to_string())));
}

enum Token {
    Step(Step),
    Text(String),
}

/// Sequences are whitespace-separated; text in double quotes (which may contain spaces) is typed text, such as `"hello world"`.
fn parse(seq: &str, allow_wait: bool) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut rest = seq.trim_start();
    while !rest.is_empty() {
        if let Some(quoted) = rest.strip_prefix('"') {
            let end = quoted
                .find('"')
                .ok_or_else(|| tr!("The text is missing its closing double quote"))?;
            tokens.push(Token::Text(quoted[..end].to_owned()));
            rest = quoted[end + 1..].trim_start();
            continue;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let token = &rest[..end];
        rest = rest[end..].trim_start();
        if let Some(ms) = token.strip_prefix('@') {
            if !allow_wait {
                return Err(tr!("expect cannot contain a wait"));
            }
            let ms = ms.parse().map_err(|_| {
                tr!(
                    "\"{token}\" should be written as @milliseconds, such as @300",
                    token = token
                )
            })?;
            tokens.push(Token::Step(Step::Wait(ms)));
        } else if let Some(name) = token.strip_prefix('+') {
            tokens.push(Token::Step(Step::Key(
                expr::single_key(name)?,
                KeyState::Down,
            )));
        } else if let Some(name) = token.strip_prefix('-') {
            tokens.push(Token::Step(Step::Key(
                expr::single_key(name)?,
                KeyState::Up,
            )));
        } else {
            for (key, down) in expr::tap_keys(token)? {
                let state = if down { KeyState::Down } else { KeyState::Up };
                tokens.push(Token::Step(Step::Key(key, state)));
            }
        }
    }
    Ok(tokens)
}

fn format_outputs(outputs: &[Output]) -> String {
    let mut s = String::new();
    for (i, output) in outputs.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        let _ = match output {
            Output::Key(key, state) => {
                let sign = if *state == KeyState::Down { '+' } else { '-' };
                write!(s, "{sign}{}", key.name())
            }
            Output::Text(text) => write!(s, "\"{text}\""),
        };
    }
    if s.is_empty() {
        s.push_str(&tr!("(no output)"));
    }
    s
}
