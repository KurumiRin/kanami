use super::*;
use crate::{Api, Hooks, ModKind, Side};

use Key::*;
use KeyState::{Down as Press, Up as Release};

/// Feeds in events in order and returns the presses/releases in the output (repeats ignored).
struct Harness {
    engine: Engine,
    now: u64,
    output: Vec<(Key, KeyState)>,
}

impl Harness {
    /// Turns off the typing streak: most events here happen at the same instant and would count as continuous typing.
    fn new(mut rules: Rules) -> Self {
        rules.settings.typing_streak = 0;
        Self::exact(rules)
    }

    fn exact(rules: Rules) -> Self {
        Harness {
            engine: Engine::new(&rules).unwrap(),
            now: 0,
            output: Vec::new(),
        }
    }

    fn collect(&mut self, keys: &[KeyEvent]) {
        self.output.extend(
            keys.iter()
                .filter(|e| e.state != KeyState::Repeat)
                .map(|e| (e.key, e.state)),
        );
    }

    fn down(&mut self, key: Key) -> &mut Self {
        self.event(key, Press)
    }

    fn up(&mut self, key: Key) -> &mut Self {
        self.event(key, Release)
    }

    fn tap(&mut self, key: Key) -> &mut Self {
        self.down(key).up(key)
    }

    fn event(&mut self, key: Key, state: KeyState) -> &mut Self {
        let out = self.engine.process(KeyEvent::new(key, state), self.now);
        let keys = out.keys.clone();
        self.collect(&keys);
        self
    }

    fn wait(&mut self, ms: u64) -> &mut Self {
        self.now += ms;
        let keys = self.engine.tick(self.now).keys.clone();
        self.collect(&keys);
        self
    }

    fn take(&mut self) -> Vec<(Key, KeyState)> {
        std::mem::take(&mut self.output)
    }
}

fn tapped(keys: &[Key]) -> Vec<(Key, KeyState)> {
    keys.iter()
        .flat_map(|&k| [(k, Press), (k, Release)])
        .collect()
}

fn remaps(pairs: &[(Key, RemapTarget)]) -> Rules {
    Rules {
        remaps: pairs
            .iter()
            .map(|&(from, to)| Remap {
                from,
                to,
                device: None,
            })
            .collect(),
        ..Rules::default()
    }
}

fn cmd() -> ModPattern {
    let mut p = ModPattern::default();
    p.require(ModKind::Command, Side::Either);
    p
}

fn rule(from: Trigger, kind: RuleKind) -> Rule {
    Rule {
        from,
        kind,
        when: Condition::default(),
        scope: Condition::default(),
    }
}

fn bare(key: Key) -> Trigger {
    Trigger { key, mods: None }
}

fn keys(chords: &[Chord]) -> Action {
    Action::Keys(chords.to_vec())
}

fn tap_hold(from: Key, tap: Key, hold: Chord, strategy: Strategy) -> Rules {
    Rules {
        rules: vec![rule(
            bare(from),
            RuleKind::TapHold(TapHold {
                tap: keys(&[Chord::key(tap)]),
                hold,
                strategy,
            }),
        )],
        ..Rules::default()
    }
}

fn nav_layer() -> Rules {
    Rules {
        layers: vec![Layer {
            name: "nav".into(),
            activation: Activation::Hold {
                key: Space,
                tap: None,
                strategy: Strategy::Permissive,
            },
            map: vec![(J, keys(&[Chord::key(Down)])), (K, keys(&[Chord::key(Up)]))],
        }],
        ..Rules::default()
    }
}

// ---- Stage one ----

#[test]
fn unmapped_keys_pass() {
    let mut h = Harness::new(Rules::default());
    assert_eq!(h.tap(A).take(), tapped(&[A]));
}

