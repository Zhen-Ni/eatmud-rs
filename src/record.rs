use chrono::NaiveDate;
use core::fmt;
use std::ops::Index;

use crate::utility::{irr, search_sorted};

pub trait RecordSlice {
    fn date(&self) -> NaiveDate;
    fn investment(&self) -> f64;
    fn present_value(&self) -> f64;
    fn comment(&self) -> &str;
    fn total_investment(&self) -> f64;
    fn profit(&self) -> f64;
}

#[derive(Debug, Clone)]
pub struct ConciseRecordSlice {
    date: NaiveDate,
    investment: f64,
    present_value: f64,
    comment: String,
    total_investment: f64,
    profit: f64,
}

#[derive(Debug, Clone)]
pub struct DetailedRecordSlice {
    date: NaiveDate,
    investment: f64,
    nav: f64,
    share: f64,
    comment: String,
    fee: f64,
    total_investment: f64,
    total_share: f64,
    present_value: f64,
    profit: f64,
}

impl DetailedRecordSlice {
    pub fn nav(&self) -> f64 {
        self.nav
    }

    pub fn share(&self) -> f64 {
        self.share
    }

    pub fn fee(&self) -> f64 {
        self.fee
    }

    pub fn total_share(&self) -> f64 {
        self.total_share
    }
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

#[derive(Debug, Clone)]
pub struct Record<Rs: RecordSlice> {
    pub name: String,
    pub code: String,
    pub comment: String,
    records: Vec<Rs>,
}

impl<Rs: RecordSlice> Record<Rs> {
    /// Create a new record.
    ///
    /// # Examples
    /// ```
    /// use eatmud::record::{Record, ConciseRecordSlice};
    /// let mut record = Record::<ConciseRecordSlice>::new("hs300", "123456");
    /// assert!(record.name() == "hs300");
    /// assert!(record.code() == "123456");
    /// assert!(record.records().is_empty());
    /// ```
    pub fn new(name: &str, code: &str) -> Self {
        Self {
            name: name.to_string(),
            code: code.to_string(),
            comment: String::new(),
            records: Vec::<Rs>::new(),
        }
    }

