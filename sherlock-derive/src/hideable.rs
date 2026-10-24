use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::utils::is_option;

pub fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
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
                    quote! {
                        self.#ident
                            .as_ref()
                            .map(::std::string::ToString::to_string)
                            .filter(|id| !id.is_empty())
                    },
                    quote! { self.#ident.as_ref().is_some_and(|id| !id.is_empty()) },
                )
            } else {
                (
                    quote! {
                        ::std::option::Option::Some(::std::string::ToString::to_string(&self.#ident))
                            .filter(|id| !id.is_empty())
                    },
                    quote! { !self.#ident.is_empty() },
                )
            };
            quote! {
                fn hide_id(&self) -> ::std::option::Option<::std::string::String> { #id_expr }
                fn is_hideable(&self) -> bool { #check_expr }
            }
        }
        None => quote! {},
    };

    Ok(quote! {
        impl crate::ui::launcher::context_menu::quick_actions::Hideable for #name { #body }
    })
}