#[test]
fn swap_uses_original_key() {
    let mut h = Harness::new(remaps(&[
        (A, RemapTarget::Key(B)),
        (B, RemapTarget::Key(A)),
    ]));
    assert_eq!(h.tap(A).tap(B).take(), tapped(&[B, A]));
}

#[test]
fn disable_swallows_key() {
    let mut h = Harness::new(remaps(&[(F1, RemapTarget::Disable)]));
    assert_eq!(h.tap(F1).take(), []);
}

#[test]
fn remap_to_modifier_applies_to_next_key() {
    let mut h = Harness::new(remaps(&[(CapsLock, RemapTarget::Key(LeftControl))]));
    let mut mods = Vec::new();
    let out = h
        .engine
        .process(KeyEvent::new(CapsLock, Press), 0)
        .keys
        .clone();
    assert_eq!(out[0].key, LeftControl);
    let out = h.engine.process(KeyEvent::new(A, Press), 0).keys.clone();
    mods.push(out[0].mods);
    assert_eq!(mods, [ModSet::LEFT_CONTROL]);
}

#[test]
fn remap_to_hyper() {
    let mut h = Harness::new(remaps(&[(CapsLock, RemapTarget::Mods(ModSet::HYPER))]));
    h.down(CapsLock);
    let pressed: Vec<Key> = h.take().into_iter().map(|(k, _)| k).collect();
    assert_eq!(pressed, ModSet::HYPER.keys().collect::<Vec<_>>());
    h.up(CapsLock);
    assert!(h.take().iter().all(|&(_, s)| s == Release));
}

#[test]
fn swap_caps_lock_and_control() {
    let mut h = Harness::new(remaps(&[
        (CapsLock, RemapTarget::Key(LeftControl)),
        (LeftControl, RemapTarget::Key(CapsLock)),
    ]));
    assert_eq!(h.tap(LeftControl).take(), tapped(&[CapsLock]));

    // The physical Control key adds no modifier, and Caps Lock adds Control.
    let out = h.engine.process(KeyEvent::new(LeftControl, Press), 0);
    assert_eq!(out.keys[0].key, CapsLock);
    let out = h.engine.process(KeyEvent::new(A, Press), 0);
    assert_eq!(out.keys[0].mods, ModSet::empty());
    h.engine.process(KeyEvent::new(A, Release), 0);
    h.engine.process(KeyEvent::new(LeftControl, Release), 0);
    h.engine.process(KeyEvent::new(CapsLock, Press), 0);
    let out = h.engine.process(KeyEvent::new(A, Press), 0);
    assert_eq!(out.keys[0].mods, ModSet::LEFT_CONTROL);
}

// ---- Rules ----

#[test]
fn chord_rule_swaps_modifiers_and_restores() {
    let rules = Rules {
        rules: vec![rule(
            Trigger {
                key: K,
                mods: Some(cmd()),
            },
            RuleKind::Action(keys(&[Chord {
                mods: ModSet::LEFT_CONTROL,
                key: Some(L),
            }])),
        )],
        ..Rules::default()
    };
    let mut h = Harness::new(rules);
    h.down(LeftCommand).tap(K).up(LeftCommand);
    assert_eq!(
        h.take(),
        [
            (LeftCommand, Press),
            (LeftCommand, Release),
            (LeftControl, Press),
            (L, Press),
            (L, Release),
            (LeftControl, Release),
            (LeftCommand, Press),
            (LeftCommand, Release),
        ]
    );
}

#[test]
fn chord_rule_is_exact() {
    let rules = Rules {
        rules: vec![rule(
            Trigger {
                key: K,
                mods: Some(cmd()),
            },
            RuleKind::Action(Action::Disable),
        )],
        ..Rules::default()
    };
    let mut h = Harness::new(rules);
    h.down(LeftCommand).down(LeftShift).tap(K);
    assert_eq!(
        h.take(),
        [
            (LeftCommand, Press),
            (LeftShift, Press),
            (K, Press),
            (K, Release)
        ]
    );
}

