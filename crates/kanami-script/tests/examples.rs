//! Runs the [[test]] blocks of the example configs in examples/ that use scripts, so the bundled plugins do not regress.

use std::path::Path;

use kanami_script::ScriptHost;

#[test]
fn scripting_example_tests_pass() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/scripting.toml");
    let config = kanami_config::load(&path).unwrap();
    assert!(!config.tests.is_empty());
    for test in &config.tests {
        let host = ScriptHost::load(&config.scripts, Box::new(|line| eprintln!("{line}"))).unwrap();
        if let Err(m) = test.run(&config.rules, Some(Box::new(host))) {
            panic!("{}: expected {}, got {}", test.name, m.expected, m.actual);
        }
    }
}
