use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod classify;
mod derive_classify;

/// `Classify` 実装のボイラープレートを生成するマクロ
///
/// 各ルールは `変換元 as 変換先;` の形で、`;` 区切りで複数書けます (最後の `;` は省略可)。
///
/// # 変換元
///
/// `[パターン:] 型` をカンマ区切りで並べます。複数指定すると、それぞれに同じ変換先の実装を生成します。
///
/// - `A`: 型のみ
/// - `パターン: A`: 変換元を分解するパターン付き。
///   束縛された変数は変換先の式の中で参照として使えます
/// - `束縛名 @ A`: 変換元の値全体を `束縛名` に束縛します (所有権ごと受け取るので、そのまま変換先に渡せます)
/// - `p1: A, p2: B, C`: パターンは各型ごとに独立で、パターンのない型 (`C`) には適用されません
///
/// # 変換先
///
/// - `式`: 変換先の型を式のパスから推論します
///   - `Enum::Variant` / `Enum::Variant(..)` / `Enum::Variant { .. }` → `Enum`
///   - `Struct` / `Struct(..)` / `Struct { .. }` (セグメントが 1 つ) → `Struct`
/// - `型, 式`: 変換先の型を明示します
///
/// # `match` 形式
///
/// `match 変換元の型 as 変換先の型 { アーム, .. }` と書くと、変換元の値に対する `match` 式を
/// そのまま `classify` の本体にします。この形式の後の `;` は省略できます。
///
/// 型の代わりに `列挙体::*` と書くと、アームに `列挙体::` を補います。
///
/// - 変換元が `Src::*`: パターン中のバリアント (`A` / `A(..)` / `A { .. }` / `A | B`) を `Src::A` などにします。
///   `_`、`x @ ..` の束縛部分、`ref x` / `mut x`、リテラルなどはそのままです
/// - 変換先が `Dst::*`: アームの式 (`A` / `A(..)` / `A { .. }` のみ) を `Dst::A` などにします。
///   それ以外の式を書きたい場合は変換先を `Dst` (型) と書いてください
///
/// # 例
///
/// ```ignore
/// classify! {
///     HogeError as LogicalError::Xxx;
///     FugaError, BarError as LogicalError::Yyy("Yyy occurred");
///     BazError as LogicalError, LogicalError::Zzz;
///     HasFieldError { inner, .. }: HasFieldError as LogicalError::Other { inner: inner.to_string() };
///     error @ anyhow::Error as Wrapped(error);
///
///     match SomeError::* as ConvertedError::* {
///         A => AA,
///         B(s) => BB(s),
///     }
///
///     match SomeError::* as OtherError {
///         A => OtherError("a".to_string()),
///         B(s) => OtherError(s),
///     }
/// }
/// ```
#[proc_macro]
pub fn classify(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as classify::Rules)
        .expand()
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// 導出対象の型への `Classify` / `Decompose` を実装する derive マクロ
///
/// どの指定から生成される impl も、`Classify` と `Decompose` の両方です。
/// `Decompose` は `Display + Debug` を要求するため、変換元の型はそれらを実装している必要があります。
///
/// # 自分自身から
///
/// `impl Classify<Self> for Self` は常に生成します。
///
/// # 型の外側: `#[classify(変換元 as 式)]`
///
/// `classify!` の `変換元 as 変換先` と同じです。変換先は常に導出対象の型なので、`as` の後ろには式だけを書きます
/// (式の中では `Self` が使えます)。変換元の書き方 (`A, B` / `パターン: A` / `束縛名 @ A`) も `classify!` と同じで、
/// `;` 区切りで複数のルールを書けます。属性を複数付けても構いません。
///
/// `#[classify(Unclassified as 式)]` だけは特別で、任意の `E` について `Classify<Unclassified<E>>` を生成します。
///
/// # フィールド: `#[classify]`
///
/// thiserror の `#[from]` と同じ発想で、フィールドの型からそのフィールドを持つ値を作ります。
/// フィールドは 1 つだけのものに限ります (他のフィールドを埋められないため)。
/// 型引数そのもの (`Variant(#[classify] E)`) には付けられません (`Classify<Report<T>>` の汎用実装と衝突します)。
///
/// # バリアント: `#[typ(型, ..)]` (= `#[classify(typ(型, ..))]`)
///
/// 列挙された型をそのバリアントに分類します。ユニットバリアントにのみ付けられます。
///
/// # バリアント: `#[variant(列挙体::バリアント, ..)]` (= `#[classify(variant(..))]`)
///
/// 分類前の列挙体のバリアントをそのバリアントに分類します。ユニットバリアントにのみ付けられます。
/// `::` より前の部分が同じものは同じ分類前の列挙体とみなし、1 つの `match` 式にまとめます。
///
/// - `列挙体::X` / `列挙体::X(..)` / `列挙体::X { .. }` などのパターンが書けます
/// - `列挙体::_` は残り全部を表し、書いた位置に関わらず最後のアームになります
/// - 網羅されていない場合は通常の `match` と同じくコンパイルエラーになります
///
/// # 例
///
/// ```ignore
/// #[derive(Debug, thiserror::Error, Classify)]
/// #[classify(SomeError as Self::Other)]
/// #[classify(Unclassified as Self::Internal)]
/// pub enum HereError {
///     #[error("xxxxx")]
///     #[typ(OtherError, ElseError)]
///     #[variant(HogeError::X, FugaError::Xxx)]
///     Xxxxx,
///     #[error("other")]
///     #[variant(HogeError::Y, FugaError::_)]
///     Other,
///     #[error("wrapped: {0}")]
///     Wrapped(#[classify] WrappedError),
///     #[error("internal")]
///     Internal,
/// }
/// ```
#[proc_macro_derive(Classify, attributes(classify, typ, variant))]
pub fn derive_classify(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    derive_classify::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
