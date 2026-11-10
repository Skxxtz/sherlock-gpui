use super::*;

pub(super) fn table_el(head: &[Rich], rows: &[Vec<Rich>], ui: &mut Ui) -> AnyElement {
    let theme = ui.theme;
    let cols = head
        .len()
        .max(rows.iter().map(Vec::len).max().unwrap_or(0))
        .max(1);
    let weights: Vec<f32> = (0..cols)
        .map(|i| {
            std::iter::once(head)
                .chain(rows.iter().map(Vec::as_slice))
                .filter_map(|row| row.get(i))
                .map(|c| c.text.chars().count())
                .max()
                .unwrap_or(0)
                .clamp(4, 40) as f32
        })
        .collect();

    let inner = px((theme.metrics.radius - 1.).max(0.));
    let empty = Rich::default();

    let row_el = |row: &[Rich], header: bool, ui: &mut Ui| {
        let mut el = div()
            .w_full()
            .flex_none()
            .flex()
            .gap(SPACE_3XL)
            .px(SPACE_3XL)
            .py(px(if header { 8. } else { 10. }));
        for (i, weight) in weights.iter().enumerate() {
            let rich = row.get(i).unwrap_or(&empty).clone();
            let mut cell = div().flex_basis(px(0.)).min_w_0();
            cell.style().flex_grow = Some(*weight);
            cell = cell.when(i == 0 && !header, |el| el.text_color(theme.primary_text));
            el = el.child(cell.child(selectable(rich, ui)));
        }
        el
    };

    let mut table = div()
        .w_full()
        .flex()
        .flex_col()
        .rounded(px(theme.metrics.radius))
        .border_1()
        .border_color(theme.border);
    table = table.child(
        row_el(head, true, ui)
            .rounded_t(inner)
            .bg(theme.bg_code)
            .border_b_1()
            .border_color(theme.border)
            .text_size(px(theme.typography.font_size_sm))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.tertiary_text),
    );
    for (r, row) in rows.iter().enumerate() {
        table = table.child(
            row_el(row, false, ui).when(r > 0, |el| el.border_t_1().border_color(theme.border)),
        );
    }
    table.into_any_element()
}
