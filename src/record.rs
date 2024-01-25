use std::ops::Index;

use chrono::NaiveDate;

use crate::utility::{iir, search_sorted};

pub trait RecordSlice {
    fn date(&self) -> NaiveDate;
    fn investment(&self) -> f64;
    fn present_value(&self) -> f64;
    fn comment(&self) -> &str;
    fn total_investment(&self) -> f64;
    fn profit(&self) -> f64;
}

pub struct ConciseRecordSlice {
    pub date: NaiveDate,
    pub investment: f64,
    pub present_value: f64,
    pub comment: String,
    pub total_investment: f64,
    pub profit: f64,
}

pub struct DetailedRecordSlice {
    pub date: NaiveDate,
    pub investment: f64,
    pub nav: f64,
    pub share: f64,
    pub comment: String,
    pub fee: f64,
    pub total_investment: f64,
    pub total_share: f64,
    pub present_value: f64,
    pub profit: f64,
}

impl RecordSlice for ConciseRecordSlice {
    fn date(&self) -> NaiveDate {
        self.date
    }
    fn investment(&self) -> f64 {
        self.investment
    }
    fn present_value(&self) -> f64 {
        self.present_value
    }
    fn comment(&self) -> &str {
        &self.comment
    }
    fn total_investment(&self) -> f64 {
        self.total_investment
    }
    fn profit(&self) -> f64 {
        self.profit
    }
}

impl RecordSlice for DetailedRecordSlice {
    fn date(&self) -> NaiveDate {
        self.date
    }
    fn investment(&self) -> f64 {
        self.investment
    }
    fn present_value(&self) -> f64 {
        self.present_value
    }
    fn comment(&self) -> &str {
        &self.comment
    }
    fn total_investment(&self) -> f64 {
        self.total_investment
    }
    fn profit(&self) -> f64 {
        self.profit
    }
}

pub struct Record<Rs: RecordSlice> {
    name: String,
    code: String,
    comment: String,
    records: Vec<Rs>,
}

