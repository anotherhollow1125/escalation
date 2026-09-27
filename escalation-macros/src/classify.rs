//! 関数風マクロ `classify!` の実装

mod match_rule;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Expr, Ident, Pat, Path, Token, Type};

use self::match_rule::MatchRule;

pub(crate) struct Rules(Vec<Item>);

enum Item {
    /// `変換元 as 変換先;`
    Rule(Box<Rule>),
    /// `match 変換元 as 変換先 { .. }`
    Match(MatchRule),
}

impl Parse for Rules {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();

        while !input.is_empty() {
            if input.peek(Token![match]) {
                items.push(Item::Match(input.parse()?));
                // `match` ブロックの後の `;` は省略可
                input.parse::<Option<Token![;]>>()?;
                continue;
            }

            items.push(Item::Rule(input.parse()?));
            if input.is_empty() {
                break;
            }
            input.parse::<Token![;]>()?;
        }

        Ok(Rules(items))
    }
}

impl Rules {
    pub(crate) fn expand(&self) -> syn::Result<TokenStream2> {
        self.0
            .iter()
            .map(|item| match item {
                Item::Rule(rule) => Ok(rule.expand()),
                Item::Match(rule) => rule.expand(),
            })
            .collect()
    }
}

struct Rule {
    sources: Vec<Source>,
    target: Type,
    value: Expr,
}

/// `パターン: 型`、`束縛名 @ 型` または `型`
pub(crate) struct Source {
    pub(crate) pat: Option<Pat>,
    pub(crate) ty: Type,
}

impl Parse for Rule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let sources = parse_sources(input)?;
        input.parse::<Token![as]>()?;
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
        // `束縛名 @ 型`: 変換元の値全体を束縛する
        // (`パターン:` の判定より先に見ないと `名前 @ パス` がパターンとして読まれてしまう)
        if input.peek(Ident) && input.peek2(Token![@]) {
            let ident = input.parse::<Ident>()?;
            input.parse::<Token![@]>()?;
            let ty = input.parse()?;

            return Ok(Source {
                pat: Some(Pat::Ident(syn::PatIdent {
                    attrs: Vec::new(),
                    by_ref: None,
                    mutability: None,
                    ident,
                    subpat: None,
                })),
                ty,
            });
        }

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

/// `as` の直前までのカンマ区切りの変換元を読む
pub(crate) fn parse_sources(input: ParseStream) -> syn::Result<Vec<Source>> {
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
