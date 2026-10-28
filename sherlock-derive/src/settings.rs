//! `#[derive(Settings)]`: settings-page rows from a config struct.
//!
//! ```ignore
//! #[derive(Settings)]
//! #[settings(path = "appearance", group = "window")]
//! pub struct ConfigAppearance {
//!     /// Window width in pixels.
//!     #[setting(label = "Width", range(300, 3000, 10))]
//!     pub width: i32,
//!
//!     pub icon_paths: Vec<PathBuf>, // no `#[setting]`: not shown
//! }
//! ```
//!
//! generates `ConfigAppearance::setting_defs()`, one `SettingDef` per `#[setting]` field:
//! - id `"appearance.width"` (`path` + field name, also the key in the config file);
//! - the field's `///` doc comment as the description (required);
//! - label from `label = "…"` (default: the field name, words capitalized);
//! - group from the field's `group = "…"`, else the struct's;
//! - `range(min, max[, step])` / `choices("a", "b")` for numbers / fixed strings;
//! - getter and setter as plain `fn`s on `SherlockConfig` through `ConfigValue`, so the field's
//!   type picks the control and an unsupported type doesn't compile.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute,
    Data,
    DeriveInput,
    Error,
    Expr,
    ExprLit,
    Fields,
    Ident,
    Lit,
    LitInt,
    LitStr,
    Meta,
    Result,
    Token,
    parenthesized,
    punctuated::Punctuated,
};

/// Where the generated code finds the settings types.
fn schema() -> TokenStream {
    quote!(crate::ui::views::settings::schema)
}

pub fn expand(input: &DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let (path, struct_group) = struct_attrs(&input.attrs, name)?;

    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            name,
            "Settings can only be derived for structs",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new_spanned(name, "Settings needs named fields"));
    };

    let schema = schema();
    let mut defs = Vec::new();
    for field in &fields.named {
        let Some(attr) = field.attrs.iter().find(|a| a.path().is_ident("setting")) else {
            continue; // not shown
        };
        let ident = field.ident.as_ref().expect("named field");
        let opts = FieldOpts::parse(attr)?;

        let description = doc(&field.attrs).ok_or_else(|| {
            Error::new_spanned(
                ident,
                "a #[setting] field needs a `///` comment: it's the description",
            )
        })?;
        let group = opts.group.or_else(|| struct_group.clone()).ok_or_else(|| {
            Error::new_spanned(
                ident,
                "no group: set `group = \"…\"` here or in the struct's #[settings(…)]",
            )
        })?;
        let label = opts.label.unwrap_or_else(|| label_from(&ident.to_string()));
        let id = format!("{}.{}", path.join("."), ident);
        let meta = opts.meta.unwrap_or_else(|| quote!(#schema::Meta::None));
        let segments: Vec<Ident> = path.iter().map(|p| format_ident!("{}", p)).collect();

        defs.push(quote! {
            #schema::SettingDef {
                id: #id,
                group: #group,
                label: #label,
                description: #description,
                meta: #meta,
                get: |c, meta| #schema::ConfigValue::to_setting(&c.#(#segments).*.#ident, meta),
                set: |c, value| #schema::ConfigValue::apply(&mut c.#(#segments).*.#ident, value),
            }
        });
    }

    Ok(quote! {
        impl #name {
            /// The settings-page rows for this part of the config (`#[derive(Settings)]`).
            pub(crate) fn setting_defs() -> ::std::vec::Vec<#schema::SettingDef> {
                ::std::vec![#(#defs),*]
            }
        }
    })
}

/// `#[settings(path = "default_apps", group = "defaults")]`
fn struct_attrs(attrs: &[Attribute], name: &Ident) -> Result<(Vec<String>, Option<String>)> {
    let mut path = None;
    let mut group = None;
    for attr in attrs.iter().filter(|a| a.path().is_ident("settings")) {
        attr.parse_nested_meta(|m| {
            if m.path.is_ident("path") {
                let lit: LitStr = m.value()?.parse()?;
                path = Some(
                    lit.value()
                        .split('.')
                        .map(str::to_string)
                        .collect::<Vec<_>>(),
                );
            } else if m.path.is_ident("group") {
                group = Some(m.value()?.parse::<LitStr>()?.value());
            } else {
                return Err(m.error("expected `path` or `group`"));
            }
            Ok(())
        })?;
    }
    let path = path.ok_or_else(|| {
        Error::new_spanned(
            name,
            "missing #[settings(path = \"…\")]: where this struct is in the config, e.g. \"appearance\"",
        )
    })?;
    Ok((path, group))
}

#[derive(Default)]
struct FieldOpts {
    label: Option<String>,
    group: Option<String>,
    meta: Option<TokenStream>,
}

impl FieldOpts {
    /// `#[setting]`, `#[setting(label = "…", group = "…", range(0, 9), choices("a", "b"))]`
    fn parse(attr: &Attribute) -> Result<Self> {
        let mut opts = Self::default();
        if matches!(attr.meta, Meta::Path(_)) {
            return Ok(opts);
        }
        let schema = schema();
        attr.parse_nested_meta(|m| {
            if m.path.is_ident("label") {
                opts.label = Some(m.value()?.parse::<LitStr>()?.value());
            } else if m.path.is_ident("group") {
                opts.group = Some(m.value()?.parse::<LitStr>()?.value());
            } else if m.path.is_ident("range") {
                let content;
                parenthesized!(content in m.input);
                let nums: Punctuated<LitInt, Token![,]> =
                    content.parse_terminated(|p| p.parse::<LitInt>(), Token![,])?;
                let nums: Vec<i64> = nums.iter().map(|n| n.base10_parse()).collect::<Result<_>>()?;
                let (min, max, step) = match nums[..] {
                    [min, max] => (min, max, 1),
                    [min, max, step] => (min, max, step),
                    _ => return Err(m.error("expected `range(min, max)` or `range(min, max, step)`")),
                };
                opts.meta = Some(quote!(#schema::Meta::range_step(#min, #max, #step)));
            } else if m.path.is_ident("choices") {
                let content;
                parenthesized!(content in m.input);
                let options: Punctuated<LitStr, Token![,]> =
                    content.parse_terminated(|p| p.parse::<LitStr>(), Token![,])?;
                let options = options.iter();
                opts.meta = Some(quote!(#schema::Meta::choices(&[#(#options),*])));
            } else {
                return Err(m.error("expected `label`, `group`, `range(…)` or `choices(…)`"));
            }
            Ok(())
        })?;
        Ok(opts)
    }
}

/// The `///` lines, joined with spaces.
fn doc(attrs: &[Attribute]) -> Option<String> {
    let lines: Vec<String> = attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| match &a.meta {
            Meta::NameValue(nv) => match &nv.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => Some(s.value().trim().to_string()),
                _ => None,
            },
            _ => None,
        })
        .filter(|l| !l.is_empty())
        .collect();
    (!lines.is_empty()).then(|| lines.join(" "))
}

/// `use_lr_nav` → `"Use lr nav"`.
fn label_from(field: &str) -> String {
    let words = field.replace('_', " ");
    let mut chars = words.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
