//! 型の外側に付ける `#[classify(変換元 as 式; ..)]`
//!
//! 変換元の書き方は `classify!` と同じ (`A, B` / `パターン: A` / `束縛名 @ A`)。
//! 変換先は常に導出対象の型なので、`as` の後ろには式だけを書く。
//! `Unclassified as 式` だけは特別扱いで、任意の `Unclassified<E>` からの実装になる。

use syn::parse::ParseStream;
use syn::{Attribute, Expr, Ident, Token, Type};

use crate::classify::{Source, parse_sources};

pub(super) enum ItemRule {
    Unclassified(Expr),
    Rule { sources: Vec<Source>, value: Expr },
}

pub(super) fn parse(attr: &Attribute) -> syn::Result<Vec<ItemRule>> {
    attr.parse_args_with(|input: ParseStream| {
        let mut rules = Vec::new();

        while !input.is_empty() {
            let fork = input.fork();
            if let Ok(name) = fork.parse::<Ident>()
                && fork.peek(syn::token::Paren)
                && (name == "typ" || name == "variant")
            {
                return Err(input.error("`typ` / `variant` can only be used on enum variants"));
            }

            rules.push(parse_rule(input)?);
            if input.is_empty() {
                break;
            }
            input.parse::<Token![;]>()?;
        }

        Ok(rules)
    })
}

fn parse_rule(input: ParseStream) -> syn::Result<ItemRule> {
    let sources = parse_sources(input)?;
    input.parse::<Token![as]>()?;
    let value = input.parse::<Expr>()?;

    let unclassified = sources
        .iter()
        .position(|source| is_unclassified(&source.ty));
    match unclassified {
        None => Ok(ItemRule::Rule { sources, value }),
        Some(index) if sources.len() == 1 && sources[index].pat.is_none() => {
            Ok(ItemRule::Unclassified(value))
        }
        Some(index) => Err(syn::Error::new_spanned(
            &sources[index].ty,
            "`Unclassified` must be written alone, like `Unclassified as expr`",
        )),
    }
}

/// 型引数なしの `Unclassified`
fn is_unclassified(ty: &Type) -> bool {
    let Type::Path(ty) = ty else {
        return false;
    };

    ty.qself.is_none()
        && ty.path.segments.len() == 1
        && ty.path.segments[0].ident == "Unclassified"
        && ty.path.segments[0].arguments.is_none()
}
