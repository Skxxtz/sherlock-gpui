use syn::Type;

/// True if the type is written as `Option<...>` (also `std::option::Option<...>`).
pub fn is_option(ty: &Type) -> bool {
    matches!(ty, Type::Path(p) if p.qself.is_none()
        && p.path.segments.last().is_some_and(|s| s.ident == "Option"))
}
