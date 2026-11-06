use super::*;

const EXAMPLE: &str = include_str!("../../../config.example.toml");

fn invalid(src: &str) -> (usize, usize, String) {
    match parse(src) {
        Err(ConfigError::Invalid {
            line,
            column,
            message,
        }) => (line, column, message),
        other => panic!("expected an Invalid error, got {other:?}"),
    }
}

#[test]
fn example_config_tests_pass() {
    let config = parse(EXAMPLE).unwrap();
    assert!(!config.tests.is_empty());
    for test in &config.tests {
        if let Err(m) = test.run(&config.rules, None) {
            panic!("{}: expected {}, got {}", test.name, m.expected, m.actual);
        }
    }
}

#[test]
fn empty_config_is_valid() {
    let config = parse("").unwrap();
    assert_eq!(config.rules, Rules::default());
}

#[test]
fn reports_unknown_key_position() {
    let (line, column, message) = invalid("[remap]\ngrave = \"esacpe\"\n");
    assert_eq!((line, column), (2, 9));
    assert!(message.contains("\"escape\""), "{message}");
}

#[test]
fn reports_rule_shape_errors() {
    let (line, _, message) = invalid("\n[[rule]]\nfrom = \"a\"\nto = \"b\"\nhold = \"c\"\n");
    assert_eq!(line, 2);
    assert!(message.contains("only one of"), "{message}");

    let (_, _, message) = invalid("[[rule]]\nfrom = \"a\"\ntap = \"b\"\n");
    assert!(message.contains("hold"), "{message}");
}

#[test]
fn maps_engine_errors_to_source() {
    let src = "[remap]\na = \"b\"\n\n[[rule]]\nfrom = \"cmd+k\"\ntap = \"a\"\nhold = \"ctrl\"\n";
    let (line, _, message) = invalid(src);
    assert_eq!(line, 4);
    assert!(message.contains("single key"), "{message}");
}

#[test]
fn rejects_unknown_fields() {
    assert!(matches!(parse("[remapp]"), Err(ConfigError::Toml(_))));
    assert!(matches!(
        parse("[[rule]]\nfrom = \"a\"\nto = \"b\"\nwhen.keyboard = []\n"),
        Err(ConfigError::Toml(_))
    ));
}

#[test]
fn reports_bad_strategy() {
    let (_, _, message) =
        invalid("[[rule]]\nfrom = \"a\"\ntap = \"a\"\nhold = \"ctrl\"\nstrategy = \"lazy\"\n");
    assert!(message.contains("eager"), "{message}");
}

#[test]
fn failing_test_reports_mismatch() {
    let config = parse("[remap]\na = \"b\"\n\n[[test]]\ninput = \"a\"\nexpect = \"a\"\n").unwrap();
    let mismatch = config.tests[0].run(&config.rules, None).unwrap_err();
    assert_eq!(mismatch.expected, "+a -a");
    assert_eq!(mismatch.actual, "+b -b");
    assert_eq!(config.tests[0].line, 4);
}

#[test]
fn reports_bad_test_sequence() {
    let (line, _, message) = invalid("[[test]]\ninput = \"a\"\nexpect = \"@100\"\n");
    assert_eq!(line, 3);
    assert!(message.contains("wait"), "{message}");
}

#[test]
fn schema_file_is_up_to_date() {
    let checked_in = include_str!("../../../schema/kanami.schema.json");
    assert!(
        checked_in == schema(),
        "schema/kanami.schema.json is out of date; run `cargo run -- schema > schema/kanami.schema.json`"
    );
}