#[test]
fn bare_rule_keeps_modifiers() {
    let rules = Rules {
        rules: vec![rule(bare(A), RuleKind::Action(keys(&[Chord::key(B)])))],
        ..Rules::default()
    };
    let mut h = Harness::new(rules);
    h.down(LeftShift);
    let out = h.engine.process(KeyEvent::new(A, Press), 0).keys.clone();
    assert_eq!((out[0].key, out[0].mods), (B, ModSet::LEFT_SHIFT));
}

#[test]
fn sequence_taps_each_chord() {
    let copy_all = [
        Chord {
            mods: ModSet::LEFT_COMMAND,
            key: Some(A),
        },
        Chord {
            mods: ModSet::LEFT_COMMAND,
            key: Some(C),
        },
    ];
    let rules = Rules {
        rules: vec![rule(bare(F5), RuleKind::Action(keys(&copy_all)))],
        ..Rules::default()
    };
    let mut h = Harness::new(rules);
    assert_eq!(
        h.tap(F5).take(),
        [
            (LeftCommand, Press),
            (A, Press),
            (A, Release),
            (LeftCommand, Release),
            (LeftCommand, Press),
            (C, Press),
            (C, Release),
            (LeftCommand, Release),
        ]
    );
}

#[test]
fn app_condition() {
    let mut r = rule(bare(A), RuleKind::Action(keys(&[Chord::key(B)])));
    r.when.app = vec!["com.apple.Terminal".into()];
    let mut h = Harness::new(Rules {
        rules: vec![r],
        ..Rules::default()
    });
    assert_eq!(h.tap(A).take(), tapped(&[A]));
    h.engine.set_app(Some("com.apple.Terminal"));
    assert_eq!(h.tap(A).take(), tapped(&[B]));
    h.engine.set_app(Some("com.apple.Safari"));
    assert_eq!(h.tap(A).take(), tapped(&[A]));
}

#[test]
fn first_matching_rule_wins() {
    let rules = Rules {
        rules: vec![
            rule(bare(A), RuleKind::Action(keys(&[Chord::key(B)]))),
            rule(bare(A), RuleKind::Action(keys(&[Chord::key(C)]))),
        ],
        ..Rules::default()
    };
    let mut h = Harness::new(rules);
    assert_eq!(h.tap(A).take(), tapped(&[B]));
}

// ---- Tap / hold ----

#[test]
fn tap_hold_tap() {
    let mut h = Harness::new(tap_hold(
        CapsLock,
        Escape,
        Chord::mods(ModSet::LEFT_CONTROL),
        Strategy::Permissive,
    ));
    assert_eq!(h.tap(CapsLock).take(), tapped(&[Escape]));
}

#[test]
fn tap_hold_permissive_hold() {
    let mut h = Harness::new(tap_hold(
        CapsLock,
        Escape,
        Chord::mods(ModSet::LEFT_CONTROL),
        Strategy::Permissive,
    ));
    h.down(CapsLock).tap(C).up(CapsLock);
    assert_eq!(
        h.take(),
        [
            (LeftControl, Press),
            (C, Press),
            (C, Release),
            (LeftControl, Release)
        ]
    );
}

#[test]
fn tap_hold_timeout() {
    let mut h = Harness::new(tap_hold(
        CapsLock,
        Escape,
        Chord::mods(ModSet::LEFT_CONTROL),
        Strategy::Permissive,
    ));
    h.down(CapsLock).wait(199);
    assert_eq!(h.take(), []);
    h.wait(1).up(CapsLock);
    assert_eq!(h.take(), [(LeftControl, Press), (LeftControl, Release)]);
}

#[test]
fn tap_hold_timeout_strategy_ignores_other_keys() {
    let mut h = Harness::new(tap_hold(
        CapsLock,
        Escape,
        Chord::mods(ModSet::LEFT_CONTROL),
        Strategy::Timeout,
    ));
    h.down(CapsLock).tap(C).up(CapsLock);
    assert_eq!(h.take(), tapped(&[Escape, C]));
}

