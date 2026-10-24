use criterion::{Criterion, black_box};

use super::*;
use crate::app::theme::ThemeData;

#[test]
fn bench_parse() {
    let mut c = Criterion::default();
    let theme = std::sync::Arc::new(ThemeData::default());
    let input = r#"hello <b>world</b> <span font_desc='Courier'>mono</span> <br/> end"#;
    c.bench_function("parse_pango", |b| {
        b.iter(|| parse_pango(black_box(input), &theme))
    });
}
