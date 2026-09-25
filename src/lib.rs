use std::{any::type_name, panic::Location};

use thiserror::Error;

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

pub trait Recognize<T> {
    fn recognize(error: &T) -> (Vec<ErrorInfo>, Self);
}

impl<T, U> Recognize<Report<T>> for U
where
    U: Recognize<T>,
{
    fn recognize(error: &Report<T>) -> (Vec<ErrorInfo>, Self) {
        let (_, s) = Self::recognize(&error.inner);

        (error.trace.clone(), s)
    }
}

#[derive(Debug)]
pub struct Report<E> {
    inner: E,
    trace: Vec<ErrorInfo>,
}

impl<E> Report<E> {
    pub fn handle(self) -> (E, Vec<ErrorInfo>) {
        (self.inner, self.trace)
    }
}

pub trait IntoNewReport<E> {
    fn into_new_report(&self, location: ErrorInfo, specified: Option<E>) -> Report<E>;
}

impl<T, U> IntoNewReport<U> for T
where
    U: Recognize<T>,
{
    fn into_new_report(&self, location: ErrorInfo, specified: Option<U>) -> Report<U> {
        let (mut trace, default) = U::recognize(self);
        trace.push(location);

        Report {
            inner: specified.unwrap_or(default),
            trace,
        }
    }
}

/// hooq::skip する場合にReport化するためのメソッド
pub trait IntoReport: Sized {
    fn into_report(self) -> Report<Self>;
}

impl<E> IntoReport for E {
    #[track_caller]
    fn into_report(self) -> Report<Self> {
        let location = Location::caller();

        Report {
            inner: self,
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
}
