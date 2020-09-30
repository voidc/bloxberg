use std::cmp::Ordering;
use std::ops::Range;

pub fn cmp_range<T: Ord>(x: T, r: Range<T>) -> Ordering {
    if x < r.start {
        Ordering::Less
    } else if x >= r.end {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}
