use std::sync::Arc;

use gpui::{AppContext, Entity, Hsla, TestAppContext, rgb};

use super::{
    SettingsView,
    config_value::{ConfigValue, Meta},
    default_config,
    defs,
    model::{Edit, SettingValue},
};
use crate::{
    app::theme::{ActiveTheme, ThemeData},
    ui::actions::UIFunction,
    utils::config::ConfigKeybinds,
};

fn view(cx: &mut TestAppContext) -> Entity<SettingsView> {
    cx.update(|cx| cx.set_global(ActiveTheme(Arc::new(ThemeData::default()))));
    cx.new(SettingsView::new)
}

#[test]
fn toggle_flips() {
    let mut v = SettingValue::Toggle(false);
    v.apply(Edit::Toggle);
    assert!(matches!(v, SettingValue::Toggle(true)));
}

#[test]
fn choice_and_number_are_clamped() {
    let mut v = SettingValue::Choice {
        options: &["a", "b"],
        selected: 0,
    };
    v.apply(Edit::SetChoice(5));
    assert!(matches!(v, SettingValue::Choice { selected: 1, .. }));

    let mut n = SettingValue::Number {
        value: 3,
        min: 1,
        max: 3,
        step: 1,
    };
    n.apply(Edit::SetNumber(9));
    assert!(matches!(n, SettingValue::Number { value: 3, .. }));
    n.apply(Edit::SetNumber(-5));
    assert!(matches!(n, SettingValue::Number { value: 1, .. }));
}

#[test]
fn only_toggles_change_on_enter() {
    assert!(SettingValue::Toggle(true).activate().is_some());
    assert!(SettingValue::Keybind(vec![]).activate().is_none());
    assert!(SettingValue::Text("x".into()).activate().is_none());
}

#[test]
fn edits_apply_to_their_kind_only() {
    let red: Hsla = rgb(0xff0000).into();
    let mut v = SettingValue::Color(rgb(0).into());
    v.apply(Edit::SetColor(red));
    assert_eq!(v, SettingValue::Color(red));
    let mut t = SettingValue::Toggle(true);
    t.apply(Edit::SetColor(red));
    t.apply(Edit::SetNumber(1));
    assert_eq!(t, SettingValue::Toggle(true));
}

#[test]
fn config_types_map_to_controls_and_back() {
    let mut b = false;
    b.apply(&b.to_setting(Meta::None));
    assert!(matches!(
        true.to_setting(Meta::None),
        SettingValue::Toggle(true)
    ));

    // u8 clamps to the meta range and to its own
    let mut n: u8 = 5;
    assert!(matches!(
        n.to_setting(Meta::range(0, 9)),
        SettingValue::Number {
            value: 5,
            min: 0,
            max: 9,
            ..
        }
    ));
    n.apply(&SettingValue::Number {
        value: 300,
        min: 0,
        max: 999,
        step: 1,
    });
    assert_eq!(n, 255);

    // fractions in percent
    let mut f: f64 = 0.6;
    assert!(matches!(
        f.to_setting(Meta::None),
        SettingValue::Number { value: 60, .. }
    ));
    f.apply(&SettingValue::Number {
        value: 25,
        min: 0,
        max: 100,
        step: 5,
    });
    assert!((f - 0.25).abs() < 1e-9);

    // Option<u8>: the lowest number is "not set"
    let mut clicks: Option<u8> = None;
    let SettingValue::Number { value, .. } = clicks.to_setting(Meta::range(0, 3)) else {
        panic!()
    };
    assert_eq!(value, 0);
    clicks.apply(&SettingValue::Number {
        value: 2,
        min: 0,
        max: 3,
        step: 1,
    });
    assert_eq!(clicks, Some(2));
    clicks.apply(&SettingValue::Number {
        value: 0,
        min: 0,
        max: 3,
        step: 1,
    });
    assert_eq!(clicks, None);

    // strings: choice with options, text otherwise; empty optional text is None
    let mut edge = String::from("bottom");
    let choice = edge.to_setting(Meta::choices(&["top", "bottom"]));
    assert!(matches!(choice, SettingValue::Choice { selected: 1, .. }));
    edge.apply(&SettingValue::Choice {
        options: &["top", "bottom"],
        selected: 0,
    });
    assert_eq!(edge, "top");
    let mut browser: Option<String> = Some("firefox".into());
    browser.apply(&SettingValue::Text("".into()));
    assert_eq!(browser, None);
}

#[test]
fn ids_are_unique() {
    let defs = defs::sections();
    let mut ids: Vec<_> = defs
        .iter()
        .flat_map(|s| &s.settings)
        .map(|d| d.id)
        .collect();
    let n = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), n, "two settings share an id");
}