impl<Rs: RecordSlice> Record<Rs> {
    /// Create a new record.
    ///
    /// # Examples
    /// ```
    /// use eatmud::record::{Record, ConciseRecordSlice};
    /// let mut record = Record::<ConciseRecordSlice>::new("hs300", "123456", None);
    /// assert!(record.name() == "hs300");
    /// assert!(record.code() == "123456");
    /// assert!(record.records().is_empty());
    /// ```
    pub fn new(name: &str, code: &str, comment: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            code: code.to_string(),
            comment: comment.unwrap_or("").to_string(),
            records: Vec::<Rs>::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn comment(&self) -> &str {
        &self.comment
    }

    pub fn records(&self) -> &[Rs] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Calculate internal rate of return (IRR) of the record.
    ///
    /// # Arguments
    ///
    /// * `start_date` - The start date for calculating IIR.
    /// * `end_date` - The end date for calculating IIR.
    /// * `start_value` - Value at the start date.
    /// * `end_value` - Value at the end date.
    /// * `start_idx` - The start index for the range of records used for calculation.
    /// * `end_idx` - The end index for the range of records used for calculation.
    /// * `x0` - The initial value for iteration.
    pub fn iir_direct(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        start_value: f64,
        end_value: f64,
        start_idx: usize,
        end_idx: usize,
        x0: f64, // initial guess for solving iir
    ) -> f64 {
        let mut t = vec![start_date];
        t.extend(self.records[start_idx..end_idx].iter().map(|rs| rs.date()));
        let t: Vec<_> = t
            .iter()
            .map(|ti| (end_date - *ti).num_days() as f64)
            .collect();
        let mut x = vec![start_value];
        x.extend(
            self.records[start_idx..end_idx]
                .iter()
                .map(|rs| rs.investment()),
        );
        iir(&t, &x, end_value, x0).unwrap()
    }

    /// Calculate internal rate of return (IRR) of the record.
    ///
    /// This is a wrapper for Record::iir_direct. The start index and
    /// end index are estimated automatically by searching the start
    /// date and end date in the records.
    ///
    /// # Arguments
    ///
    /// * `start_date` - The start date for calculating IIR.
    /// Defaults to the first date in the records.
    /// * `end_date` - The end date for calculating IIR.
    /// Defaults to the last date in the records.
    /// * `start_value` - Value at the start date.
    /// If None is given, it will be evaluated from the nearest date in the records.
    /// * `end_value` - Value at the end date.
    /// If None is given, it will be evaluated from the nearest date in the records.
    /// * `x0` - The initial value for iteration. Default to 0.0.
    pub fn iir(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        start_value: Option<f64>,
        end_value: Option<f64>,
        x0: Option<f64>,
    ) -> f64 {
        let start_date = start_date.unwrap_or(self.records[0].date());
        let end_date = end_date.unwrap_or(self.records[self.len() - 1].date());
        let start_idx = search_sorted(self.records(), &start_date, |s| s.date(), None);
        let end_idx = search_sorted(self.records(), &end_date, |s| s.date(), None);

        let start_value = start_value.unwrap_or(if start_idx == 0 {
            0.0
        } else {
            if (start_date - self[start_idx - 1].date()) < (self[start_idx].date() - start_date) {
                self[start_idx - 1].present_value()
            } else {
                let slice = &self[start_idx];
                slice.present_value() - slice.investment()
            }
        });

        let end_value = end_value.unwrap_or(if end_idx == self.len() {
            self[self.len() - 1].present_value()
        } else {
            if (end_date - self[end_idx - 1].date()) < (self[end_idx].date() - end_date) {
                self[end_idx - 1].present_value()
            } else {
                let slice = &self[end_idx];
                slice.present_value() - slice.investment()
            }
        });
        self.iir_direct(
            start_date,
            end_date,
            start_value,
            end_value,
            start_idx,
            end_idx,
            x0.unwrap_or_default(),
        )
    }
}

impl<Rs: RecordSlice> Index<usize> for Record<Rs> {
    type Output = Rs;
    fn index(&self, index: usize) -> &Rs {
        &self.records[index]
    }
}

impl Record<ConciseRecordSlice> {
    /// Appends new data to the end of the record.
    ///
    /// # Examples
    /// ```
    /// use chrono::NaiveDate;
    /// use eatmud::{prelude::*, ConciseRecord};
    /// let mut record = ConciseRecord::new("hs300", "123456", None);
    /// NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
    /// record.append(NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d")
    ///     .unwrap(), 10., 10., "initial investment");
    /// record.append(NaiveDate::parse_from_str("2024-02-01", "%Y-%m-%d")
    ///     .unwrap(), 10., 21., "investment 2");
    /// assert_eq!(record[1].total_investment(), 20.);
    /// assert_eq!(record[1].profit(), 1.);
    /// ```
    pub fn append(&mut self, date: NaiveDate, investment: f64, present_value: f64, comment: &str) {
        if self.len() > 0 && date < self[self.len() - 1].date() {
            panic!("date must be ordered");
        }
        let total_investment =
            self.records.iter().map(|rs| rs.investment).sum::<f64>() + investment;
        self.records.push(ConciseRecordSlice {
            date,
            investment,
            present_value,
            comment: comment.to_string(),
            total_investment,
            profit: present_value - total_investment,
        })
    }
}

impl Record<DetailedRecordSlice> {
    /// Appends new data to the end of the record.
    ///
    /// # Examples
    /// ```
    /// use chrono::NaiveDate;
    /// use eatmud::{prelude::*, DetailedRecord};
    /// let mut record = DetailedRecord::new("hs300", "123456", None);
    /// NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
    /// record.append(NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d")
    ///     .unwrap(), 10., 1., 10., "initial investment");
    /// record.append(NaiveDate::parse_from_str("2024-02-01", "%Y-%m-%d")
    ///     .unwrap(), 10., 1.2, 8.3, "investment 2");
    /// assert_eq!(record[1].total_investment(), 20.);
    /// assert!((record[1].profit() - 1.96).abs() < 1e-3);
    /// ```
    pub fn append(
        &mut self,
        date: NaiveDate,
        investment: f64,
        nav: f64,
        share: f64,
        comment: &str,
    ) {
        if self.len() > 0 && date < self[self.len() - 1].date() {
            panic!("date must be ordered");
        }
        let total_investment =
            self.records.iter().map(|rs| rs.investment).sum::<f64>() + investment;
        let total_share = self.records.iter().map(|rs| rs.share).sum::<f64>() + share;
        let present_value = nav * total_share;
        self.records.push(DetailedRecordSlice {
            date,
            investment,
            nav,
            share,
            comment: comment.to_string(),
            fee: investment - nav * share,
            total_investment,
            total_share,
            present_value,
            profit: present_value - total_investment,
        })
    }
}

pub type ConciseRecord = Record<ConciseRecordSlice>;
pub type DetailedRecord = Record<DetailedRecordSlice>;