#[test]
fn tap_hold_eager_decides_on_press() {
    let mut h = Harness::new(tap_hold(
        CapsLock,
        Escape,
        Chord::mods(ModSet::LEFT_CONTROL),
        Strategy::Eager,
    ));
    h.down(CapsLock).down(C);
    assert_eq!(h.take(), [(LeftControl, Press), (C, Press)]);
}

#[test]
fn tap_hold_hold_key() {
    let mut h = Harness::new(tap_hold(Tab, Tab, Chord::key(F13), Strategy::Permissive));
    h.down(Tab).wait(300).up(Tab);
    assert_eq!(h.take(), tapped(&[F13]));
}

// ---- Layers ----

#[test]
fn layer_rollover_is_tap() {
    let mut h = Harness::new(nav_layer());
    h.down(Space).down(B).up(Space).up(B);
    assert_eq!(
        h.take(),
        [(Space, Press), (Space, Release), (B, Press), (B, Release)]
    );
}

#[test]
fn layer_hold_maps_keys() {
    let mut h = Harness::new(nav_layer());
    h.down(Space).wait(300).tap(J).tap(L).up(Space);
    assert_eq!(h.take(), tapped(&[Down, L]));
    assert_eq!(h.tap(J).take(), tapped(&[J]));
}

#[test]
fn layer_permissive_hold() {
    let mut h = Harness::new(nav_layer());
    h.down(Space).tap(K).up(Space);
    assert_eq!(h.take(), tapped(&[Up]));
}

#[test]
fn long_press_without_use_is_still_tap() {
    let mut h = Harness::new(nav_layer());
    // Held longer than the time limit but no other key was pressed: the space is still posted on release
    h.down(Space).wait(400).up(Space);
    assert_eq!(h.take(), tapped(&[Space]));
    // If the layer was used, nothing is posted
    h.down(Space).wait(300).tap(J).up(Space);
    assert_eq!(h.take(), tapped(&[Down]));
    h.down(Space)
        .wait(300)
        .down(LeftCommand)
        .up(LeftCommand)
        .up(Space);
    assert!(!h.take().contains(&(Space, Press)));
}

fn streaking(mut rules: Rules) -> Harness {
    rules.settings.typing_streak = 150;
    Harness::exact(rules)
}

#[test]
fn typing_streak_makes_layer_key_plain() {
    let mut h = streaking(nav_layer());
    // Space pressed mid-typing: posted immediately, and J pressed while it is held is an ordinary key
    h.tap(A).wait(80).down(Space);
    assert_eq!(h.take(), [(A, Press), (A, Release), (Space, Press)]);
    h.tap(J).up(Space);
    assert_eq!(h.take(), [(J, Press), (J, Release), (Space, Release)]);
    // After a pause, decide as usual
    h.wait(200).down(Space).wait(300).tap(J).up(Space);
    assert_eq!(h.take(), tapped(&[Down]));
}

#[test]
fn typing_streak_ignores_modifiers() {
    let mut h = streaking(tap_hold(
        CapsLock,
        Escape,
        Chord::mods(ModSet::LEFT_CONTROL),
        Strategy::Permissive,
    ));
    h.tap(A)
        .wait(300)
        .down(LeftShift)
        .up(LeftShift)
        .down(CapsLock)
        .tap(C)
        .up(CapsLock);
    let out = h.take();
    assert!(out.contains(&(LeftControl, Press)), "{out:?}");
    // Pressed right after a letter: treated as a tap directly
    h.tap(A).down(CapsLock).up(CapsLock);
    assert_eq!(h.take(), tapped(&[A, Escape]));
}

