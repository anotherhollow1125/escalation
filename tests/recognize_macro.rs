use escalation::{Recognize, recognize};

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

recognize! {
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
recognize! {
    std::io::Error => errors::UnitError, errors::UnitError;
    e: std::fmt::Error, e: HogeError => errors::Wrapped, errors::Wrapped(std::any::type_name_of_val(e).to_string());
}

struct Pair(u8, #[allow(dead_code)] bool);

#[derive(Debug, PartialEq)]
enum Multi {
    Num(String),
}

// 変換元ごとに異なるパターン
recognize! {
    HasFieldError { inner, .. }: HasFieldError, Pair(inner, _): Pair => Multi::Num(inner.to_string());
}

#[derive(Debug, PartialEq)]
struct Plain(u8);

recognize! {
    BazError => Plain(1);
}

#[test]
fn enum_variants() {
    assert_eq!(
        <LogicalError as Recognize<HogeError>>::recognize(&HogeError).1,
        LogicalError::Xxx
    );
    assert_eq!(
        <LogicalError as Recognize<FugaError>>::recognize(&FugaError).1,
        LogicalError::Yyy("Yyy occurred")
    );
    assert_eq!(
        <LogicalError as Recognize<BarError>>::recognize(&BarError).1,
        LogicalError::Yyy("Yyy occurred")
    );
    assert_eq!(
        <LogicalError as Recognize<BazError>>::recognize(&BazError).1,
        LogicalError::Zzz
    );

    let (trace, e) = <LogicalError as Recognize<HasFieldError>>::recognize(&HasFieldError {
        inner: 42,
        other: true,
    });
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
        <errors::UnitError as Recognize<std::io::Error>>::recognize(&io).1,
        errors::UnitError
    );
    assert_eq!(
        <errors::Wrapped as Recognize<HogeError>>::recognize(&HogeError).1,
        errors::Wrapped("recognize_macro::HogeError".to_string())
    );
}

#[test]
fn single_segment_struct() {
    assert_eq!(
        <Plain as Recognize<BazError>>::recognize(&BazError).1,
        Plain(1)
    );
}

#[test]
fn different_patterns_per_source() {
    assert_eq!(
        <Multi as Recognize<HasFieldError>>::recognize(&HasFieldError {
            inner: 7,
            other: false,
        })
        .1,
        Multi::Num("7".to_string())
    );
    assert_eq!(
        <Multi as Recognize<Pair>>::recognize(&Pair(3, true)).1,
        Multi::Num("3".to_string())
    );
}
