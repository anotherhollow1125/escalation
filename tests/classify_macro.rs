use escalation::{Classify, classify};

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
    e: std::fmt::Error, e: HogeError => errors::Wrapped, errors::Wrapped(std::any::type_name_of_val(e).to_string());
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

#[test]
fn enum_variants() {
    assert_eq!(
        <LogicalError as Classify<HogeError>>::classify(&HogeError).2,
        LogicalError::Xxx
    );
    assert_eq!(
        <LogicalError as Classify<FugaError>>::classify(&FugaError).2,
        LogicalError::Yyy("Yyy occurred")
    );
    assert_eq!(
        <LogicalError as Classify<BarError>>::classify(&BarError).2,
        LogicalError::Yyy("Yyy occurred")
    );
    assert_eq!(
        <LogicalError as Classify<BazError>>::classify(&BazError).2,
        LogicalError::Zzz
    );

    let (cause, trace, e) = <LogicalError as Classify<HasFieldError>>::classify(&HasFieldError {
        inner: 42,
        other: true,
    });
    assert!(cause.is_none());
    assert!(trace.is_empty());
    assert_eq!(
        e,
        LogicalError::Other {
            inner: "42".to_string()
        }
    );
}

#[test]
fn explicit_target_and_binding_pattern() {
    let io = std::io::Error::other("x");
    assert_eq!(
        <errors::UnitError as Classify<std::io::Error>>::classify(&io).2,
        errors::UnitError
    );
    assert_eq!(
        <errors::Wrapped as Classify<HogeError>>::classify(&HogeError).2,
        errors::Wrapped("classify_macro::HogeError".to_string())
    );
}

#[test]
fn single_segment_struct() {
    assert_eq!(
        <Plain as Classify<BazError>>::classify(&BazError).2,
        Plain(1)
    );
}

#[test]
fn different_patterns_per_source() {
    assert_eq!(
        <Multi as Classify<HasFieldError>>::classify(&HasFieldError {
            inner: 7,
            other: false,
        })
        .2,
        Multi::Num("7".to_string())
    );
    assert_eq!(
        <Multi as Classify<Pair>>::classify(&Pair(3, true)).2,
        Multi::Num("3".to_string())
    );
}
