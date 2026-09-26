use std::fmt::{Debug, Display};

use escalation::{Classify, Decompose, classify};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
#[error("hoge error")]
struct HogeError;

#[derive(Debug, Error, PartialEq)]
#[error("fuga error")]
struct FugaError;

#[derive(Debug, Error)]
enum SomeError {
    #[error("a")]
    A,
    #[error("b {0}")]
    B(String),
}

#[derive(Debug, Error, PartialEq)]
enum ConvertedError {
    #[error("aa")]
    AA,
    #[error("bb {0}")]
    BB(String),
}

#[derive(Debug, Error, PartialEq)]
#[error("other error {0}")]
struct OtherError(String);

// 依頼時の例そのまま
classify! {
    // 型単位
    HogeError => FugaError;

    // enum -> enum
    match SomeError::* => ConvertedError::* {
        A => AA,
        B(s) => BB(s),
    }

    // enum -> 任意の型
    match SomeError::* => OtherError {
        A => OtherError("a".to_string()),
        B(s) => OtherError(s),
    }
}

#[derive(Debug, Error)]
enum RichError {
    #[error("unit")]
    Unit,
    #[error("tuple {0}")]
    Tuple(u8),
    #[error("named {code}")]
    Named { code: u16, message: String },
    #[error("other1")]
    Other1,
    #[error("other2")]
    Other2,
}

#[derive(Debug, Error, PartialEq)]
enum Summary {
    #[error("simple")]
    Simple,
    #[error("code {code}")]
    Code { code: u16 },
    #[error("message {0}")]
    Message(String),
}

mod nested {
    #[derive(Debug, thiserror::Error)]
    pub enum PathError {
        #[error("x")]
        X,
        #[error("y")]
        Y,
    }
}

classify! {
    // 構造体バリアント・`|`・`@` 束縛・ガード・`_`・ブロック式
    match RichError::* => Summary::* {
        Unit => Simple,
        Tuple(n) if n > 100 => Code { code: n as u16 },
        Tuple(_) => Simple,
        Named { code, .. } => Code { code },
        Other1 | Other2 => Simple,
    };

    match RichError::* => String {
        e @ (Unit | Tuple(_)) => e.to_string(),
        Named { message, .. } => { message }
        _ => "other".to_string(),
    }

    // `::*` なし (普通の match 式) とモジュール付きパス
    match nested::PathError => Summary {
        nested::PathError::X => Summary::Message("x".to_string()),
        nested::PathError::Y => Summary::Simple,
    }
}

/// `U: Decompose<T>` が実装されていることをコンパイル時に確認する
fn assert_decompose<T: Display + Debug, U: Decompose<T>>() {}

#[test]
fn request_example() {
    assert_eq!(FugaError::classify(HogeError), FugaError);

    assert_eq!(ConvertedError::classify(SomeError::A), ConvertedError::AA);
    assert_eq!(
        ConvertedError::classify(SomeError::B("s".to_string())),
        ConvertedError::BB("s".to_string())
    );

    assert_eq!(
        OtherError::classify(SomeError::A),
        OtherError("a".to_string())
    );
    assert_eq!(
        OtherError::classify(SomeError::B("s".to_string())),
        OtherError("s".to_string())
    );
}

#[test]
fn various_patterns() {
    assert_eq!(Summary::classify(RichError::Unit), Summary::Simple);
    assert_eq!(
        Summary::classify(RichError::Tuple(200)),
        Summary::Code { code: 200 }
    );
    assert_eq!(Summary::classify(RichError::Tuple(1)), Summary::Simple);
    assert_eq!(
        Summary::classify(RichError::Named {
            code: 404,
            message: "not found".to_string(),
        }),
        Summary::Code { code: 404 }
    );
    assert_eq!(Summary::classify(RichError::Other1), Summary::Simple);
    assert_eq!(Summary::classify(RichError::Other2), Summary::Simple);
}

#[test]
fn arbitrary_target_type() {
    assert_eq!(String::classify(RichError::Unit), "unit");
    assert_eq!(String::classify(RichError::Tuple(3)), "tuple 3");
    assert_eq!(
        String::classify(RichError::Named {
            code: 1,
            message: "msg".to_string(),
        }),
        "msg"
    );
    assert_eq!(String::classify(RichError::Other1), "other");
}

#[test]
fn plain_match_without_wildcard() {
    assert_eq!(
        Summary::classify(nested::PathError::X),
        Summary::Message("x".to_string())
    );
    assert_eq!(Summary::classify(nested::PathError::Y), Summary::Simple);
}

#[test]
fn decompose_is_implemented() {
    assert_decompose::<HogeError, FugaError>();
    assert_decompose::<SomeError, ConvertedError>();
    assert_decompose::<SomeError, OtherError>();
    assert_decompose::<RichError, Summary>();
    assert_decompose::<nested::PathError, Summary>();
}