#[test]
fn typing_streak_skips_combo_wait() {
    let mut h = streaking(jk_combo());
    h.tap(A).down(J);
    assert_eq!(h.take(), [(A, Press), (A, Release), (J, Press)]);
    h.down(K).up(J).up(K);
    assert_eq!(h.take(), [(K, Press), (J, Release), (K, Release)]);
    h.wait(200).down(J).down(K);
    assert_eq!(h.take(), tapped(&[Escape]));
}

#[test]
fn layer_trigger_with_modifier_is_plain_key() {
    let mut h = Harness::new(nav_layer());
    h.down(LeftCommand).down(Space).wait(300);
    assert_eq!(
        h.take(),
        [(LeftCommand, Press), (Space, Press)],
        "Cmd+Space is posted immediately"
    );
    h.tap(J).up(Space).up(LeftCommand);
    assert_eq!(
        h.take(),
        [
            (J, Press),
            (J, Release),
            (Space, Release),
            (LeftCommand, Release)
        ]
    );
}

#[test]
fn emergency_stop_passes_everything_and_resumes() {
    let mut rules = nav_layer();
    rules.remaps = vec![Remap {
        from: A,
        to: RemapTarget::Key(B),
        device: None,
    }];
    let mut h = Harness::new(rules);

    // Disable: release the keys the engine pressed, and swallow the Esc that completes the combo
    h.down(LeftControl).down(Space).down(Escape);
    assert!(h.engine.stopped());
    assert_eq!(
        h.take(),
        [
            (LeftControl, Press),
            (Space, Press),
            (Space, Release),
            (LeftControl, Release)
        ]
    );
    h.up(Escape).up(Space).up(LeftControl).tap(A);
    assert_eq!(
        h.take(),
        [
            (Escape, Release),
            (Space, Release),
            (LeftControl, Release),
            (A, Press),
            (A, Release)
        ],
        "Passed through unchanged while disabled, with no remapping"
    );

    // Re-enable: the control pressed while disabled was already sent to the system, and counts as a modifier as usual afterwards
    h.down(LeftControl).down(Space).down(Escape);
    assert!(!h.engine.stopped());
    assert_eq!(h.take(), [(LeftControl, Press), (Space, Press)]);
    assert_eq!(h.engine.mods(), ModSet::LEFT_CONTROL);
    h.tap(A).up(Escape).up(Space).up(LeftControl);
    assert_eq!(
        h.take(),
        [
            (B, Press),
            (B, Release),
            (Space, Release),
            (LeftControl, Release)
        ]
    );
}

#[test]
fn emergency_stop_needs_physical_keys() {
    // Caps remapped to control does not count: only the physical left Control does
    let mut h = Harness::new(remaps(&[(
        CapsLock,
        RemapTarget::Mods(ModSet::LEFT_CONTROL),
    )]));
    h.down(CapsLock).down(Space).down(Escape);
    assert!(!h.engine.stopped());
}

#[test]
fn held_tracks_physical_keys_through_layers() {
    let mut h = Harness::new(nav_layer());
    h.down(Space).wait(300).down(J);
    assert!(h.engine.is_held(Space) && h.engine.is_held(J));
    // When a release did not pass through kanami, the release the backend delivers must finish both the layer and the key
    h.take();
    h.up(J).up(Space);
    assert_eq!(h.take(), [(Down, Release)]);
    assert!(!h.engine.is_held(Space) && !h.engine.is_held(J));
}

#[test]
fn key_released_after_layer_ends() {
    let mut h = Harness::new(nav_layer());
    h.down(Space).wait(300).down(J).up(Space).up(J);
    assert_eq!(h.take(), tapped(&[Down]));
}

#[test]
fn toggle_layer() {
    let rules = Rules {
        layers: vec![Layer {
            name: "num".into(),
            activation: Activation::Toggle(bare(F5)),
            map: vec![(J, keys(&[Chord::key(Num1)]))],
        }],
        ..Rules::default()
    };
    let mut h = Harness::new(rules);
    assert_eq!(h.tap(F5).tap(J).take(), tapped(&[Num1]));
    assert_eq!(h.tap(F5).tap(J).take(), tapped(&[J]));
}

