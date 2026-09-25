#![feature(try_as_dyn)]

use std::any::try_as_dyn_mut;
use std::panic::Location;

use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[error("{path}:{fn_name}:{line}:{col}")]
pub struct LocationInfo {
    pub path: &'static str,
    pub fn_name: &'static str,
    pub line: u32,
    pub col: u32,
    pub expr: &'static str,
    pub tag: &'static str,
}

pub trait Recognize<T> {
    fn recognize(error: &T) -> Self;
}

impl<T, U> Recognize<Report<T>> for U
where
    U: Recognize<T>,
{
    fn recognize(error: &Report<T>) -> Self {
        Self::recognize(&error.inner)
    }
}

#[derive(Debug)]
pub struct Report<E> {
    inner: E,
    trace: Vec<LocationInfo>,
}

impl<E> Report<E> {
    pub fn handle(self) -> (E, Vec<LocationInfo>) {
        (self.inner, self.trace)
    }
}

trait IntoTrace {
    fn into_trace(&mut self) -> Option<Vec<LocationInfo>>;
}

impl<T> IntoTrace for Option<&Report<T>> {
    fn into_trace(&mut self) -> Option<Vec<LocationInfo>> {
        if let Some(s) = self.take() {
            let Report { inner: _, trace } = s;

            Some(trace.clone())
        } else {
            None
        }
    }
}

pub trait IntoNewReport<E> {
    fn into_new_report(&self, location: LocationInfo, specified: Option<E>) -> Report<E>;
}

impl<T, U> IntoNewReport<U> for T
where
    U: Recognize<T>,
{
    fn into_new_report(&self, location: LocationInfo, specified: Option<U>) -> Report<U> {
        let default = U::recognize(self);
        let mut wrapped = Some(self);
        if let Some(r) = try_as_dyn_mut::<_, dyn IntoTrace>(&mut wrapped) {
            let mut trace = r.into_trace().unwrap();
            trace.push(location);

            Report {
                inner: specified.unwrap_or(default),
                trace,
            }
        } else {
            Report {
                inner: specified.unwrap_or(default),
                trace: vec![location],
            }
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
            trace: vec![LocationInfo {
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
