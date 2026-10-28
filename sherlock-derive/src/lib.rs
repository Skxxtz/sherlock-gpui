use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod finalize;
mod hideable;
mod settings;
mod utils;

#[proc_macro_derive(Hideable, attributes(hide_id))]
pub fn derive_hideable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    hideable::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Finalize)]
pub fn finalize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    finalize::expand(&input).into()
}

/// Settings-page rows from a config struct.
#[proc_macro_derive(Settings, attributes(settings, setting))]
pub fn derive_settings(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    settings::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
