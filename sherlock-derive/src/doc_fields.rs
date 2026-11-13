use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, Lit, Meta};

/// Emits `pub const DOC_FIELDS: &[DocField]` with every named field in declaration
/// order: its name, its type as written, and its `///` doc comment.
pub fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            name,
            "DocFields only supports structs",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(
            name,
            "DocFields needs named fields",
        ));
    };

    let entries = fields.named.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap().to_string();
        let ty = &f.ty;
        let ty = quote!(#ty).to_string();
        let doc = f
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("doc"))
            .filter_map(|a| match &a.meta {
                Meta::NameValue(nv) => match &nv.value {
                    Expr::Lit(l) => match &l.lit {
                        Lit::Str(s) => Some(s.value().trim().to_string()),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        quote! { (#ident, #ty, #doc) }
    });

    let (impl_g, ty_g, where_c) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_g #name #ty_g #where_c {
            /// `(name, type, doc comment)` per field, in declaration order.
            pub const DOC_FIELDS: &'static [(&'static str, &'static str, &'static str)] =
                &[#(#entries),*];
        }
    })
}
