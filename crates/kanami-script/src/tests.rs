use std::cell::RefCell;
use std::rc::Rc;

use kanami_config::{Script, parse};
use kanami_core::{Engine, Key, KeyEvent, KeyState};

use super::*;

/// Loads inline scripts, attaches them to an engine, and returns the engine and the collected logs.
fn engine(config: &str) -> (Engine, Rc<RefCell<Vec<String>>>) {
    let config = parse(config).unwrap();
    let logs = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&logs);
    let host = ScriptHost::load(
        &config.scripts,
        Box::new(move |l| sink.borrow_mut().push(l)),
    )
    .unwrap();
    let mut engine = Engine::new(&config.rules).unwrap();
    assert!(host.missing(engine.functions()).is_empty());
    engine.set_hooks(Box::new(host));
    (engine, logs)
}

fn tap(engine: &mut Engine, key: Key, now: u64) -> Vec<(Key, KeyState)> {
    let mut out: Vec<_> = engine
        .process(KeyEvent::new(key, KeyState::Down), now)
        .keys
        .iter()
        .map(|e| (e.key, e.state))
        .collect();
    out.extend(
        engine
            .process(KeyEvent::new(key, KeyState::Up), now)
            .keys
            .iter()
            .map(|e| (e.key, e.state)),
    );
    out
}

#[test]
fn call_taps_keys() {
    let (mut e, _) = engine(
        r#"
        [script]
        source = 'function copy() kanami.tap("cmd+c") end'
        [[rule]]
        from = "f5"
        call = "copy"
        "#,
    );
    assert_eq!(
        tap(&mut e, Key::F5, 0),
        [
            (Key::LeftCommand, KeyState::Down),
            (Key::C, KeyState::Down),
            (Key::C, KeyState::Up),
            (Key::LeftCommand, KeyState::Up)
        ]
    );
}

#[test]
fn on_key_and_vars() {
    let (mut e, _) = engine(
        r#"
        [script]
        source = '''
        kanami.on_key(function(e)
            if e.key == "f5" and e.state == "down" then
                kanami.set("mode", kanami.get("mode") == "vim" and nil or "vim")
                return true
            end
        end)
        '''
        [[rule]]
        from = "h"
        to = "left"
        when.var = { mode = "vim" }
        "#,
    );
    assert_eq!(tap(&mut e, Key::H, 0)[0].0, Key::H);
    assert!(tap(&mut e, Key::F5, 0).is_empty());
    assert_eq!(tap(&mut e, Key::H, 0)[0].0, Key::Left);
}

#[test]
fn timers_and_text() {
    let (mut e, _) = engine(
        r#"
        [script]
        source = 'function later() kanami.after(100, function() kanami.type("你好") end) end'
        [[rule]]
        from = "f5"
        call = "later"
        "#,
    );
    tap(&mut e, Key::F5, 0);
    assert_eq!(e.next_deadline(), Some(100));
    assert_eq!(&*e.tick(100).texts[0].1, "你好");
}

#[test]
fn runaway_script_is_stopped() {
    let (mut e, logs) = engine(
        r#"
        [script]
        source = 'function spin() while true do end end'
        [[rule]]
        from = "f5"
        call = "spin"
        "#,
    );
    let started = std::time::Instant::now();
    tap(&mut e, Key::F5, 0);
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    assert!(
        logs.borrow()[0].contains("timed out"),
        "{:?}",
        logs.borrow()
    );
    // Works normally afterwards
    assert_eq!(tap(&mut e, Key::A, 0)[0].0, Key::A);
}

#[test]
fn sandbox_has_no_system_access() {
    let (mut e, logs) = engine(
        r#"
        [script]
        source = '''
        function probe()
            kanami.log(type(io), type(os.execute), type(require))
        end
        '''
        [[rule]]
        from = "f5"
        call = "probe"
        "#,
    );
    tap(&mut e, Key::F5, 0);
    assert_eq!(logs.borrow()[0], "[script] nil nil nil");
}

#[test]
fn reports_load_errors_and_missing_functions() {
    let bad = [Script {
        name: "bad.luau".into(),
        code: "function (".into(),
    }];
    let err = ScriptHost::load(&bad, Box::new(|_| {})).err().unwrap();
    assert!(err.contains("bad.luau"), "{err}");

    let outside = [Script {
        name: "x".into(),
        code: "kanami.tap('a')".into(),
    }];
    let err = ScriptHost::load(&outside, Box::new(|_| {})).err().unwrap();
    assert!(err.contains("only be used inside a callback"), "{err}");

    let host = ScriptHost::load(&[], Box::new(|_| {})).unwrap();
    assert_eq!(host.missing(["nope"].into_iter()), ["nope"]);
}
