use std::{
    any::type_name,
    fmt::{Debug, Display},
    panic::Location,
};

pub use escalation_macros::{Classify, classify};
use thiserror::Error;

#[macro_export]
macro_rules! wrapping {
    (_) => {
        None
    };
    ($val:expr) => {
        Some($val)
    };
}

#[derive(Error, Debug, Clone, Copy)]
#[error("{path}:{fn_name}:{line}:{col}")]
pub struct ErrorInfo {
    pub error_type_name: &'static str,
    pub path: &'static str,
    pub fn_name: &'static str,
    pub line: u32,
    pub col: u32,
    pub expr: &'static str,
    pub tag: &'static str,
}

#[derive(Debug, Clone)]
pub struct ErrorCause {
    pub display: String,
    pub debug: String,
    pub error_type_name: &'static str,
}

pub trait Classify<T> {
    fn classify(error: T) -> Self;
}

#[doc(hidden)]
pub struct Decomposed<T> {
    cause: ErrorCause,
    trace: Vec<ErrorInfo>,
    classified: T,
}

pub trait Decompose<T>: Classify<T> + Sized
where
    T: Display + Debug,
{
    fn decompose(error: T) -> Decomposed<Self> {
        let cause = ErrorCause {
            display: error.to_string(),
            debug: format!("{error:?}"),
            error_type_name: type_name::<T>(),
        };

        Decomposed {
            cause,
            trace: Vec::new(),
            classified: Self::classify(error),
        }
    }
}

#[derive(Debug)]
pub struct Report<E> {
    inner: E,
    cause: ErrorCause,
    trace: Vec<ErrorInfo>,
}

impl<E> Report<E> {
    pub fn handle(self) -> (E, ErrorCause, Vec<ErrorInfo>) {
        (self.inner, self.cause, self.trace)
    }
}

impl<E> Display for Report<E>
where
    E: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Report {{ inner: \"{:?}\", cause: \"{:?}\"}}",
            self.inner, self.cause
        )
    }
}

pub trait Handleable {
    type ConvertedType;

    fn handle(self) -> Self::ConvertedType;
}

impl<T, E> Handleable for Result<T, Report<E>> {
    type ConvertedType = Result<T, (E, ErrorCause, Vec<ErrorInfo>)>;

    fn handle(self) -> Self::ConvertedType {
        match self {
            Ok(v) => Ok(v),
            Err(r) => {
                let tuple = r.handle();
                Err(tuple)
            }
        }
    }
}

impl<T, U> Classify<Report<T>> for U
where
    U: Classify<T>,
{
    fn classify(report: Report<T>) -> Self {
        Self::classify(report.inner)
    }
}

impl<T, U> Decompose<Report<T>> for U
where
    T: Debug,
    U: Classify<T>,
{
    fn decompose(report: Report<T>) -> Decomposed<U> {
        let Report {
            inner,
            cause,
            trace,
        } = report;

        Decomposed {
            cause,
            trace,
            classified: U::classify(inner),
        }
    }
}

/// deriveマクロ `Classify` でこの構造体の Classify<Unclassified> を生成できるようになっている
pub struct Unclassified<E>(pub E);

impl<E: Display> Display for Unclassified<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<E: Debug> Debug for Unclassified<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<E> From<E> for Unclassified<E> {
    fn from(value: E) -> Self {
        Self(value)
    }
}

pub trait Escalate<E> {
    fn escalate(self, location: ErrorInfo, specified: Option<E>) -> Report<E>;
}

impl<T, U> Escalate<U> for T
where
    T: Display + Debug,
    U: Decompose<T>,
{
    fn escalate(self, location: ErrorInfo, specified: Option<U>) -> Report<U> {
        let Decomposed {
            cause,
            mut trace,
            classified,
        } = U::decompose(self);

        trace.push(location);

        Report {
            inner: specified.unwrap_or(classified),
            cause,
            trace,
        }
    }
}

pub trait IntoReport: Sized {
    fn into_report(self) -> Report<Self>;

    fn into_report_with_error_info(self, error_info: ErrorInfo) -> Report<Self>;
}

impl<E> IntoReport for E
where
    E: Display + Debug,
{
    #[track_caller]
    fn into_report(self) -> Report<Self> {
        let location = Location::caller();

        let cause = ErrorCause {
            display: self.to_string(),
            debug: format!("{self:?}"),
            error_type_name: type_name::<E>(),
        };

        Report {
            inner: self,
            cause,
            trace: vec![ErrorInfo {
                error_type_name: type_name::<E>(),
                path: location.file(),
                fn_name: "<unknown>",
                line: location.line(),
                col: location.column(),
                expr: "<unknown>",
                tag: "<unknown>",
            }],
        }
    }

    fn into_report_with_error_info(self, error_info: ErrorInfo) -> Report<Self> {
        let cause = ErrorCause {
            display: self.to_string(),
            debug: format!("{self:?}"),
            error_type_name: type_name::<E>(),
        };

        Report {
            inner: self,
            cause,
            trace: vec![error_info],
        }
    }
}