    pub fn new_comment(name: &str, code: &str, comment: &str) -> Self {
        Self {
            name: name.to_string(),
            code: code.to_string(),
            comment: comment.to_string(),
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

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Calculate internal rate of return (IRR) of the record.
    ///
    /// # Arguments
    ///
    /// * `start_date` - The start date for calculating IRR.
    /// * `end_date` - The end date for calculating IRR.
    /// * `start_value` - Value at the start date.
    /// * `end_value` - Value at the end date.
    /// * `start_idx` - The start index for the range of records used for calculation.
    /// * `end_idx` - The end index for the range of records used for calculation.
    /// * `x0` - The initial value for iteration.
    #[allow(clippy::too_many_arguments)]
    pub fn irr_direct(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        start_value: f64,
        end_value: f64,
        start_idx: usize,
        end_idx: usize,
        x0: f64, // initial guess for solving irr
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
        irr(&t, &x, end_value, x0).expect("fail to solve irr")
    }

    /// Calculate internal rate of return with default parameters.
    pub fn irr_naive(&self) -> f64 {
        self.irr(None, None, None, None, None)
    }

    /// Calculate internal rate of return (IRR) of the record.
    ///
    /// This is a wrapper for Record::irr_direct. The start index and
    /// end index are estimated automatically by searching the start
    /// date and end date in the records.
    ///
    /// # Arguments
    ///
    /// * `start_date` - The start date for calculating IRR.
    /// Defaults to the first date in the records.
    /// * `end_date` - The end date for calculating IRR.
    /// Defaults to the last date in the records.
    /// * `start_value` - Value at the start date.
    /// If None is given, it will be evaluated from the nearest date in the records.
    /// * `end_value` - Value at the end date.
    /// If None is given, it will be evaluated from the nearest date in the records.
    /// * `x0` - The initial value for iteration. Default to 0.0.
    pub fn irr(
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
        } else if (start_date - self[start_idx - 1].date()) < (self[start_idx].date() - start_date)
        {
            self[start_idx - 1].present_value()
        } else {
            let slice = &self[start_idx];
            slice.present_value() - slice.investment()
        });

        let end_value = end_value.unwrap_or(if end_idx == self.len() {
            self[self.len() - 1].present_value()
        } else if (end_date - self[end_idx - 1].date()) < (self[end_idx].date() - end_date) {
            self[end_idx - 1].present_value()
        } else {
            let slice = &self[end_idx];
            slice.present_value() - slice.investment()
        });
        self.irr_direct(
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
    /// let mut record = ConciseRecord::new("hs300", "123456");
    /// NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
    /// record.append(NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d")
    ///     .unwrap(), 10., 10., "initial investment");
    /// record.append(NaiveDate::parse_from_str("2024-02-01", "%Y-%m-%d")
    ///     .unwrap(), 10., 21., "investment 2");
    /// assert_eq!(record[1].total_investment(), 20.);
    /// assert_eq!(record[1].profit(), 1.);
    /// ```
    pub fn append(&mut self, date: NaiveDate, investment: f64, present_value: f64, comment: &str) {
        if !self.is_empty() && date < self[self.len() - 1].date() {
            panic!("date must be ordered");
        }
        let mut total_investment = investment;
        if !self.is_empty() {
            total_investment += self.records[self.len() - 1].total_investment();
        }
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
    /// let mut record = DetailedRecord::new_comment("hs300", "123456", "HS300 TEST");
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
        if !self.is_empty() && date < self[self.len() - 1].date() {
            panic!("date must be ordered");
        }
        let mut total_investment = investment;
        let mut total_share = share;
        if !self.is_empty() {
            total_investment += self.records[self.len() - 1].total_investment();
            total_share += self.records[self.len() - 1].total_share();
        }
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

impl From<&DetailedRecord> for ConciseRecord {
    fn from(detailed_record: &DetailedRecord) -> Self {
        let mut concise_record = ConciseRecord::new_comment(
            detailed_record.name(),
            detailed_record.code(),
            detailed_record.comment(),
        );
        concise_record.records = detailed_record
            .records
            .iter()
            .map(|s| ConciseRecordSlice {
                date: s.date(),
                investment: s.investment(),
                present_value: s.present_value(),
                comment: s.comment().to_string(),
                total_investment: s.total_investment(),
                profit: s.profit(),
            })
            .collect();
        concise_record
    }
}

impl From<DetailedRecord> for ConciseRecord {
    fn from(detailed_record: DetailedRecord) -> Self {
        let mut concise_record = ConciseRecord::new_comment(
            detailed_record.name(),
            detailed_record.code(),
            detailed_record.comment(),
        );
        concise_record.records = detailed_record
            .records
            .into_iter()
            .map(|s| ConciseRecordSlice {
                date: s.date(),
                investment: s.investment(),
                present_value: s.present_value(),
                total_investment: s.total_investment(),
                profit: s.profit(),
                comment: s.comment, // move this field to the end as move occurs
            })
            .collect();
        concise_record
    }
}

impl fmt::Display for ConciseRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "date        invest    present     comment    total       profit    "
        )?;
        for rs in self.records() {
            writeln!(
                f,
                "{:<}  {:<8.2}  {:<10.2}  {:<.10}  {:<10.2}  {:<10.2}",
                rs.date(),
                rs.investment(),
                rs.present_value(),
                rs.comment(),
                rs.total_investment(),
                rs.profit()
            )?;
        }
        fmt::Result::Ok(())
    }
}

impl fmt::Display for DetailedRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "date        invest    nav      share     comment  fee   tot_invest  tot_share   profit    "
        )?;
        for rs in self.records() {
            writeln!(
                f,
                "{:<}  {:<8.2}  {:<7.4}  {:<8.4}  {:<7}  {:<4.2}  {:<10.2}  {:<10.2}  {:<10.2}",
                rs.date(),
                rs.investment(),
                rs.nav,
                rs.share,
                rs.comment(),
                rs.fee,
                rs.total_investment(),
                rs.total_share,
                rs.profit()
            )?;
        }
        fmt::Result::Ok(())
    }
}