// ---- Combos ----

fn jk_combo() -> Rules {
    Rules {
        combos: vec![Combo {
            keys: vec![J, K],
            action: keys(&[Chord::key(Escape)]),
            timeout: None,
        }],
        ..Rules::default()
    }
}

#[test]
fn combo_fires() {
    let mut h = Harness::new(jk_combo());
    h.down(K).down(J);
    assert_eq!(h.take(), tapped(&[Escape]));
    assert_eq!(h.up(J).up(K).take(), []);
}

#[test]
fn combo_times_out() {
    let mut h = Harness::new(jk_combo());
    h.down(J).wait(50).down(K).up(J).up(K);
    assert_eq!(
        h.take(),
        [(J, Press), (K, Press), (J, Release), (K, Release)]
    );
}

#[test]
fn combo_interrupted_by_other_key() {
    let mut h = Harness::new(jk_combo());
    h.down(J).down(X).up(J).up(X);
    assert_eq!(
        h.take(),
        [(J, Press), (X, Press), (J, Release), (X, Release)]
    );
}

#[test]
fn combo_prefers_longer_until_timeout() {
    let mut rules = jk_combo();
    rules.combos.push(Combo {
        keys: vec![J, K, L],
        action: keys(&[Chord::key(Enter)]),
        timeout: None,
    });
    let mut h = Harness::new(rules.clone());
    h.down(J).down(K);
    assert_eq!(h.take(), [], "Still waiting for the third key");
    h.wait(50);
    assert_eq!(h.take(), tapped(&[Escape]));

    let mut h = Harness::new(rules);
    h.down(J).down(K).down(L);
    assert_eq!(h.take(), tapped(&[Enter]));
}

// ---- Other ----

#[test]
fn reset_releases_everything() {
    let mut h = Harness::new(remaps(&[(CapsLock, RemapTarget::Key(LeftControl))]));
    h.down(CapsLock).down(A);
    h.take();
    let out = h.engine.reset().keys.clone();
    h.collect(&out);
    assert_eq!(h.take(), [(A, Release), (LeftControl, Release)]);
    // After a reset, the release of a physical key passes through unchanged
    assert_eq!(h.up(A).take(), [(A, Release)]);
}

#[test]
fn repeats_follow_mapping() {
    let mut engine = Engine::new(&remaps(&[(A, RemapTarget::Key(B))])).unwrap();
    engine.process(KeyEvent::new(A, Press), 0);
    let out = engine
        .process(KeyEvent::new(A, KeyState::Repeat), 0)
        .keys
        .clone();
    assert_eq!((out[0].key, out[0].state), (B, KeyState::Repeat));
}

#[test]
fn rejects_invalid_rules() {
    let chord_tap_hold = Rules {
        rules: vec![rule(
            Trigger {
                key: K,
                mods: Some(cmd()),
            },
            RuleKind::TapHold(TapHold {
                tap: Action::Disable,
                hold: Chord::mods(ModSet::LEFT_CONTROL),
                strategy: Strategy::Permissive,
            }),
        )],
        ..Rules::default()
    };
    let e = Engine::new(&chord_tap_hold).err().unwrap();
    assert_eq!(
        (e.at, e.kind),
        (Location::Rule(0), RuleErrorKind::TapHoldNeedsBareKey)
    );

    let modifier_combo = Rules {
        combos: vec![Combo {
            keys: vec![LeftShift, A],
            action: Action::Disable,
            timeout: None,
        }],
        ..Rules::default()
    };
    assert_eq!(
        Engine::new(&modifier_combo).err().unwrap().kind,
        RuleErrorKind::ComboModifier(LeftShift)
    );
}