#[test]
fn parses_scripts_calls_and_conditions() {
    let dir = std::env::temp_dir().join(format!("kanami-config-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.luau"), "function f() end").unwrap();
    let src = r#"
        [script]
        files = ["a.luau"]
        source = "function g() end"

        [[rule]]
        from = "f5"
        call = "f"
        when.var = { mode = "vim", on = true }
        when.input_source = ["com.apple.keylayout.ABC"]

        [[combo]]
        keys = ["j", "k"]
        call = "g"
    "#;
    let config = parse_in(src, Some(&dir)).unwrap();
    assert_eq!(config.scripts.len(), 2);
    assert_eq!(config.scripts[0].code, "function f() end");
    assert_eq!(config.watched, [dir.join("a.luau")]);
    let rule = &config.rules.rules[0];
    assert_eq!(rule.kind, RuleKind::Action(Action::Call("f".into())));
    assert_eq!(
        rule.when.vars,
        [("mode".into(), "vim".into()), ("on".into(), "true".into())]
    );
    assert_eq!(rule.when.input_source, ["com.apple.keylayout.ABC"]);
    assert_eq!(config.rules.combos[0].action, Action::Call("g".into()));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn reports_missing_script_file() {
    let (line, _, message) = invalid("[script]\nfiles = [\"nope.luau\"]\n");
    assert_eq!(line, 2);
    assert!(message.contains("nope.luau"), "{message}");
}

#[test]
fn test_sequences_support_text() {
    let config = parse("[[test]]\ninput = \"a\"\nexpect = '\"hello world\"'\n").unwrap();
    let mismatch = config.tests[0].run(&config.rules, None).unwrap_err();
    assert_eq!(mismatch.expected, "\"hello world\"");
    assert_eq!(mismatch.actual, "+a -a");
}

#[test]
fn device_rules_and_tests() {
    let config = parse(
        r#"
        [device.ext]
        product = "Magic Keyboard"

        [[rule]]
        from = "left_option"
        to = "left_command"
        when.device = ["ext"]

        [[test]]
        device = "ext"
        input = "left_option"
        expect = "left_command"

        [[test]]
        input = "left_option"
        expect = "left_option"
        "#,
    )
    .unwrap();
    for test in &config.tests {
        test.run(&config.rules, None).unwrap();
    }

    let (line, _, message) = invalid("[[rule]]\nfrom = \"a\"\nto = \"b\"\nwhen.device = [\"x\"]\n");
    assert_eq!(line, 1);
    assert!(message.contains("device.x"), "{message}");

    let (_, _, message) = invalid("[device.empty]\n");
    assert!(message.contains("vendor_id"), "{message}");
}

fn run_tests(src: &str) {
    let config = parse(src).unwrap();
    assert!(!config.tests.is_empty());
    for test in &config.tests {
        if let Err(m) = test.run(&config.rules, None) {
            panic!("{}: expected {}, got {}", test.name, m.expected, m.actual);
        }
    }
}

#[test]
fn disabled_rules_and_groups() {
    run_tests(
        r#"
[[rule]]
from = "a"
to = "b"
enabled = false

[[group]]
name = "In the terminal"
when.app = ["com.apple.Terminal"]

[[group.rule]]
from = "c"
to = "d"

[[group]]
name = "Disabled group"
enabled = false

[[group.rule]]
from = "e"
to = "f"

[[test]]
input = "a e"
expect = "a e"

[[test]]
input = "c"
expect = "c"

[[test]]
app = "com.apple.Terminal"
input = "c"
expect = "d"
"#,
    );
}

#[test]
fn rules_follow_file_order_across_groups() {
    // Rules in a group written earlier match before top-level rules written later
    run_tests(
        r#"
[[group]]
name = "First"

[[group.rule]]
from = "a"
to = "b"

[[rule]]
from = "a"
to = "c"

[[test]]
input = "a"
expect = "b"
"#,
    );
}

#[test]
fn device_rules_remaps_and_inherit() {
    run_tests(
        r#"
[remap]
x = "y"

[[rule]]
from = "a"
to = "b"

[device.niz]
product = "NIZ"

[device.niz.remap]
left_option = "left_command"

[[device.niz.rule]]
from = "a"
to = "c"

[device.hhkb]
product = "HHKB"
inherit = false

[[device.hhkb.group]]
name = "HHKB only"

[[device.hhkb.group.rule]]
from = "d"
to = "e"

[[test]]
input = "a x d"
expect = "b y d"

[[test]]
device = "niz"
input = "a x +left_option -left_option"
expect = "c y +left_command -left_command"

[[test]]
device = "hhkb"
input = "a x d"
expect = "a x e"
"#,
    );
}

#[test]
fn device_sections_reject_device_conditions() {
    let (_, _, message) = invalid(
        r#"
[device.niz]
product = "NIZ"

[[device.niz.rule]]
from = "a"
to = "b"
when.device = ["niz"]
"#,
    );
    assert!(
        message.contains("when.device cannot be used here"),
        "{message}"
    );
}

#[test]
fn new_key_names_parse() {
    run_tests(
        r#"
[remap]
keypad_enter = "enter"
menu = "right_command"
print_screen = "f13"

[[test]]
input = "keypad_enter num_lock insert"
expect = "enter num_lock insert"
"#,
    );
}

#[test]
fn language_setting() {
    assert_eq!(parse("").unwrap().rules.settings.language, None);
    let en = parse("[settings]\nlanguage = \"en\"\n").unwrap();
    assert_eq!(en.rules.settings.language, Some(kanami_i18n::Lang::En));
    let zh = parse("[settings]\nlanguage = \"zh-CN\"\n").unwrap();
    assert_eq!(zh.rules.settings.language, Some(kanami_i18n::Lang::ZhCn));

    // Only the exact tags are accepted.
    let (line, column, message) = invalid("[settings]\nlanguage = \"zh\"\n");
    assert_eq!((line, column), (2, 12));
    assert!(message.contains("\"zh-CN\""), "{message}");
    assert!(message.contains("Unknown language \"zh\""), "{message}");
    invalid("[settings]\nlanguage = \"EN\"\n");
}
