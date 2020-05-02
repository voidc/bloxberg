use std::ops::Range;
use std::cmp::Ordering;

pub fn unit() -> () {}

pub fn id<T>(t: T) -> T { t }

pub fn run<R, F: FnOnce() -> R>(f: F) -> R { f() }

pub fn cmp_range<T: Ord>(x: T, r: Range<T>) -> Ordering {
    if x < r.start {
        Ordering::Less
    } else if x >= r.end {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

pub trait UtilExt {
    fn apply<F, R>(self, f: F) -> R
        where
            Self: Sized,
            F: FnOnce(Self) -> R;

    fn also<F>(&self, f: F) -> &Self
        where
            F: FnOnce(&Self) -> ();
}

impl<T> UtilExt for T {
    fn apply<F, R>(self, f: F) -> R
        where
            Self: Sized,
            F: FnOnce(Self) -> R,
    {
        f(self)
    }

    fn also<F>(&self, f: F) -> &Self
        where
            F: FnOnce(&Self) -> (),
    {
        f(&self);
        &self
    }
}