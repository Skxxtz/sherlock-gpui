use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod hideable;
mod utils;

#[proc_macro_derive(Hideable, attributes(hide_id))]
pub fn derive_hideable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    hideable::expand(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
