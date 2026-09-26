use escalation::{Classify, Decompose, classify};

struct HogeError;
struct FugaError;
struct BarError;
struct BazError;
struct HasFieldError {
    inner: i32,
    #[allow(dead_code)]
    other: bool,
}

#[derive(Debug, PartialEq)]
enum LogicalError {
    Xxx,
    Yyy(&'static str),
    Zzz,
    Other { inner: String },
}

classify! {
    HogeError => LogicalError::Xxx;
    FugaError, BarError => LogicalError::Yyy("Yyy occurred");
    BazError => LogicalError, LogicalError::Zzz;
    HasFieldError { inner, .. }: HasFieldError => LogicalError::Other { inner: inner.to_string() }
}

mod errors {
    #[derive(Debug, PartialEq)]
    pub struct UnitError;

    #[derive(Debug, PartialEq)]
    pub struct Wrapped(pub String);
}

// 1 セグメントの構造体・パス付きの変換元・束縛のみのパターン
classify! {
    std::io::Error => errors::UnitError, errors::UnitError;
    e: std::fmt::Error, e: HogeError => errors::Wrapped, errors::Wrapped(std::any::type_name_of_val(&e).to_string());
}

struct Pair(u8, #[allow(dead_code)] bool);

#[derive(Debug, PartialEq)]
enum Multi {
    Num(String),
}

// 変換元ごとに異なるパターン
classify! {
    HasFieldError { inner, .. }: HasFieldError, Pair(inner, _): Pair => Multi::Num(inner.to_string());
}

#[derive(Debug, PartialEq)]
struct Plain(u8);

classify! {
    BazError => Plain(1);
}

/// `U: Decompose<T>` が実装されていることをコンパイル時に確認する
fn assert_decompose<T, U: Decompose<T>>() {}

#[test]
fn enum_variants() {
    assert_eq!(
        <LogicalError as Classify<HogeError>>::classify(HogeError),
        LogicalError::Xxx
    );
    assert_eq!(
        <LogicalError as Classify<FugaError>>::classify(FugaError),
        LogicalError::Yyy("Yyy occurred")
    );
    assert_eq!(
        <LogicalError as Classify<BarError>>::classify(BarError),
        LogicalError::Yyy("Yyy occurred")
    );
    assert_eq!(
        <LogicalError as Classify<BazError>>::classify(BazError),
        LogicalError::Zzz
    );
    assert_eq!(
        <LogicalError as Classify<HasFieldError>>::classify(HasFieldError {
            inner: 42,
            other: true,
        }),
        LogicalError::Other {
            inner: "42".to_string()
        }
    );
}

#[test]
fn explicit_target_and_binding_pattern() {
    let io = std::io::Error::other("x");
    assert_eq!(
        <errors::UnitError as Classify<std::io::Error>>::classify(io),
        errors::UnitError
    );
    assert_eq!(
        <errors::Wrapped as Classify<HogeError>>::classify(HogeError),
        errors::Wrapped("classify_macro::HogeError".to_string())
    );
}

#[test]
fn single_segment_struct() {
    assert_eq!(<Plain as Classify<BazError>>::classify(BazError), Plain(1));
}

#[test]
fn different_patterns_per_source() {
    assert_eq!(
        <Multi as Classify<HasFieldError>>::classify(HasFieldError {
            inner: 7,
            other: false,
        }),
        Multi::Num("7".to_string())
    );
    assert_eq!(
        <Multi as Classify<Pair>>::classify(Pair(3, true)),
        Multi::Num("3".to_string())
    );
}

#[test]
fn decompose_is_implemented() {
    assert_decompose::<HogeError, LogicalError>();
    assert_decompose::<FugaError, LogicalError>();
    assert_decompose::<BarError, LogicalError>();
    assert_decompose::<BazError, LogicalError>();
    assert_decompose::<HasFieldError, LogicalError>();
    assert_decompose::<std::io::Error, errors::UnitError>();
    assert_decompose::<std::fmt::Error, errors::Wrapped>();
    assert_decompose::<HogeError, errors::Wrapped>();
    assert_decompose::<HasFieldError, Multi>();
    assert_decompose::<Pair, Multi>();
    assert_decompose::<BazError, Plain>();
}
