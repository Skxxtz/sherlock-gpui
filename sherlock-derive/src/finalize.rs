use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

/// `impl Finalize for T {}`, using the trait's default (value unchanged).
pub fn expand(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics crate::ui::runtime_arguments::utils::Finalize
            for #name #ty_generics #where_clause {}
    }
}
