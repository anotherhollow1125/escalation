//! `match 変換元 => 変換先 { .. }` 形式のルール
//!
//! 変換元・変換先に `列挙体::*` と書いた場合、各アームのパターン・式の先頭に `列挙体::` を補う。

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Arm, Expr, Pat, Path, PathArguments, PathSegment, Token, Type, braced};

pub(super) struct MatchRule {
    source: MatchType,
    target: MatchType,
    arms: Vec<Arm>,
}

/// `型` または `列挙体::*`
enum MatchType {
    Type(Box<Type>),
    Wildcard(Path),
}

impl Parse for MatchRule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![match]>()?;
        let source = input.parse()?;
        input.parse::<Token![=>]>()?;
        let target = input.parse()?;

        let content;
        braced!(content in input);
        let mut arms = Vec::new();
        while !content.is_empty() {
            arms.push(content.call(Arm::parse)?);
        }

        Ok(MatchRule {
            source,
            target,
            arms,
        })
    }
}

impl Parse for MatchType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fork = input.fork();
        if let Ok(path) = parse_wildcard_path(&fork) {
            input.advance_to(&fork);
            return Ok(MatchType::Wildcard(path));
        }

        Ok(MatchType::Type(input.parse()?))
    }
}

/// `列挙体::*` を読み、`::*` を除いたパスを返す
fn parse_wildcard_path(input: ParseStream) -> syn::Result<Path> {
    let leading_colon = input.parse()?;
    let mut segments = Punctuated::new();

    loop {
        segments.push_value(input.parse::<PathSegment>()?);
        let colon = input.parse::<Token![::]>()?;

        if input.parse::<Option<Token![*]>>()?.is_some() {
            return Ok(Path {
                leading_colon,
                segments,
            });
        }

        segments.push_punct(colon);
    }
}

impl MatchType {
    fn to_type(&self) -> Type {
        match self {
            MatchType::Type(ty) => (**ty).clone(),
            MatchType::Wildcard(path) => Type::Path(syn::TypePath {
                qself: None,
                path: path.clone(),
            }),
        }
    }

    /// パターン・式の先頭に付けるパス (`::*` 指定がなければ `None`)
    fn prefix(&self) -> Option<Path> {
        let MatchType::Wildcard(path) = self else {
            return None;
        };

        // 式・パターン中では `Enum<T>::A` ではなく `Enum::<T>::A` と書く必要がある
        let mut path = path.clone();
        for segment in &mut path.segments {
            if let PathArguments::AngleBracketed(args) = &mut segment.arguments {
                args.colon2_token.get_or_insert_with(Default::default);
            }
        }

        Some(path)
    }
}

impl MatchRule {
    pub(super) fn expand(&self) -> syn::Result<TokenStream2> {
        let source = self.source.to_type();
        let target = self.target.to_type();
        let source_prefix = self.source.prefix();
        let target_prefix = self.target.prefix();

        let arms = self
            .arms
            .iter()
            .map(|arm| {
                let mut arm = arm.clone();
                if let Some(prefix) = &source_prefix {
                    prefix_pat(&mut arm.pat, prefix);
                }
                if let Some(prefix) = &target_prefix {
                    prefix_expr(&mut arm.body, prefix)?;
                }

                Ok(arm)
            })
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(quote! {
            impl ::escalation::Classify<#source> for #target {
                fn classify(error: #source) -> #target {
                    match error {
                        #(#arms)*
                    }
                }
            }

            impl ::escalation::Decompose<#source> for #target {}
        })
    }
}

fn prefixed(prefix: &Path, path: &Path) -> Path {
    let mut new_path = prefix.clone();
    new_path.segments.extend(path.segments.iter().cloned());

    new_path
}

/// パターン中のバリアント名の先頭に `prefix::` を付ける
///
/// `_` や `x @ ..` の束縛部分、リテラルなどはそのまま残す
fn prefix_pat(pat: &mut Pat, prefix: &Path) {
    match pat {
        // `A` → `Enum::A` (束縛修飾子・サブパターン付きは束縛として扱う)
        Pat::Ident(p) => {
            if let Some((_, sub)) = &mut p.subpat {
                prefix_pat(sub, prefix);
            } else if p.by_ref.is_none() && p.mutability.is_none() {
                *pat = Pat::Path(syn::ExprPath {
                    attrs: p.attrs.clone(),
                    qself: None,
                    path: prefixed(prefix, &p.ident.clone().into()),
                });
            }
        }
        Pat::Path(p) if p.qself.is_none() => p.path = prefixed(prefix, &p.path),
        Pat::TupleStruct(p) if p.qself.is_none() => p.path = prefixed(prefix, &p.path),
        Pat::Struct(p) if p.qself.is_none() => p.path = prefixed(prefix, &p.path),
        Pat::Or(p) => p.cases.iter_mut().for_each(|case| prefix_pat(case, prefix)),
        Pat::Paren(p) => prefix_pat(&mut p.pat, prefix),
        _ => {}
    }
}

/// アームの式 `A` / `A(..)` / `A { .. }` の先頭に `prefix::` を付ける
fn prefix_expr(expr: &mut Expr, prefix: &Path) -> syn::Result<()> {
    let path = match expr {
        Expr::Path(e) if e.qself.is_none() => &mut e.path,
        Expr::Struct(e) if e.qself.is_none() => &mut e.path,
        Expr::Call(e) => match &mut *e.func {
            Expr::Path(f) if f.qself.is_none() => &mut f.path,
            _ => return Err(unsupported_expr(expr)),
        },
        _ => return Err(unsupported_expr(expr)),
    };

    *path = prefixed(prefix, path);

    Ok(())
}

fn unsupported_expr(expr: &Expr) -> syn::Error {
    syn::Error::new_spanned(
        expr,
        "with `Enum::*` as the target, each arm must be `Variant`, `Variant(..)` or `Variant { .. }`; \
         write the target as a plain type to use arbitrary expressions",
    )
}
