//! derive マクロ `Classify` の実装
//!
//! - 自分自身からの `Classify` (常に生成)
//! - 型の外側の `#[classify(変換元 as 式)]` ([`item_attr`])
//! - フィールドの `#[classify]` (thiserror の `#[from]` と同様)
//! - バリアントの `#[typ(..)]` / `#[variant(..)]` ([`variant_attr`])

mod item_attr;
mod variant_attr;

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, GenericParam, Generics, Ident, Meta, Type,
    parse_quote,
};

use self::item_attr::ItemRule;
use self::variant_attr::{VariantArms, VariantAttrs};

/// 生成する impl の対象 (導出対象の型)
struct Target<'a> {
    ident: &'a Ident,
    generics: &'a Generics,
}

impl Target<'_> {
    /// `impl Classify<source> for Self` と `impl Decompose<source> for Self` を生成する
    fn classify_impl(
        &self,
        source: &Type,
        param: TokenStream2,
        body: TokenStream2,
    ) -> TokenStream2 {
        let ident = self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();

        quote! {
            impl #impl_generics ::escalation::Classify<#source> for #ident #ty_generics
            #where_clause
            {
                fn classify(#param: #source) -> Self {
                    #body
                }
            }

            impl #impl_generics ::escalation::Decompose<#source> for #ident #ty_generics
            #where_clause
            {}
        }
    }

    /// 任意の `E` についての `Classify<Unclassified<E>>` / `Decompose<Unclassified<E>>` を生成する
    fn unclassified_impl(&self, value: &Expr) -> TokenStream2 {
        let ident = self.ident;
        let (_, ty_generics, where_clause) = self.generics.split_for_impl();

        // 導出対象の型のジェネリクスと衝突しない型パラメータを追加する
        let error_param = format_ident!("__EscalationUnclassifiedError");
        let mut unclassified_generics = self.generics.clone();
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
        let (decompose_impl_generics, _, decompose_where_clause) =
            decompose_generics.split_for_impl();

        quote! {
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
        }
    }
}

pub(crate) fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let target = Target {
        ident: &input.ident,
        generics: &input.generics,
    };

    // 自分自身からの Classify
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let ident = &input.ident;
    let self_ty: Type = parse_quote!(#ident #ty_generics);
    let mut output = target.classify_impl(&self_ty, quote!(error), quote!(error));

    output.extend(expand_item_attrs(&target, &input.attrs)?);

    match &input.data {
        Data::Struct(data) => {
            reject_variant_attrs(&input.attrs)?;
            if let Some((source, construct)) = classify_field(&data.fields, quote!(Self))? {
                output.extend(target.classify_impl(&source, quote!(error), construct));
            }
        }
        Data::Enum(data) => {
            reject_variant_attrs(&input.attrs)?;
            let mut arms = VariantArms::default();

            for variant in &data.variants {
                let variant_ident = &variant.ident;
                let VariantAttrs { types, patterns } = VariantAttrs::parse(&variant.attrs)?;

                if (!types.is_empty() || !patterns.is_empty())
                    && !matches!(variant.fields, Fields::Unit)
                {
                    return Err(syn::Error::new_spanned(
                        &variant.fields,
                        "`typ` / `variant` can only be used on unit variants",
                    ));
                }

                for ty in &types {
                    output.extend(target.classify_impl(
                        ty,
                        quote!(_error),
                        quote!(Self::#variant_ident),
                    ));
                }
                for pattern in patterns {
                    arms.push(pattern, variant_ident)?;
                }

                if let Some((source, construct)) =
                    classify_field(&variant.fields, quote!(Self::#variant_ident))?
                {
                    output.extend(target.classify_impl(&source, quote!(error), construct));
                }
            }

            for (source, match_arms) in arms.finish() {
                output.extend(target.classify_impl(
                    &source,
                    quote!(error),
                    quote! {
                        match error {
                            #(#match_arms)*
                        }
                    },
                ));
            }
        }
        Data::Union(_) => {}
    }

    Ok(output)
}

/// 型の外側の `#[classify(変換元 as 式)]` を展開する
fn expand_item_attrs(target: &Target, attrs: &[Attribute]) -> syn::Result<TokenStream2> {
    let mut output = TokenStream2::new();
    let mut has_unclassified = false;

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("classify")) {
        for rule in item_attr::parse(attr)? {
            match rule {
                ItemRule::Unclassified(value) => {
                    if has_unclassified {
                        return Err(syn::Error::new_spanned(
                            attr,
                            "duplicate `#[classify(Unclassified as ...)]` attribute",
                        ));
                    }
                    has_unclassified = true;

                    output.extend(target.unclassified_impl(&value));
                }
                ItemRule::Rule { sources, value } => {
                    for source in sources {
                        let param = match source.pat {
                            Some(pat) => quote!(#pat),
                            None => quote!(_error),
                        };
                        output.extend(target.classify_impl(&source.ty, param, quote!(#value)));
                    }
                }
            }
        }
    }

    Ok(output)
}

/// 型の外側に付いた `#[typ]` / `#[variant]` をエラーにする
fn reject_variant_attrs(attrs: &[Attribute]) -> syn::Result<()> {
    match attrs
        .iter()
        .find(|attr| attr.path().is_ident("typ") || attr.path().is_ident("variant"))
    {
        Some(attr) => Err(syn::Error::new_spanned(
            attr,
            "`typ` / `variant` can only be used on enum variants",
        )),
        None => Ok(()),
    }
}

/// `#[classify]` が付いたフィールドがあれば、その型と `error` から値を作る式を返す
///
/// `constructor` は `Self` または `Self::Variant`
fn classify_field(
    fields: &Fields,
    constructor: TokenStream2,
) -> syn::Result<Option<(Type, TokenStream2)>> {
    let mut marked = None;

    for (index, field) in fields.iter().enumerate() {
        let Some(attr) = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("classify"))
        else {
            continue;
        };

        if !matches!(attr.meta, Meta::Path(_)) {
            return Err(syn::Error::new_spanned(
                attr,
                "use bare `#[classify]` on a field",
            ));
        }
        if marked.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                "only one field can be marked with `#[classify]`",
            ));
        }

        marked = Some((index, field));
    }

    let Some((index, field)) = marked else {
        return Ok(None);
    };

    if fields.len() > 1 {
        return Err(syn::Error::new(
            fields.span(),
            "a `#[classify]` field must be the only field, since the other fields cannot be filled",
        ));
    }

    let construct = match &field.ident {
        Some(name) => quote!(#constructor { #name: error }),
        None => {
            debug_assert_eq!(index, 0);
            quote!(#constructor(error))
        }
    };

    Ok(Some((field.ty.clone(), construct)))
}
