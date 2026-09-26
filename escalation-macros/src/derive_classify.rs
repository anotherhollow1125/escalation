//! derive マクロ `Classify` の実装

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{DeriveInput, Expr, GenericParam, Ident, Token, parse_quote};

/// `#[classify(Unclassified => 式)]` の中身
struct UnclassifiedRule {
    value: Expr,
}

impl Parse for UnclassifiedRule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = input.parse::<Ident>()?;
        if key != "Unclassified" {
            return Err(syn::Error::new(
                key.span(),
                "expected `Unclassified => expr`",
            ));
        }

        input.parse::<Token![=>]>()?;
        let value = input.parse()?;

        Ok(UnclassifiedRule { value })
    }
}

pub(crate) fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let DeriveInput {
        attrs,
        ident,
        generics,
        ..
    } = &input;

    let mut unclassified: Option<UnclassifiedRule> = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("classify")) {
        let rule = attr.parse_args::<UnclassifiedRule>()?;

        if unclassified.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                "duplicate `#[classify(Unclassified => ...)]` attribute",
            ));
        }

        unclassified = Some(rule);
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let self_impl = quote! {
        impl #impl_generics ::escalation::Classify<#ident #ty_generics> for #ident #ty_generics
        #where_clause
        {
            fn classify(error: #ident #ty_generics) -> Self {
                error
            }
        }

        impl #impl_generics ::escalation::Decompose<#ident #ty_generics> for #ident #ty_generics
        #where_clause
        {}
    };

    let Some(UnclassifiedRule { value }) = unclassified else {
        return Ok(self_impl);
    };

    // 導出対象の型のジェネリクスと衝突しない型パラメータを追加する
    let error_param = format_ident!("__EscalationUnclassifiedError");
    let mut unclassified_generics = generics.clone();
    unclassified_generics
        .params
        .push(GenericParam::Type(parse_quote!(#error_param)));
    let (unclassified_impl_generics, _, _) = unclassified_generics.split_for_impl();

    let mut decompose_generics = unclassified_generics.clone();
    decompose_generics
        .make_where_clause()
        .predicates
        .push(parse_quote!(
            #error_param: ::std::fmt::Display + ::std::fmt::Debug
        ));
    let (decompose_impl_generics, _, decompose_where_clause) = decompose_generics.split_for_impl();

    Ok(quote! {
        #self_impl

        impl #unclassified_impl_generics
            ::escalation::Classify<::escalation::Unclassified<#error_param>>
            for #ident #ty_generics
        #where_clause
        {
            fn classify(_error: ::escalation::Unclassified<#error_param>) -> Self {
                #value
            }
        }

        impl #decompose_impl_generics
            ::escalation::Decompose<::escalation::Unclassified<#error_param>>
            for #ident #ty_generics
        #decompose_where_clause
        {}
    })
}
