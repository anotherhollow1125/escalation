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

pub struct Report<E> {
    inner: E,
    trace: Vec<LocationInfo>,
}

impl<E> Report<E> {
    pub fn handle(self) -> (E, Vec<LocationInfo>) {
        (self.inner, self.trace)
    }
}

pub trait IntoNewReport<E> {
    fn into_new_report(&mut self, error: E, location: LocationInfo) -> Report<E>;
}

impl<E, U> IntoNewReport<E> for Report<U> {
    fn into_new_report(&mut self, error: E, location: LocationInfo) -> Report<E> {
        let mut trace: Vec<_> = self.trace.drain(..).collect();

        trace.push(location);

        Report {
            inner: error,
            trace,
        }
    }
}

impl<E> From<E> for Report<E> {
    #[track_caller]
    fn from(error: E) -> Self {
        let location = Location::caller();

        Report {
            inner: error,
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

impl<E> From<(E, LocationInfo)> for Report<E> {
    fn from((error, info): (E, LocationInfo)) -> Self {
        Report {
            inner: error,
            trace: vec![info],
        }
    }
}
