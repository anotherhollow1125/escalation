use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod classify;
mod derive_classify;

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
    parse_macro_input!(input as classify::Rules).expand().into()
}

/// 自分自身と `Unclassified<E>` からの `Classify` / `Decompose` を実装する derive マクロ
///
/// - `impl Classify<Self> for Self` (と `Decompose<Self>`) は常に生成します
/// - `#[classify(Unclassified => 式)]` がある場合、任意の `E` について
///   `impl Classify<Unclassified<E>> for Self` (と `Decompose<Unclassified<E>>`) を生成し、式の値を返します。
///   式の中では `Self` が使えます
///
/// `Decompose` は `Display + Debug` を要求するため、導出対象の型はそれらを実装している必要があります。
///
/// # 例
///
/// ```ignore
/// #[derive(Debug, thiserror::Error, Classify)]
/// #[classify(Unclassified => Self::Internal)]
/// pub enum SomeError {
///     #[error("xxx")]
///     Xxx,
///     #[error("internal")]
///     Internal,
/// }
/// ```
#[proc_macro_derive(Classify, attributes(classify))]
pub fn derive_classify(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    derive_classify::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