#[macro_export]
macro_rules! merge_records {
    ($record: expr) => {
        merge_records!($record, ConciseRecord::new("", ""))
    };
    ($record1: expr, $record2: expr) => {{
        use $crate::record::RecordSlice;
    let mut record = ConciseRecord::new("", "");
    let mut it1 = $record1.records().iter();
    let mut it2 = $record2.records().iter();
    let mut rs1 = it1.next();
    let mut rs2 = it2.next();
    // present value cache
    let mut pv1 = 0.0;
    let mut pv2 = 0.0;
    loop {
        let present_date = match (rs1, rs2) {
            (Some(s1), Some(s2)) => Ord::min(s1.date(), s2.date()),
            (Some(s1), None) => s1.date(),
            (None, Some(s2)) => s2.date(),
            (None, None) => break record,
        };
        let mut present_investment = 0.;
        let mut comments: Vec<&str> = Vec::new();
        let mut status1 = false;
        let mut status2 = false;
        while let Some(s) = rs1 {
            if s.date() != present_date {
                break;
            }
            pv1 = s.present_value();
            present_investment += s.investment();
            if !s.comment().is_empty() {
                comments.push(s.comment());
            }
            status1 = true;
            rs1 = it1.next();
        }
        while let Some(s) = rs2 {
            if s.date() != present_date {
                break;
            }
            pv2 = s.present_value();
            present_investment += s.investment();
            if !s.comment().is_empty() {
                comments.push(s.comment());
            }
            status2 = true;
            rs2 = it2.next();
        }
        if !(status1 && status2) {
            comments.push("estimated");
        }
        record.append(
            present_date,
            present_investment,
            pv1 + pv2,
            &comments.join("; "),
        );
    }}
    };
    ($r1: expr, $r2: expr, $($rs: expr), +) => {
        merge_records!(merge_records!(r1, r2), $($rs), +)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_record_from() {
        let mut record = DetailedRecord::new("hs300", "123456");
        record.append(
            NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap(),
            10.,
            1.,
            10.,
            "initial investment",
        );
        record.append(
            NaiveDate::parse_from_str("2024-02-01", "%Y-%m-%d").unwrap(),
            10.,
            1.2,
            8.3,
            "investment 2",
        );
        let record2 = ConciseRecord::from(&record);
        assert_eq!(record2[1].total_investment(), 20.);
        assert!((record2[1].profit() - 1.96).abs() < 1e-3);
    }

    #[test]
    fn test_merge() {
        let mut record1 = ConciseRecord::new("hs300", "123456");
        record1.append(
            NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap(),
            10.,
            10.,
            "investment 0",
        );
        record1.append(
            NaiveDate::parse_from_str("2024-02-01", "%Y-%m-%d").unwrap(),
            15.,
            25.,
            "investment 3",
        );

        let mut record2 = DetailedRecord::new("hs300", "123456");
        record2.append(
            NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap(),
            10.,
            1.,
            10.,
            "investment 1",
        );
        record2.append(
            NaiveDate::parse_from_str("2024-01-05", "%Y-%m-%d").unwrap(),
            20.,
            1.,
            20.,
            "investment 2",
        );
        record2.append(
            NaiveDate::parse_from_str("2024-02-05", "%Y-%m-%d").unwrap(),
            5.,
            1.,
            5.,
            "investment 4",
        );
        let merged = merge_records!(&record1, &record2);
        assert_eq!(merged.len(), 4);
        assert_eq!(merged.records()[3].total_investment(), 60.);
        assert_eq!(
            merged.records()[0].comment(),
            format!("investment 0; investment 1")
        );
        for i in 1..4 {
            assert_eq!(
                merged.records()[i].comment(),
                format!("investment {}; estimated", i + 1)
            );
        }
    }
}
