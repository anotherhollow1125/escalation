use std::fmt::{Debug, Display};

use escalation::{Classify, Decompose, Unclassified};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
#[error("some error")]
struct SomeError;

#[derive(Debug, Error, PartialEq)]
#[error("other error")]
struct OtherError;

#[derive(Debug, Error, PartialEq)]
#[error("else error")]
struct ElseError;

#[derive(Debug, Error, PartialEq)]
enum HogeError {
    #[error("x")]
    X,
    #[error("y")]
    Y,
}

#[derive(Debug, Error, PartialEq)]
enum FugaError {
    #[error("x")]
    X,
    #[error("xxx")]
    Xxx,
    #[error("zzz")]
    Zzz,
}

// ---- 依頼時の例 ----

// 1. 型の外側の `#[classify(.. as ..)]`
#[derive(Debug, Error, PartialEq, Classify)]
#[classify(SomeError as HereError1::Other)]
enum HereError1 {
    #[error("xxxxx")]
    #[allow(dead_code)]
    Xxxxx,
    #[error("other")]
    Other,
}

// 2. フィールドの `#[classify]`
#[derive(Debug, Error, PartialEq, Classify)]
enum HereError2 {
    #[error("xxxxx")]
    #[allow(dead_code)]
    Xxxxx,
    #[error("other: {0}")]
    Other(#[classify] SomeError),
}

// 3. バリアントの `#[typ(..)]` / `#[classify(typ(..))]`
#[derive(Debug, Error, PartialEq, Classify)]
enum HereError3 {
    #[error("xxxxx")]
    #[typ(SomeError, OtherError)]
    Xxxxx,
    #[error("other")]
    #[classify(typ(ElseError))]
    Other,
}

// 4. バリアントの `#[variant(..)]` / `#[classify(variant(..))]`
#[derive(Debug, Error, PartialEq, Classify)]
enum HereError4 {
    #[error("xxxxx")]
    #[variant(HogeError::X, FugaError::Xxx)]
    Xxxxx,
    #[error("other")]
    #[classify(variant(HogeError::Y, FugaError::_))]
    Other,
}

// ---- その他のケース ----

// 型の外側: 複数ルール (`;` 区切り)・複数属性・`@` 束縛・Unclassified との併用
#[derive(Debug, Error, PartialEq, Classify)]
#[classify(SomeError, OtherError as Self::Fixed; e @ std::num::ParseIntError as Self::Message(e.to_string()))]
#[classify(Unclassified as Self::Unknown)]
enum ItemAttrs {
    #[error("fixed")]
    Fixed,
    #[error("message: {0}")]
    Message(String),
    #[error("unknown")]
    Unknown,
}

// 構造体のフィールド (タプル・名前付き)
#[derive(Debug, Error, PartialEq, Classify)]
#[error("tuple: {0}")]
struct TupleWrapper(#[classify] SomeError);

#[derive(Debug, Error, PartialEq, Classify)]
#[error("named: {inner}")]
struct NamedWrapper {
    #[classify]
    inner: SomeError,
}

// 名前付きフィールドのバリアント、`typ` と `variant` を 1 つの `classify` にまとめた書き方、
// `_` を先に書いても最後のアームになること、フィールドのあるパターン
#[derive(Debug, Error, PartialEq)]
enum Detailed {
    #[error("code {0}")]
    Code(u16),
    #[error("named {name}")]
    Named { name: String },
    #[error("unit")]
    Unit,
}

#[derive(Debug, Error, PartialEq, Classify)]
enum Mixed {
    #[error("wrapped")]
    Wrapped {
        #[classify]
        source: OtherError,
    },
    #[error("rest")]
    #[classify(typ(ElseError), variant(Detailed::_))]
    Rest,
    #[error("big code")]
    #[variant(Detailed::Code(500..), Detailed::Named { .. })]
    Specific,
}

// ジェネリックな列挙体への導出と、ジェネリックな分類前列挙体 (ターボフィッシュ)
#[derive(Debug, Error, PartialEq)]
enum Gen<T: Debug> {
    #[error("a {0:?}")]
    A(T),
    #[error("b")]
    B,
}

// (型引数 `E` そのものに `#[classify]` を付けると、`Classify<Report<T>>` の汎用実装と衝突するので不可)
#[derive(Debug, Error, PartialEq, Classify)]
enum GenericTarget<E: Debug + Display> {
    #[error("generic {0}")]
    #[allow(dead_code)]
    Generic(E),
    #[error("wrapped {0}")]
    Wrapped(#[classify] SomeError),
    #[error("from a")]
    #[variant(Gen::<u8>::A(_))]
    FromA,
    #[error("from b")]
    #[variant(Gen::<u8>::B)]
    FromB,
}

/// `U: Decompose<T>` が実装されていることをコンパイル時に確認する
fn assert_decompose<T: Display + Debug, U: Decompose<T>>() {}

#[test]
fn request_examples() {
    // 1
    assert_eq!(HereError1::classify(SomeError), HereError1::Other);

    // 2
    assert_eq!(
        HereError2::classify(SomeError),
        HereError2::Other(SomeError)
    );

    // 3
    assert_eq!(HereError3::classify(SomeError), HereError3::Xxxxx);
    assert_eq!(HereError3::classify(OtherError), HereError3::Xxxxx);
    assert_eq!(HereError3::classify(ElseError), HereError3::Other);

    // 4
    assert_eq!(HereError4::classify(HogeError::X), HereError4::Xxxxx);
    assert_eq!(HereError4::classify(HogeError::Y), HereError4::Other);
    assert_eq!(HereError4::classify(FugaError::Xxx), HereError4::Xxxxx);
    assert_eq!(HereError4::classify(FugaError::X), HereError4::Other);
    assert_eq!(HereError4::classify(FugaError::Zzz), HereError4::Other);

    // 自分自身からの Classify はこれまで通り
    assert_eq!(HereError4::classify(HereError4::Xxxxx), HereError4::Xxxxx);
}

#[test]
fn item_attrs() {
    assert_eq!(ItemAttrs::classify(SomeError), ItemAttrs::Fixed);
    assert_eq!(ItemAttrs::classify(OtherError), ItemAttrs::Fixed);

    let parse_error = "x".parse::<u8>().unwrap_err();
    assert_eq!(
        ItemAttrs::classify(parse_error.clone()),
        ItemAttrs::Message(parse_error.to_string())
    );

    assert_eq!(
        <ItemAttrs as Classify<Unclassified<&str>>>::classify(Unclassified("?")),
        ItemAttrs::Unknown
    );
}

#[test]
fn struct_fields() {
    assert_eq!(TupleWrapper::classify(SomeError), TupleWrapper(SomeError));
    assert_eq!(
        NamedWrapper::classify(SomeError),
        NamedWrapper { inner: SomeError }
    );
}

#[test]
fn mixed() {
    assert_eq!(
        Mixed::classify(OtherError),
        Mixed::Wrapped { source: OtherError }
    );
    assert_eq!(Mixed::classify(ElseError), Mixed::Rest);

    assert_eq!(Mixed::classify(Detailed::Code(503)), Mixed::Specific);
    assert_eq!(Mixed::classify(Detailed::Code(404)), Mixed::Rest);
    assert_eq!(
        Mixed::classify(Detailed::Named {
            name: "n".to_string()
        }),
        Mixed::Specific
    );
    assert_eq!(Mixed::classify(Detailed::Unit), Mixed::Rest);
}

#[test]
fn generics() {
    assert_eq!(
        GenericTarget::<SomeError>::classify(SomeError),
        GenericTarget::Wrapped(SomeError)
    );
    assert_eq!(
        GenericTarget::<SomeError>::classify(Gen::<u8>::A(1)),
        GenericTarget::FromA
    );
    assert_eq!(
        GenericTarget::<SomeError>::classify(Gen::<u8>::B),
        GenericTarget::FromB
    );
}

#[test]
fn decompose_is_implemented() {
    assert_decompose::<SomeError, HereError1>();
    assert_decompose::<SomeError, HereError2>();
    assert_decompose::<SomeError, HereError3>();
    assert_decompose::<OtherError, HereError3>();
    assert_decompose::<ElseError, HereError3>();
    assert_decompose::<HogeError, HereError4>();
    assert_decompose::<FugaError, HereError4>();
    assert_decompose::<SomeError, TupleWrapper>();
    assert_decompose::<SomeError, NamedWrapper>();
    assert_decompose::<Detailed, Mixed>();
    assert_decompose::<Gen<u8>, GenericTarget<SomeError>>();
}