#[test]
fn tracks_input_keys() {
    let engine = Engine::new(&nav_layer()).unwrap();
    assert!(engine.uses_key(Space));
    assert!(!engine.uses_key(CapsLock));
}

// ---- Extension interface ----

/// Test extension: `call("copy")` taps Cmd+C; `call("later")` types text after 100ms;
/// `on_key` swallows F5 and toggles the variable mode.
struct TestHooks;

impl Hooks for TestHooks {
    fn wants_keys(&self) -> bool {
        true
    }

    fn on_key(&mut self, api: &mut Api<'_>, event: KeyEvent) -> bool {
        if event.key != F5 || event.state != KeyState::Down {
            return false;
        }
        let next = if api.var("mode") == Some("vim") {
            None
        } else {
            Some("vim")
        };
        api.set_var("mode", next);
        true
    }

    fn call(&mut self, api: &mut Api<'_>, function: &str) {
        match function {
            "copy" => {
                api.tap(Chord {
                    mods: ModSet::LEFT_COMMAND,
                    key: Some(C),
                });
            }
            "later" => {
                api.after(100);
            }
            _ => {}
        }
    }

    fn timer(&mut self, api: &mut Api<'_>, _id: u32) {
        api.type_text("你好");
    }
}

fn hooked(rules: Rules) -> Harness {
    let mut h = Harness::new(rules);
    h.engine.set_hooks(Box::new(TestHooks));
    h
}

#[test]
fn call_runs_script_function() {
    let rules = Rules {
        rules: vec![rule(
            bare(F6),
            RuleKind::Action(Action::Call("copy".into())),
        )],
        ..Rules::default()
    };
    let mut h = hooked(rules);
    assert_eq!(
        h.tap(F6).take(),
        [
            (LeftCommand, Press),
            (C, Press),
            (C, Release),
            (LeftCommand, Release)
        ]
    );
    assert_eq!(h.engine.functions().collect::<Vec<_>>(), ["copy"]);
}

#[test]
fn script_timer_and_text() {
    let rules = Rules {
        rules: vec![rule(
            bare(F6),
            RuleKind::Action(Action::Call("later".into())),
        )],
        ..Rules::default()
    };
    let mut engine = Engine::new(&rules).unwrap();
    engine.set_hooks(Box::new(TestHooks));
    engine.process(KeyEvent::new(F6, Press), 0);
    assert_eq!(engine.next_deadline(), Some(100));
    assert!(engine.tick(99).texts.is_empty());
    let out = engine.tick(100);
    assert_eq!(out.texts, [(0, Arc::from("你好"))]);
    assert_eq!(engine.next_deadline(), None);
}

#[test]
fn on_key_swallows_and_vars_gate_rules() {
    let mut r = rule(bare(H), RuleKind::Action(keys(&[Chord::key(Left)])));
    r.when.vars = vec![("mode".into(), "vim".into())];
    let mut h = hooked(Rules {
        rules: vec![r],
        ..Rules::default()
    });
    assert_eq!(h.tap(H).take(), tapped(&[H]));
    assert_eq!(
        h.tap(F5).take(),
        [],
        "F5 is swallowed by the script, and its release does not leak out either"
    );
    assert_eq!(h.tap(H).take(), tapped(&[Left]));
    h.tap(F5);
    assert_eq!(h.tap(H).take(), tapped(&[H]));
}

#[test]
fn input_source_condition() {
    let mut r = rule(bare(A), RuleKind::Action(keys(&[Chord::key(B)])));
    r.when.input_source = vec!["com.apple.keylayout.ABC".into()];
    let mut h = Harness::new(Rules {
        rules: vec![r],
        ..Rules::default()
    });
    assert_eq!(h.tap(A).take(), tapped(&[A]));
    h.engine.set_input_source(Some("com.apple.keylayout.ABC"));
    assert_eq!(h.tap(A).take(), tapped(&[B]));
}

