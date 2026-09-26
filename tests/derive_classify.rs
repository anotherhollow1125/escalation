use std::fmt::{Debug, Display};

use escalation::{Classify, Decompose, ErrorInfo, Escalate, Handleable, Report, Unclassified};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Classify)]
#[classify(Unclassified => Self::Internal)]
enum SomeError {
    #[error("xxx")]
    Xxx,
    #[error("yyy")]
    Yyy,
    #[error("internal")]
    Internal,
}

// 属性なし: 自分自身への Classify のみ
#[derive(Debug, Error, PartialEq, Classify)]
#[error("plain error {0}")]
struct PlainError(u8);

// ジェネリクス付き・ジェネリクス名 `E` との衝突がないこと
#[derive(Debug, Error, PartialEq, Classify)]
#[classify(Unclassified => Self::Other)]
enum GenericError<E: Debug + Display> {
    #[error("wrapped {0}")]
    Wrapped(E),
    #[error("other")]
    Other,
}

/// `U: Decompose<T>` が実装されていることをコンパイル時に確認する
fn assert_decompose<T: Display + Debug, U: Decompose<T>>() {}

fn error_info() -> ErrorInfo {
    ErrorInfo {
        error_type_name: "test",
        path: file!(),
        fn_name: "test",
        line: line!(),
        col: column!(),
        expr: "test",
        tag: "test",
    }
}

#[test]
fn classify_self() {
    assert_eq!(SomeError::classify(SomeError::Xxx), SomeError::Xxx);
    assert_eq!(SomeError::classify(SomeError::Yyy), SomeError::Yyy);
    assert_eq!(PlainError::classify(PlainError(3)), PlainError(3));
    assert_eq!(
        GenericError::<u8>::classify(GenericError::Wrapped(1)),
        GenericError::Wrapped(1)
    );
}

#[test]
fn classify_unclassified() {
    assert_eq!(
        <SomeError as Classify<Unclassified<std::io::Error>>>::classify(Unclassified(
            std::io::Error::other("io")
        )),
        SomeError::Internal
    );
    assert_eq!(
        <SomeError as Classify<Unclassified<&str>>>::classify(Unclassified("str")),
        SomeError::Internal
    );
    assert_eq!(
        <GenericError<u8> as Classify<Unclassified<&str>>>::classify(Unclassified("str")),
        GenericError::Other
    );
}

#[test]
fn decompose_is_implemented() {
    assert_decompose::<SomeError, SomeError>();
    assert_decompose::<Unclassified<std::io::Error>, SomeError>();
    assert_decompose::<PlainError, PlainError>();
    assert_decompose::<GenericError<u8>, GenericError<u8>>();
    assert_decompose::<Unclassified<&str>, GenericError<u8>>();
}

#[test]
fn escalate_with_derived_impls() {
    let report: Report<SomeError> = SomeError::Yyy.escalate(error_info(), None);
    let Err((e, cause, trace)) = Err::<(), _>(report).handle() else {
        unreachable!()
    };
    assert_eq!(e, SomeError::Yyy);
    assert_eq!(cause.display, "yyy");
    assert_eq!(trace.len(), 1);

    let report: Report<SomeError> =
        Unclassified(std::io::Error::other("io failed")).escalate(error_info(), None);
    let (e, cause, _) = report.handle();
    assert_eq!(e, SomeError::Internal);
    assert_eq!(cause.display, "io failed");
}
