use std::sync::Arc;

use gpui::{
    AppContext,
    Entity,
    IntoElement,
    ParentElement,
    Render,
    Styled,
    TestAppContext,
    VisualTestContext,
    div,
};

use super::*;
use crate::{
    app::theme::{ActiveTheme, ThemeData},
    ui::actions::{Execute, ItemDown, Quit},
};

struct Harness {
    select: Entity<Select>,
    changes: Rc<Cell<Option<usize>>>,
    _sub: gpui::Subscription,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.select.clone())
    }
}

fn open_select(cx: &mut TestAppContext) -> (Entity<Harness>, &mut VisualTestContext) {
    cx.update(|cx| cx.set_global(ActiveTheme(Arc::new(ThemeData::default()))));
    let (view, cx) = cx.add_window_view(|_, cx| {
        let select = cx.new(|cx| {
            let options = ["System", "Dark", "Light", "Solarized"].map(SharedString::from);
            Select::new(options.to_vec(), 1, cx)
        });
        let changes = Rc::new(Cell::new(None));
        let seen = changes.clone();
        let _sub = cx.subscribe(&select, move |_, _, SelectEvent::Changed(i), _| {
            seen.set(Some(*i))
        });
        Harness {
            select,
            changes,
            _sub,
        }
    });
    let select = view.read_with(cx, |h, _| h.select.clone());
    cx.update(|window, cx| select.update(cx, |s, cx| s.open(window, cx)));
    cx.run_until_parked();
    (view, cx)
}

#[gpui::test]
fn opens_on_current_and_esc_changes_nothing(cx: &mut TestAppContext) {
    let (view, cx) = open_select(cx);
    let select = view.read_with(cx, |h, _| h.select.clone());
    assert_eq!(select.read_with(cx, |s, _| s.cursor), 1);
    cx.dispatch_action(ItemDown);
    cx.dispatch_action(Quit);
    assert_eq!(
        select.read_with(cx, |s, _| (s.selected, s.open)),
        (1, false)
    );
    assert_eq!(view.read_with(cx, |h, _| h.changes.get()), None);
}

#[gpui::test]
fn arrows_typeahead_and_enter_pick(cx: &mut TestAppContext) {
    let (view, cx) = open_select(cx);
    let select = view.read_with(cx, |h, _| h.select.clone());
    cx.simulate_keystrokes("s");
    assert_eq!(
        select.read_with(cx, |s, _| s.cursor),
        3,
        "next option starting with s"
    );
    cx.simulate_keystrokes("s");
    assert_eq!(select.read_with(cx, |s, _| s.cursor), 0, "wraps around");
    cx.dispatch_action(ItemDown);
    cx.dispatch_action(ItemDown);
    cx.dispatch_action(Execute);
    assert_eq!(view.read_with(cx, |h, _| h.changes.get()), Some(2));
    assert!(!select.read_with(cx, |s, _| s.open));
}
