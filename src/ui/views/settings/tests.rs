use super::model::apply_edit;
use super::*;
use gpui::{Hsla, rgb};

#[test]
fn toggle_flips() {
    let mut v = SettingValue::Toggle(false);
    v.apply(Edit::Toggle);
    assert!(matches!(v, SettingValue::Toggle(true)));
}

#[test]
fn choice_is_clamped() {
    let mut v = SettingValue::Choice {
        options: &["a", "b"],
        selected: 0,
    };
    v.apply(Edit::SetChoice(5));
    assert!(matches!(v, SettingValue::Choice { selected: 1, .. }));
}

#[test]
fn number_is_clamped() {
    let mut v = SettingValue::Number {
        value: 3,
        min: 1,
        max: 3,
        step: 1,
    };
    v.apply(Edit::Step(1));
    assert!(matches!(v, SettingValue::Number { value: 3, .. }));
    v.apply(Edit::Step(-5));
    assert!(matches!(v, SettingValue::Number { value: 1, .. }));
}

#[test]
fn activate_steps_and_wraps_numbers() {
    let mut v = SettingValue::Number {
        value: 2,
        min: 1,
        max: 3,
        step: 1,
    };
    for expected in [3, 1, 2] {
        v.apply(v.activate().unwrap());
        assert!(matches!(v, SettingValue::Number { value, .. } if value == expected));
    }
}

#[test]
fn activate_skips_values_with_own_editor() {
    assert!(SettingValue::Keybind(vec![]).activate().is_none());
    assert!(SettingValue::Color(rgb(0).into()).activate().is_none());
    let choice = SettingValue::Choice {
        options: &["a"],
        selected: 0,
    };
    assert!(choice.activate().is_none());
}

#[test]
fn set_color_applies_to_colors_only() {
    let red: Hsla = rgb(0xff0000).into();
    let mut v = SettingValue::Color(rgb(0).into());
    v.apply(Edit::SetColor(red));
    assert_eq!(v, SettingValue::Color(red));
    let mut t = SettingValue::Toggle(true);
    t.apply(Edit::SetColor(red));
    assert_eq!(t, SettingValue::Toggle(true));
}

#[test]
fn shared_chords_are_reported() {
    let mut view = SettingsView::new();
    view.chords_mut(2, 0).unwrap().push("ctrl-p".into()); // Previous item has it too
    assert_eq!(view.conflicts((2, 0), "ctrl-p"), ["Previous item"]);
    assert!(view.conflict_note(2, 0).unwrap().contains("Previous item"));
}

#[test]
fn mismatched_edit_is_ignored() {
    let mut v = SettingValue::Toggle(true);
    v.apply(Edit::Step(1));
    assert!(matches!(v, SettingValue::Toggle(true)));
}

#[test]
fn edit_targets_one_row() {
    let mut sections = data::dummy_sections();
    let before = sections[0].settings[1].value.clone();
    apply_edit(&mut sections, 0, 0, Edit::Toggle);
    assert!(matches!(
        sections[0].settings[0].value,
        SettingValue::Toggle(false)
    ));
    assert_eq!(before, sections[0].settings[1].value);
}

#[test]
fn search_spans_sections() {
    let mut view = SettingsView::new();
    view.query = "item".into();
    let sections: Vec<_> = view.visible().iter().map(|&(si, _)| si).collect();
    // "Close after run" (General) and three keybinds (Keyboard)
    assert!(sections.contains(&0) && sections.contains(&2));
    view.query = "".into();
    assert_eq!(view.visible().len(), view.sections[0].settings.len());
}

#[test]
fn enter_marks_setting_modified_until_reverted() {
    let mut view = SettingsView::new();
    assert_eq!(view.unsaved(), 0);
    view.activate();
    assert!(view.is_modified(0, 0));
    assert_eq!(view.unsaved(), 1);
    view.activate();
    assert_eq!(view.unsaved(), 0);
}
