#![feature(try_as_dyn)]

use std::any::try_as_dyn_mut;
use std::panic::Location;

use thiserror::Error;

#[derive(Error, Debug)]
#[error("{path}:{fn_name}:{line}:{col}")]
pub struct LocationInfo {
    pub path: &'static str,
    pub fn_name: &'static str,
    pub line: u32,
    pub col: u32,
    pub expr: &'static str,
    pub tag: &'static str,
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

trait CanBeConvertedAnotherReport<U> {
    fn into_new_report_inner(
        &mut self,
        location: LocationInfo,
    ) -> (&dyn RecognizedAs<U>, Vec<LocationInfo>);
}

impl<E, U> CanBeConvertedAnotherReport<U> for Report<E>
where
    E: RecognizedAs<U>,
{
    fn into_new_report_inner(
        &mut self,
        location: LocationInfo,
    ) -> (&dyn RecognizedAs<U>, Vec<LocationInfo>) {
        let Report { inner, trace } = self;
        let mut trace: Vec<_> = trace.drain(..).collect();

        trace.push(location);

        (inner, trace)
    }
}

pub trait IntoNewReport<E> {
    fn into_new_report(&mut self, location: LocationInfo) -> Report<E>;
}

impl<T, U> IntoNewReport<U> for T
where
    T: RecognizedAs<U>,
{
    fn into_new_report(&mut self, location: LocationInfo) -> Report<U> {
        if let Some(r) = try_as_dyn_mut::<_, dyn CanBeConvertedAnotherReport<U>>(self) {
            let (inner, trace) = r.into_new_report_inner(location);

            Report {
                inner: inner.recognized_as(),
                trace,
            }
        } else {
            Report {
                inner: self.recognized_as(),
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

pub trait RecognizedAs<T> {
    fn recognized_as(&self) -> T;
}

impl<T> RecognizedAs<T> for T
where
    T: Clone,
{
    fn recognized_as(&self) -> T {
        self.clone()
    }
}

impl<T, U> RecognizedAs<T> for Report<U>
where
    U: RecognizedAs<T>,
{
    fn recognized_as(&self) -> T {
        self.inner.recognized_as()
    }
}
