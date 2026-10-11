use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type, parse_macro_input};

/// True if the type is written as `Option<...>` (also `std::option::Option<...>`).
fn is_option(ty: &Type) -> bool {
    matches!(ty, Type::Path(p) if p.qself.is_none()
        && p.path.segments.last().is_some_and(|s| s.ident == "Option"))
}

#[proc_macro_derive(Hideable, attributes(hide_id))]
pub fn derive_hideable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let field = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => f
                .named
                .iter()
                .find(|f| f.attrs.iter().any(|a| a.path().is_ident("hide_id"))),
            _ => None,
        },
        _ => None,
    };

    let body = match field {
        Some(f) => {
            let ident = f.ident.as_ref().unwrap(); // named fields always have one
            let (id_expr, check_expr) = if is_option(&f.ty) {
                (
                    quote! { self.#ident.as_ref().map(::std::string::ToString::to_string) },
                    quote! { self.#ident.as_ref().is_some_and(|id| !id.is_empty()) },
                )
            } else {
                (
                    quote! { ::std::option::Option::Some(::std::string::ToString::to_string(&self.#ident)) },
                    quote! { true },
                )
            };
            quote! {
                fn hide_id(&self) -> ::std::option::Option<::std::string::String> { #id_expr }
                fn is_hideable(&self) -> bool { #check_expr }
            }
        }
        None => quote! {},
    };

    quote! {
        impl crate::ui::launcher::context_menu::quick_actions::Hideable for #name { #body }
    }
    .into()
}