/// Every id is the key of its field in the config file (catches a wrong `path` or
/// `#[serde(rename)]`, and keybind names that aren't the action's name in the file).
#[test]
fn ids_are_config_keys() {
    let mut config = default_config();
    // keybinds only appear in the file when changed: give every row a key of its own
    for (i, def) in defs::sections()
        .iter()
        .flat_map(|s| &s.settings)
        .enumerate()
    {
        if def.id.starts_with("keybinds.") {
            let chord = format!("ctrl-alt-{i}").into();
            (def.set)(&mut config, &SettingValue::Keybind(vec![chord]));
        }
    }
    let file = toml::Value::try_from(&config).expect("config serializes");
    let actions: Vec<String> = config
        .keybinds
        .0
        .values()
        .map(|f| {
            toml::Value::try_from(f)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();

    for def in defs::sections().iter().flat_map(|s| &s.settings) {
        if def.id.starts_with("keybinds.") {
            // `keybinds.item_down`, `keybinds.variable.complete`: the action is the last part
            let name = def.id.rsplit('.').next().unwrap();
            assert!(
                actions.iter().any(|a| a == name),
                "{} isn't an action name (known: {actions:?})",
                def.id
            );
            continue;
        }
        let last = def.id.rsplit('.').next().unwrap();
        let mut node = &file;
        for key in def.id.split('.') {
            node = match node.get(key) {
                Some(next) => next,
                // `None` options aren't serialized: the key is still valid
                None if key == last => break,
                None => panic!("{} is not a key in the config file", def.id),
            };
        }
    }
}

/// Every descriptor writes what it reads, and reads what it wrote.
#[test]
fn every_setting_round_trips() {
    for def in defs::sections().iter().flat_map(|s| &s.settings) {
        let mut config = default_config();
        let value = (def.get)(&config, def.meta);
        (def.set)(&mut config, &value);
        assert_eq!(
            (def.get)(&config, def.meta),
            value,
            "{} changes on a no-op write",
            def.id
        );

        let changed = match value.clone() {
            SettingValue::Toggle(on) => SettingValue::Toggle(!on),
            SettingValue::Number {
                value,
                min,
                max,
                step,
            } => SettingValue::Number {
                value: if value + step <= max {
                    value + step
                } else {
                    min
                },
                min,
                max,
                step,
            },
            SettingValue::Text(_) => SettingValue::Text("changed".into()),
            SettingValue::Keybind(_) => SettingValue::Keybind(vec!["ctrl-f12".into()]),
            SettingValue::Choice { options, selected } => SettingValue::Choice {
                options,
                selected: (selected + 1) % options.len(),
            },
            SettingValue::Color(c) => SettingValue::Color(c),
        };
        (def.set)(&mut config, &changed);
        assert_eq!(
            (def.get)(&config, def.meta),
            changed,
            "{} doesn't read back its write",
            def.id
        );
    }
}

#[test]
fn a_chord_moves_between_actions() {
    let mut config = default_config();
    let down = keybind_def("keybinds.item_down");
    let up = keybind_def("keybinds.item_up");
    // "add binding": the built-in key stays, ctrl-j is added
    (down.set)(
        &mut config,
        &SettingValue::Keybind(vec!["down".into(), "ctrl-j".into()]),
    );
    (up.set)(
        &mut config,
        &SettingValue::Keybind(vec!["ctrl-j".into(), "up".into()]),
    );

    assert_eq!(config.keybinds.0.get("ctrl-j"), Some(&UIFunction::ItemUp));
    assert_eq!(
        (down.get)(&config, Meta::None),
        SettingValue::Keybind(vec!["down".into()]),
        "ctrl-j moved to up; down keeps its built-in key"
    );
    assert_eq!(
        (up.get)(&config, Meta::None),
        SettingValue::Keybind(vec!["ctrl-j".into(), "up".into()])
    );
}

#[test]
fn removing_the_only_key_leaves_none() {
    let mut config = default_config();
    let down = keybind_def("keybinds.item_down");
    let up = keybind_def("keybinds.item_up");
    // down: only ctrl-j (its built-in `down` removed); then up takes ctrl-j
    (down.set)(&mut config, &SettingValue::Keybind(vec!["ctrl-j".into()]));
    (up.set)(
        &mut config,
        &SettingValue::Keybind(vec!["ctrl-j".into(), "up".into()]),
    );

    assert_eq!(
        (down.get)(&config, Meta::None),
        SettingValue::Keybind(vec![])
    );
    assert_eq!(config.keybinds.0.get("down"), Some(&UIFunction::Unbind));
}

#[gpui::test]
fn stateful_values_get_a_control(cx: &mut TestAppContext) {
    let view = view(cx);
    view.read_with(cx, |v, _| {
        assert!(v.controls.contains_key("default_apps.terminal"));
        assert!(v.controls.contains_key("keybinds.item_down"));
        assert!(!v.controls.contains_key("behavior.animate"));
    });
}

#[gpui::test]
fn edits_reach_the_config_and_mark_modified(cx: &mut TestAppContext) {
    let view = view(cx);
    view.update(cx, |v, cx| {
        let before = v.config.behavior.animate;
        assert!(!v.is_modified("behavior.animate"));
        v.apply("behavior.animate", Edit::Toggle, cx);
        assert_eq!(v.config.behavior.animate, !before);
        assert!(v.is_modified("behavior.animate"));
        v.apply("behavior.animate", Edit::Toggle, cx);
        assert!(!v.is_modified("behavior.animate"));
    });
}

#[gpui::test]
fn a_moved_chord_updates_the_other_row(cx: &mut TestAppContext) {
    let view = view(cx);
    view.update(cx, |v, cx| {
        v.apply(
            "keybinds.item_down",
            Edit::SetChords(vec!["down".into(), "ctrl-j".into()]),
            cx,
        );
        v.apply(
            "keybinds.item_up",
            Edit::SetChords(vec!["ctrl-j".into(), "up".into()]),
            cx,
        );
        assert_eq!(
            v.setting("keybinds.item_down").map(|s| s.value.clone()),
            Some(SettingValue::Keybind(vec!["down".into()])),
            "the other row loses the moved key"
        );
    });
}

#[gpui::test]
fn search_spans_sections(cx: &mut TestAppContext) {
    let view = view(cx);
    view.update(cx, |v, _| {
        v.query = "item".into();
        let sections: Vec<_> = v.visible().iter().map(|&(si, _)| si).collect();
        assert!(sections.len() > 1);
        v.query = "".into();
        assert_eq!(v.visible().len(), v.sections[0].settings.len());
    });
}

/// The keybind row of `id`.
fn keybind_def(id: &str) -> defs::SettingDef {
    defs::sections()
        .into_iter()
        .flat_map(|s| s.settings)
        .find(|d| d.id == id)
        .unwrap_or_else(|| panic!("no row {id}"))
}

#[test]
fn defaults_apply_without_config() {
    let binds = ConfigKeybinds::default().effective();
    assert_eq!(binds.get("down"), Some(&UIFunction::ItemDown));
    assert_eq!(binds.get("variable.tab"), Some(&UIFunction::Complete));

    // a user key adds to the defaults
    let user = ConfigKeybinds([("ctrl-j".to_string(), UIFunction::ItemDown)].into());
    let binds = user.effective();
    assert_eq!(binds.get("ctrl-j"), Some(&UIFunction::ItemDown));
    assert_eq!(
        binds.get("down"),
        Some(&UIFunction::ItemDown),
        "defaults stay"
    );
    assert_eq!(
        binds.get("up"),
        Some(&UIFunction::ItemUp),
        "other actions keep theirs"
    );

    // a key runs one action: another action's default key moves
    // TODO: once the move asks first (popup), test that it doesn't happen without confirming
    let user = ConfigKeybinds([("ctrl-enter".to_string(), UIFunction::Execute)].into());
    let binds = user.effective();
    assert_eq!(binds.get("ctrl-enter"), Some(&UIFunction::Execute));
    assert!(
        !binds.values().any(|&f| f == UIFunction::ExecuteInplace),
        "run in place had only ctrl-enter"
    );

    // `unbind` removes a default
    let user = ConfigKeybinds([("down".to_string(), UIFunction::Unbind)].into());
    assert_eq!(user.effective().get("down"), None);
}

#[test]
fn unconfigured_rows_show_the_built_in_keys() {
    let config = default_config();
    let down = keybind_def("keybinds.item_down");
    assert_eq!(
        (down.get)(&config, Meta::None),
        SettingValue::Keybind(vec!["down".into()])
    );
    let complete = keybind_def("keybinds.variable.complete");
    assert_eq!(
        (complete.get)(&config, Meta::None),
        SettingValue::Keybind(vec!["tab".into()]),
        "context keys show without their prefix"
    );
}

#[test]
fn only_changed_keys_are_written() {
    let mut config = default_config();
    let down = keybind_def("keybinds.item_down");

    // a change is written, and replaces the default
    (down.set)(&mut config, &SettingValue::Keybind(vec!["ctrl-j".into()]));
    assert_eq!(config.keybinds.0.get("ctrl-j"), Some(&UIFunction::ItemDown));
    assert_eq!(
        (down.get)(&config, Meta::None),
        SettingValue::Keybind(vec!["ctrl-j".into()])
    );

    // back to the built-in keys: the entry goes, nothing default is written
    (down.set)(&mut config, &SettingValue::Keybind(vec!["down".into()]));
    assert!(config.keybinds.0.is_empty(), "{:?}", config.keybinds.0);
    assert_eq!(
        (down.get)(&config, Meta::None),
        SettingValue::Keybind(vec!["down".into()])
    );
}