#[test]
fn run_outputs_command_text() {
    let rules = Rules {
        rules: vec![rule(
            bare(F5),
            RuleKind::Action(Action::Run("open -a Terminal".into())),
        )],
        ..Rules::default()
    };
    let mut engine = Engine::new(&rules).unwrap();
    let out = engine.process(KeyEvent::new(F5, Press), 0);
    assert_eq!(out.runs, [Arc::from("open -a Terminal")]);
}

#[test]
fn device_condition() {
    use crate::backend::DeviceInfo;
    let mut r = rule(
        bare(LeftOption),
        RuleKind::Action(keys(&[Chord::key(LeftCommand)])),
    );
    r.when.device = vec!["ext".into()];
    let rules = Rules {
        rules: vec![r],
        devices: vec![DeviceSpec {
            name: "ext".into(),
            vendor_id: Some(0x05ac),
            product: Some("magic".into()),
            ..DeviceSpec::default()
        }],
        ..Rules::default()
    };
    let internal = DeviceInfo {
        id: 1,
        vendor_id: 0x05ac,
        product_id: 0x0342,
        product: "Apple Internal Keyboard / Trackpad".into(),
    };
    let external = DeviceInfo {
        id: 2,
        vendor_id: 0x05ac,
        product_id: 0x029c,
        product: "Magic Keyboard".into(),
    };
    let mut h = Harness::new(rules);
    h.engine.set_device(Some(&internal));
    assert_eq!(h.tap(LeftOption).take(), tapped(&[LeftOption]));
    h.engine.set_device(Some(&external));
    assert_eq!(h.tap(LeftOption).take(), tapped(&[LeftCommand]));
    h.engine.set_device(None);
    assert_eq!(h.tap(LeftOption).take(), tapped(&[LeftOption]));

    let mut bad = rule(bare(A), RuleKind::Action(Action::Disable));
    bad.when.device = vec!["nope".into()];
    let e = Engine::new(&Rules {
        rules: vec![bad],
        ..Rules::default()
    })
    .err()
    .unwrap();
    assert_eq!(e.kind, RuleErrorKind::UnknownDevice("nope".into()));
}

#[test]
fn device_remaps_and_exclusive_devices() {
    let spec = |name: &str, exclusive| DeviceSpec {
        name: name.into(),
        product: Some(name.into()),
        exclusive,
        ..DeviceSpec::default()
    };
    let remap = |from, to, device| Remap {
        from,
        to: RemapTarget::Key(to),
        device,
    };
    let rules = Rules {
        remaps: vec![
            remap(A, B, None),
            remap(C, D, None),
            remap(A, X, Some(0)),
            remap(A, Y, Some(1)),
        ],
        devices: vec![spec("niz", false), spec("hhkb", true)],
        ..Rules::default()
    };
    let mut h = Harness::new(rules);
    h.engine.assume_device(None);
    assert_eq!(
        h.tap(A).tap(C).take(),
        tapped(&[B, D]),
        "Other keyboards use the general remaps"
    );
    h.engine.assume_device(Some("niz"));
    assert_eq!(
        h.tap(A).tap(C).take(),
        tapped(&[X, D]),
        "The dedicated remaps take priority and the rest fall back to the general ones"
    );
    h.engine.assume_device(Some("hhkb"));
    assert_eq!(
        h.tap(A).tap(C).take(),
        tapped(&[Y, C]),
        "Uses only its own remaps, not the general ones"
    );
}

#[test]
fn rule_scope_must_also_match() {
    let mut r = rule(bare(A), RuleKind::Action(keys(&[Chord::key(B)])));
    r.scope.app = vec!["com.apple.Terminal".into()];
    let mut h = Harness::new(Rules {
        rules: vec![r],
        ..Rules::default()
    });
    assert_eq!(h.tap(A).take(), tapped(&[A]));
    h.engine.set_app(Some("com.apple.Terminal"));
    assert_eq!(h.tap(A).take(), tapped(&[B]));
}
