mod common;
pub mod data;
pub mod prelude;
pub mod record;
pub mod transaction;
pub mod utility;
pub mod strategy;

pub use chrono::{Duration, NaiveDate};
pub use data::{read_gta, Fund, Stock};
pub use ndarray::{Array1, Array2, ArrayView1, ArrayView2};
pub use prelude::*;
pub use record::{ConciseRecord, DetailedRecord};
pub use transaction::{Transaction, TransactionIterator, Weekday};
pub use utility::{SIDE, DAYS_PER_YEAR};
