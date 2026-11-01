//! `#[derive(Extract)]` for `pluck`.
//!
//! ```ignore
//! #[derive(Extract)]
//! struct Hour {
//!     #[at("time", with = parse_time)]
//!     time: NaiveTime,
//!     #[at("weatherDesc", trim)]
//!     desc: String,
//!     #[at("moonrise", none_if = "No moonrise")]
//!     moonrise: Option<String>,
//!     #[at("uvIndex", default)]
//!     uv: u8,
//!     // no #[at]: the field name is the key
//!     humidity: u8,
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    Data,
    DeriveInput,
    Expr,
    ExprPath,
    Field,
    Fields,
    GenericArgument,
    Ident,
    LitStr,
    PathArguments,
    Token,
    Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
    parse_quote,
    spanned::Spanned,
};

// Same parser as the runtime, so both always agree on the syntax.
#[path = "../../src/syntax.rs"]
mod syntax;
use syntax::RawSegment;

#[proc_macro_derive(Extract, attributes(at))]
pub fn derive_extract(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(mut input: DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "Extract can only be derived for structs",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(
            data.fields.span(),
            "Extract needs named fields, each with an #[at(\"path\")]",
        ));
    };

    let inits = fields
        .named
        .iter()
        .map(field_init)
        .collect::<syn::Result<Vec<_>>>()?;

    // Every type parameter must itself be extractable.
    for param in input.generics.type_params_mut() {
        param.bounds.push(parse_quote!(::pluck::Extract));
    }
    let name = &input.ident;
    let (impl_g, ty_g, where_c) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_g ::pluck::Extract for #name #ty_g #where_c {
            fn extract(
                value: &::pluck::Value,
                at: &str,
            ) -> ::core::result::Result<Self, ::pluck::Error> {
                ::core::result::Result::Ok(Self { #(#inits),* })
            }
        }
    })
}

/// Parsed `#[at(...)]`: `#[at("path", trim, default, none_if = "x", with = f)]`.
/// Every part is optional; without a path the field name is used.
#[derive(Default)]
struct AtArgs {
    path: Option<LitStr>,
    trim: bool,
    /// `Some(None)` = `default`, `Some(Some(e))` = `default = e`
    default: Option<Option<Expr>>,
    none_if: Vec<LitStr>,
    with: Option<ExprPath>,
}

impl Parse for AtArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = AtArgs::default();
        if input.peek(LitStr) {
            args.path = Some(input.parse()?);
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        while !input.is_empty() {
            // `default` is a keyword, so parse any ident-like token.
            let key: Ident = if input.peek(Token![default]) {
                let kw: Token![default] = input.parse()?;
                Ident::new("default", kw.span)
            } else {
                input.parse()?
            };
            match key.to_string().as_str() {
                "trim" => args.trim = true,
                "default" => {
                    args.default = Some(if input.peek(Token![=]) {
                        input.parse::<Token![=]>()?;
                        Some(input.parse()?)
                    } else {
                        None
                    })
                }
                "none_if" => {
                    input.parse::<Token![=]>()?;
                    args.none_if.push(input.parse()?);
                }
                "with" => {
                    input.parse::<Token![=]>()?;
                    args.with = Some(input.parse()?);
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown option `{other}` (expected trim, default, none_if, with)"),
                    ));
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(args)
    }
}

fn at_args(field: &Field) -> syn::Result<AtArgs> {
    let mut found = None;
    for attr in field.attrs.iter().filter(|a| a.path().is_ident("at")) {
        if found.is_some() {
            return Err(syn::Error::new(attr.span(), "duplicate #[at]"));
        }
        found = Some(attr.parse_args()?);
    }
    Ok(found.unwrap_or_default())
}

/// How a `with = f` result maps onto the field type.
enum Shape {
    One,
    Opt,
    Many,
}

/// Syntactic check of the outer type. Type aliases of `Option`/`Vec` are
/// treated as plain values, which is fine unless used together with `with`.
fn shape(ty: &Type) -> Shape {
    let Type::Path(p) = ty else { return Shape::One };
    let Some(last) = p.path.segments.last() else {
        return Shape::One;
    };
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Shape::One;
    };
    if args.args.len() != 1 || !matches!(args.args[0], GenericArgument::Type(_)) {
        return Shape::One;
    }
    match last.ident.to_string().as_str() {
        "Option" => Shape::Opt,
        "Vec" => Shape::Many,
        _ => Shape::One,
    }
}

fn field_init(field: &Field) -> syn::Result<TokenStream2> {
    let ident = field.ident.as_ref().expect("named field");
    let args = at_args(field)?;

    let (raw, span) = match &args.path {
        Some(l) => (l.value(), l.span()),
        None => (
            ident.to_string().trim_start_matches("r#").to_string(),
            ident.span(),
        ),
    };
    let path = const_path(&raw, span)?;
    let trim = args.trim;
    let none_if = &args.none_if;

    let call = match &args.with {
        None => quote!(::pluck::field_opts(value, at, &PATH, &OPTS)),
        Some(f) => {
            let func = match shape(&field.ty) {
                Shape::One => quote!(one),
                Shape::Opt => quote!(opt),
                Shape::Many => quote!(many),
            };
            quote!(::pluck::with::#func(value, at, &PATH, &OPTS, #f))
        }
    };

    let result = match &args.default {
        None => quote!(#call?),
        Some(expr) => {
            let fallback = match expr {
                Some(e) => quote!(#e),
                None => quote!(::core::default::Default::default()),
            };
            quote! {
                match #call {
                    ::core::result::Result::Ok(v) => v,
                    ::core::result::Result::Err(::pluck::Error {
                        kind: ::pluck::ErrorKind::Missing, ..
                    }) => #fallback,
                    ::core::result::Result::Err(e) => return ::core::result::Result::Err(e),
                }
            }
        }
    };

    Ok(quote! {
        #ident: {
            const PATH: ::pluck::Path = #path;
            const OPTS: ::pluck::Opts = ::pluck::Opts {
                trim: #trim,
                none_if: &[#(#none_if),*],
            };
            #result
        }
    })
}

/// Path string → `const` Path expression, validated at compile time.
fn const_path(raw: &str, span: Span) -> syn::Result<TokenStream2> {
    let segments = syntax::parse(raw)
        .map_err(|why| syn::Error::new(span, format!("invalid path {raw:?}: {why}")))?
        .into_iter()
        .map(|s| match s {
            RawSegment::Key(k) => {
                quote!(::pluck::Segment::Key(::std::borrow::Cow::Borrowed(#k)))
            }
            RawSegment::Index(i) => quote!(::pluck::Segment::Index(#i)),
            RawSegment::Each => quote!(::pluck::Segment::Each),
        });
    Ok(quote!(::pluck::Path::from_static(&[#(#segments),*])))
}
