//! 列挙体のバリアントに付ける属性
//!
//! - `#[typ(A, B)]` / `#[classify(typ(A, B))]`: 型 `A`, `B` をこのバリアントに分類する
//! - `#[variant(E::X, F::_)]` / `#[classify(variant(E::X, F::_))]`: 列挙体 `E` の `X` などをこのバリアントに分類する。
//!   同じ列挙体 (`::` より前が同じ) のものは 1 つの `match` 式にまとめる

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, Meta, Pat, Path, PathSegment, Token, Type};

/// 1 つのバリアントに付いた `typ` / `variant` の指定
pub(super) struct VariantAttrs {
    pub(super) types: Vec<Type>,
    pub(super) patterns: Vec<VariantPattern>,
}

impl VariantAttrs {
    pub(super) fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = VariantAttrs {
            types: Vec::new(),
            patterns: Vec::new(),
        };

        for attr in attrs {
            if attr.path().is_ident("typ") {
                result.types.extend(attr.parse_args_with(parse_types)?);
            } else if attr.path().is_ident("variant") {
                result
                    .patterns
                    .extend(attr.parse_args_with(parse_patterns)?);
            } else if attr.path().is_ident("classify") {
                if !matches!(attr.meta, Meta::List(_)) {
                    return Err(unsupported_classify(attr));
                }

                let items =
                    attr.parse_args_with(Punctuated::<ClassifyItem, Token![,]>::parse_terminated)?;
                for item in items {
                    match item {
                        ClassifyItem::Types(types) => result.types.extend(types),
                        ClassifyItem::Patterns(patterns) => result.patterns.extend(patterns),
                    }
                }
            }
        }

        Ok(result)
    }
}

fn unsupported_classify(attr: &Attribute) -> syn::Error {
    syn::Error::new_spanned(
        attr,
        "on enum variants, use `#[classify(typ(..))]` or `#[classify(variant(..))]`",
    )
}

/// `#[classify(..)]` の中の `typ(..)` / `variant(..)`
enum ClassifyItem {
    Types(Vec<Type>),
    Patterns(Vec<VariantPattern>),
}

impl Parse for ClassifyItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse::<Ident>()?;
        let content;
        syn::parenthesized!(content in input);

        if name == "typ" {
            Ok(ClassifyItem::Types(parse_types(&content)?))
        } else if name == "variant" {
            Ok(ClassifyItem::Patterns(parse_patterns(&content)?))
        } else {
            Err(syn::Error::new(
                name.span(),
                "on enum variants, expected `typ(..)` or `variant(..)`",
            ))
        }
    }
}

fn parse_types(input: ParseStream) -> syn::Result<Vec<Type>> {
    Ok(Punctuated::<Type, Token![,]>::parse_terminated(input)?
        .into_iter()
        .collect())
}

fn parse_patterns(input: ParseStream) -> syn::Result<Vec<VariantPattern>> {
    Ok(
        Punctuated::<VariantPattern, Token![,]>::parse_terminated(input)?
            .into_iter()
            .collect(),
    )
}

/// `列挙体::バリアントのパターン` または `列挙体::_`
pub(super) struct VariantPattern {
    /// `::` より前の部分 (分類前の列挙体)
    enum_path: Path,
    /// `None` なら `列挙体::_` (残り全部)
    pat: Option<Pat>,
    span: Span,
}

impl Parse for VariantPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();

        let fork = input.fork();
        if let Ok(enum_path) = parse_wildcard(&fork) {
            syn::parse::discouraged::Speculative::advance_to(input, &fork);
            return Ok(VariantPattern {
                enum_path,
                pat: None,
                span,
            });
        }

        let pat = Pat::parse_single(input)?;
        let path = match &pat {
            Pat::Path(p) if p.qself.is_none() => &p.path,
            Pat::TupleStruct(p) if p.qself.is_none() => &p.path,
            Pat::Struct(p) if p.qself.is_none() => &p.path,
            _ => return Err(expected_enum_variant(&pat)),
        };
        if path.segments.len() < 2 {
            return Err(expected_enum_variant(&pat));
        }

        let mut enum_path = path.clone();
        enum_path.segments.pop();
        enum_path.segments.pop_punct();

        Ok(VariantPattern {
            enum_path,
            pat: Some(pat),
            span,
        })
    }
}

fn expected_enum_variant(pat: &Pat) -> syn::Error {
    syn::Error::new_spanned(
        pat,
        "expected `Enum::Variant`, `Enum::Variant(..)`, `Enum::Variant { .. }` or `Enum::_`",
    )
}

/// `列挙体::_` を読み、`::_` を除いたパスを返す
fn parse_wildcard(input: ParseStream) -> syn::Result<Path> {
    let leading_colon = input.parse()?;
    let mut segments = Punctuated::new();

    loop {
        segments.push_value(input.parse::<PathSegment>()?);
        let colon = input.parse::<Token![::]>()?;

        if input.parse::<Option<Token![_]>>()?.is_some() {
            return Ok(Path {
                leading_colon,
                segments,
            });
        }

        segments.push_punct(colon);
    }
}

/// `#[variant(..)]` のアームを分類前の列挙体ごとに集める
#[derive(Default)]
pub(super) struct VariantArms {
    groups: Vec<Group>,
}

struct Group {
    /// `::` より前の部分をトークン列にした文字列 (同じ列挙体かの判定に使う)
    key: String,
    source: Path,
    arms: Vec<TokenStream2>,
    wildcard: Option<TokenStream2>,
}

impl VariantArms {
    pub(super) fn push(&mut self, pattern: VariantPattern, variant: &Ident) -> syn::Result<()> {
        let key = pattern.enum_path.to_token_stream().to_string();
        let index = match self.groups.iter().position(|group| group.key == key) {
            Some(index) => index,
            None => {
                self.groups.push(Group {
                    key,
                    source: pattern.enum_path,
                    arms: Vec::new(),
                    wildcard: None,
                });
                self.groups.len() - 1
            }
        };
        let group = &mut self.groups[index];

        match pattern.pat {
            Some(pat) => group.arms.push(quote!(#pat => Self::#variant,)),
            None => {
                if group.wildcard.is_some() {
                    return Err(syn::Error::new(
                        pattern.span,
                        "`_` for the same enum is specified more than once",
                    ));
                }
                group.wildcard = Some(quote!(_ => Self::#variant,));
            }
        }

        Ok(())
    }

    /// 分類前の列挙体の型と `match` のアームの組を返す (`_` のアームは最後に置く)
    pub(super) fn finish(self) -> Vec<(Type, Vec<TokenStream2>)> {
        self.groups
            .into_iter()
            .map(|group| {
                let mut arms = group.arms;
                arms.extend(group.wildcard);

                let source = Type::Path(syn::TypePath {
                    qself: None,
                    path: group.source,
                });
                (source, arms)
            })
            .collect()
    }
}
