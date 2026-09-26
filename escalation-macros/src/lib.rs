use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, Pat, Path, Token, Type, parse_macro_input};

/// `Classify` 実装のボイラープレートを生成するマクロ
///
/// 各ルールは `変換元 => 変換先;` の形で、`;` 区切りで複数書けます (最後の `;` は省略可)。
///
/// # 変換元
///
/// `[パターン:] 型` をカンマ区切りで並べます。複数指定すると、それぞれに同じ変換先の実装を生成します。
///
/// - `A`: 型のみ
/// - `パターン: A`: 変換元を分解するパターン付き。
///   束縛された変数は変換先の式の中で参照として使えます
/// - `p1: A, p2: B, C`: パターンは各型ごとに独立で、パターンのない型 (`C`) には適用されません
///
/// # 変換先
///
/// - `式`: 変換先の型を式のパスから推論します
///   - `Enum::Variant` / `Enum::Variant(..)` / `Enum::Variant { .. }` → `Enum`
///   - `Struct` / `Struct(..)` / `Struct { .. }` (セグメントが 1 つ) → `Struct`
/// - `型, 式`: 変換先の型を明示します
///
/// # 例
///
/// ```ignore
/// classify! {
///     HogeError => LogicalError::Xxx;
///     FugaError, BarError => LogicalError::Yyy("Yyy occurred");
///     BazError => LogicalError, LogicalError::Zzz;
///     HasFieldError { inner, .. }: HasFieldError => LogicalError::Other { inner: inner.to_string() };
/// }
/// ```
#[proc_macro]
pub fn classify(input: TokenStream) -> TokenStream {
    let Rules(rules) = parse_macro_input!(input as Rules);

    rules
        .iter()
        .map(Rule::expand)
        .collect::<TokenStream2>()
        .into()
}

struct Rules(Vec<Rule>);

impl Parse for Rules {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let rules = Punctuated::<Rule, Token![;]>::parse_terminated(input)?;

        Ok(Rules(rules.into_iter().collect()))
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
