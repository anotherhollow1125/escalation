//! 関数風マクロ `classify!` の実装

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, Pat, Path, Token, Type};

pub(crate) struct Rules(Vec<Rule>);

impl Parse for Rules {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let rules = Punctuated::<Rule, Token![;]>::parse_terminated(input)?;

        Ok(Rules(rules.into_iter().collect()))
    }
}

impl Rules {
    pub(crate) fn expand(&self) -> TokenStream2 {
        self.0.iter().map(Rule::expand).collect()
    }
}

struct Rule {
    sources: Vec<Source>,
    target: Type,
    value: Expr,
}

/// `パターン: 型` または `型`
struct Source {
    pat: Option<Pat>,
    ty: Type,
}

impl Parse for Rule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let sources = parse_sources(input)?;
        input.parse::<Token![=>]>()?;
        let (target, value) = parse_target(input)?;

        Ok(Rule {
            sources,
            target,
            value,
        })
    }
}

impl Parse for Source {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pat = parse_pattern(input)?;
        let ty = input.parse()?;

        Ok(Source { pat, ty })
    }
}

/// `パターン:` があれば読み進めて返す
fn parse_pattern(input: ParseStream) -> syn::Result<Option<Pat>> {
    let fork = input.fork();
    let is_pattern =
        Pat::parse_single(&fork).is_ok() && fork.peek(Token![:]) && !fork.peek(Token![::]);

    if !is_pattern {
        return Ok(None);
    }

    let pat = Pat::parse_single(input)?;
    input.parse::<Token![:]>()?;

    Ok(Some(pat))
}

/// `=>` の直前までのカンマ区切りの変換元を読む
fn parse_sources(input: ParseStream) -> syn::Result<Vec<Source>> {
    let mut sources = vec![input.parse::<Source>()?];

    while input.parse::<Option<Token![,]>>()?.is_some() {
        sources.push(input.parse()?);
    }

    Ok(sources)
}

/// `型, 式` または `式` を読む
fn parse_target(input: ParseStream) -> syn::Result<(Type, Expr)> {
    let fork = input.fork();
    if fork.parse::<Type>().is_ok() && fork.peek(Token![,]) {
        let target = input.parse::<Type>()?;
        input.parse::<Token![,]>()?;
        let value = input.parse::<Expr>()?;

        return Ok((target, value));
    }

    let value = input.parse::<Expr>()?;
    let target = infer_target(&value)?;

    Ok((target, value))
}

/// 式のパスから変換先の型を推論する
fn infer_target(value: &Expr) -> syn::Result<Type> {
    let (qself, path): (_, &Path) = match value {
        Expr::Path(e) => (&e.qself, &e.path),
        Expr::Struct(e) => (&e.qself, &e.path),
        Expr::Call(e) => match &*e.func {
            Expr::Path(f) => (&f.qself, &f.path),
            _ => return Err(cannot_infer(value)),
        },
        _ => return Err(cannot_infer(value)),
    };

    if qself.is_some() {
        return Err(cannot_infer(value));
    }

    let mut path = path.clone();
    if path.segments.len() >= 2 {
        // `Enum::Variant` → `Enum`
        path.segments.pop();
        path.segments.pop_punct();
    }

    Ok(Type::Path(syn::TypePath { qself: None, path }))
}

fn cannot_infer(value: &Expr) -> syn::Error {
    syn::Error::new(
        value.span(),
        "cannot infer the target type from this expression; \
         specify it explicitly like `TargetType, expr`",
    )
}

impl Rule {
    fn expand(&self) -> TokenStream2 {
        let Rule {
            sources,
            target,
            value,
        } = self;

        sources
            .iter()
            .map(|Source { pat, ty }| {
                let pat = match pat {
                    Some(pat) => quote! { #pat },
                    None => quote! { _error },
                };

                quote! {
                    impl ::escalation::Classify<#ty> for #target {
                        fn classify(#pat: #ty) -> #target {
                            #value
                        }
                    }

                    impl ::escalation::Decompose<#ty> for #target {}
                }
            })
            .collect()
    }
}
