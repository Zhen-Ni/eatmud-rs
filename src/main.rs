use std::ops::RangeBounds;

use chrono::NaiveDate;
use eatmud::Fund;


fn main() {
    let a = 1..5;
    let c = 1..=5;
    let b = ..6;
    a.start;
    b.start_bound()
}
