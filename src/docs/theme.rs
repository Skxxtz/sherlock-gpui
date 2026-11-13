use md_rs::{
    components::{
        ParentComponentExt,
        code_block::codeblock,
        container::Container,
        heading::{h1, h2},
        span::code,
        table::{Table, table},
    },
    md,
    p,
};

use crate::{
    app::theme::{Metrics, ThemeColors, ThemeData, Typography},
    docs::Documentation,
};

type Fields = &'static [(&'static str, &'static str, &'static str)];

/// Every documented group: heading, intro and its fields.
const GROUPS: [(&str, &str, Fields); 3] = [
    (
        "Colors",
        "Hex colors: `#rgb`, `#rrggbb` or `#rrggbbaa`.",
        ThemeColors::DOC_FIELDS,
    ),
    (
        "Typography",
        "Font names and sizes in px.",
        Typography::DOC_FIELDS,
    ),
    ("Metrics", "Sizes in px.", Metrics::DOC_FIELDS),
];

fn type_name(rust: &str) -> &'static str {
    match rust {
        "Hsla" => "color",
        "f32" => "number",
        "SharedString" => "string",
        _ => "unknown",
    }
}

fn group_table(fields: Fields, defaults: &toml::Table) -> Table {
    table()
        .headers(["Key", "Type", "Default", "Description"])
        .rows(fields.iter().map(|(name, ty, doc)| {
            let default = match defaults.get(*name) {
                Some(toml::Value::String(s)) => format!("`\"{s}\"`"),
                Some(v) => format!("`{v}`"),
                None => "—".into(),
            };
            [
                format!("`{name}`"),
                type_name(ty).into(),
                default,
                (*doc).into(),
            ]
        }))
}

impl Documentation for ThemeData {
    type Docs = Container;
    fn docs() -> Self::Docs {
        let defaults = toml::Table::try_from(ThemeData::dark()).unwrap_or_default();

        let mut out = md!(
            h1("Themes"),
            p!(
                "Themes are TOML files in",
                code("~/.config/sherlock/themes/"),
                ". A theme only has to set the keys it changes: everything else \
                comes from its base. Set the base with a top-level",
                code("extends"),
                ", naming a built-in theme or another theme file in the same \
                directory (without",
                code(".toml"),
                "). Without",
                code("extends"),
                ", the default theme is the base."
            ),
            p!(
                "Built-in themes:",
                ThemeData::builtins()
                    .map(|(name, _)| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            codeblock().lang("toml").content(
                "extends = \"nord\"\n\naccent = \"#ebcb8b\"\nfont_family = \"Inter\"\nradius = 12.0\n"
            ),
        );
        for (title, intro, fields) in GROUPS {
            out = out
                .child(h2(title))
                .child(p!(intro))
                .child(group_table(fields, &defaults));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_key_is_documented() {
        let defaults = toml::Table::try_from(ThemeData::dark()).unwrap();
        let documented: Vec<&str> = GROUPS
            .iter()
            .flat_map(|(_, _, f)| f.iter().map(|(n, _, _)| *n))
            .collect();
        for key in defaults.keys() {
            assert!(
                documented.contains(&key.as_str()),
                "`{key}` missing from theme docs"
            );
        }
        for (_, _, fields) in GROUPS {
            for (name, ty, doc) in fields {
                assert!(
                    !doc.is_empty(),
                    "theme key `{name}` has no `///` doc comment"
                );
                assert_ne!(type_name(ty), "unknown", "add `{ty}` to `type_name`");
            }
        }
    }
}
