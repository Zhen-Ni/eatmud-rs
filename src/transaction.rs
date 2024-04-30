use crate::{
    common::warning, merge_records, utility::search_sorted, ConciseRecord, DataSlice,
    DetailedRecord, Fund,
};
use chrono::{Datelike, Duration, NaiveDate};
use ndarray::{s, Array1, Array2, ArrayView1, ArrayView2, AssignElem, Axis, ShapeBuilder};

#[derive(Debug)]
pub struct TransactionError(&'static str);

impl std::fmt::Display for TransactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Transaction Error: {}", self.0)
    }
}

impl std::error::Error for TransactionError {}

pub struct Transaction {
    names: Vec<String>,
    codes: Vec<String>,
    date: Vec<NaiveDate>,
    navs: Array2<f64>, // net asset value
    start_date: NaiveDate,
    end_date: NaiveDate,
}

impl Transaction {
    /// Create a new Transaction object.
    ///
    /// # Examples
    /// ```
    /// use chrono::NaiveDate;
    /// use eatmud::prelude::*;
    /// use eatmud::{read_gta, Transaction, Fund};
    /// let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
    /// let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
    /// let start_date = NaiveDate::parse_from_str("2020-01-01", "%Y-%m-%d").unwrap();
    /// let t = Transaction::new(&[&hs300, &gz2000], Some(start_date), None);
    /// ```
    pub fn new(
        funds: &[&Fund],
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Transaction {
        let names: Vec<_> = funds.iter().map(|d| d.name().to_string()).collect();
        let codes: Vec<_> = funds.iter().map(|d| d.code().to_string()).collect();

        let start_date = start_date.unwrap_or(funds.iter().map(|d| d[0].date()).max().unwrap());
        let end_date = end_date.unwrap_or(
            funds.iter().map(|d| d[d.len() - 1].date()).min().unwrap() + chrono::Days::new(1),
        );
        let mut date = Vec::new();
        let mut navs = Vec::with_capacity(date.len() * funds.len());
        for (i, d) in funds.iter().enumerate() {
            let beg = search_sorted(d.data(), &start_date, |rs| rs.date(), None);
            let end = search_sorted(&d.data()[beg..], &end_date, |rs| rs.date(), None) + beg;
            if i == 0 {
                date.extend(d.data()[beg..end].iter().map(|rs| rs.date()));
            }
            // Check whether all dates in data are the same.
            else if date
                .iter()
                .copied()
                .ne(d.data()[beg..end].iter().map(|rs| rs.date()))
            {
                panic!("data not match")
            }

            navs.extend(d.data()[beg..end].iter().map(|rs| rs.value()));
        }
        let navs = Array2::from_shape_vec((date.len(), funds.len()).strides((1, date.len())), navs)
            .unwrap();

        Transaction {
            names,
            codes,
            date,
            navs,
            start_date,
            end_date,
        }
    }

    pub fn from_funds(funds: &[&Fund]) -> Self {
        Self::new(funds, None, None)
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn codes(&self) -> &[String] {
        &self.codes
    }

    pub fn ndays(&self) -> usize {
        self.date.len()
    }

    pub fn nfunds(&self) -> usize {
        self.names.len()
    }

    pub fn start_date(&self) -> NaiveDate {
        self.start_date
    }

    pub fn end_date(&self) -> NaiveDate {
        self.end_date
    }

    pub fn date(&self) -> &[NaiveDate] {
        &self.date
    }

    pub fn navs(&self) -> &Array2<f64> {
        &self.navs
    }

    pub fn iter(&self, save_log: bool, save_record: bool) -> TransactionIterator {
        TransactionIterator::new(self, save_log, save_record)
    }
}

/// Records transaction operations during one iteration.
///
/// This is used by TransactionIterator to record transaction
/// operations and its fields are not visible to user of
/// TransactionIterator.
///
/// This struct is necessary despite the existance of IterStatus, as
/// we do not want the user to observe changes in the status of the
/// TransactionIterator when they are making transactions.
struct IterBuffer {
    cash: f64,
    shares: Vec<f64>,
}

impl IterBuffer {
    fn reset(&mut self) {
        self.cash = 0.;
        self.shares.fill(0.);
    }
}

/// Current transation status.
///
/// This struct is hold by TransactionIterator and has two fields:
/// cash and shares. `index` is the current index of the transaction
/// iterator, ranging from 0 to `ndate`. The TransactionIterator
/// reaches end when `index` equals to `ndate`. `cash` is the current
/// cash at the beginning of the day. `shares` is a list of floats
/// indicating the shares of the funds at the beginning of the day.
struct IterStatus {
    cash: f64,
    shares: Vec<f64>,
}

/// Log of history cash and shares.
///
/// This is an optional struct for TransactionIterator.
struct IterLog {
    cash: Array1<f64>,
    shares: Array2<f64>,
}

/// Struct for storing records.
///
/// This is an optional struct for TransactionIterator.
struct IterRecord {
    // Investments of each fund is necessary for records.
    investments: Vec<f64>,
    cash_comment_buffer: String,
    fund_comment_buffer: Vec<String>,
    cash_record: ConciseRecord,
    fund_records: Vec<DetailedRecord>,
}

impl IterRecord {
    fn reset_buffer(&mut self) {
        self.investments.fill(0.);
        self.cash_comment_buffer.clear();
        for item in &mut self.fund_comment_buffer {
            item.clear();
        }
    }
}

pub use chrono::Weekday;

/// Transact over a given `Transaction` object.
pub struct TransactionIterator<'a> {
    transaction: &'a Transaction,
    index: usize,
    iter_buffer: IterBuffer,
    iter_status: IterStatus,
    iter_log: Option<IterLog>,
    iter_record: Option<IterRecord>,
}

impl<'a> TransactionIterator<'a> {
    pub(crate) fn new(trans: &'a Transaction, save_log: bool, save_record: bool) -> Self {
        let ndays = trans.ndays();
        let nfunds = trans.nfunds();
        TransactionIterator {
            transaction: trans,
            index: 0,
            iter_buffer: IterBuffer {
                cash: 0.,
                shares: vec![0.0; nfunds],
            },
            iter_status: IterStatus {
                cash: 0.,
                shares: vec![0.0; nfunds],
            },
            iter_log: if save_log {
                Some(IterLog {
                    cash: Array1::zeros(ndays),
                    shares: Array2::zeros((ndays, nfunds)),
                })
            } else {
                None
            },
            iter_record: if save_record {
                Some(IterRecord {
                    investments: vec![0.0; nfunds],
                    cash_comment_buffer: String::new(),
                    fund_comment_buffer: vec!["".to_string(); nfunds],
                    cash_record: ConciseRecord::new("cash", ""),
                    fund_records: Vec::from_iter(
                        trans
                            .names
                            .iter()
                            .zip(trans.codes.iter())
                            .map(|(name, code)| DetailedRecord::new(name, code)),
                    ),
                })
            } else {
                None
            },
        }
    }

    #[inline]
    fn is_finished(&self) -> bool {
        self.index == self.ndays()
    }

    #[inline]
    fn assert_not_finished(&self) -> Result<(), TransactionError> {
        if self.is_finished() {
            Err(TransactionError("transaction iteration reaches the end"))
        } else {
            Ok(())
        }
    }

    pub fn nfunds(&self) -> usize {
        self.transaction.nfunds()
    }

    pub fn ndays(&self) -> usize {
        self.transaction.ndays()
    }

    pub fn today(&self) -> NaiveDate {
        if self.is_finished() {
            self.transaction.date[self.index - 1]
        } else {
            self.transaction.date[self.index]
        }
    }

    /// Cash at the *beginning* of the day.
    pub fn cash(&self) -> f64 {
        self.iter_status.cash
    }

    /// Share at the *beginning* of the day.
    pub fn share(&self, idx: usize) -> f64 {
        self.iter_status.shares[idx]
    }

    /// Asset of specfied fund id at the *beginning* of the day.
    pub fn fund_asset(&self, idx: usize) -> f64 {
        if self.index == 0 {
            0.
        } else {
            self.share(idx) * self.transaction.navs[[self.index - 1, idx]]
        }
    }

    /// Total asset at the *beginning* of the day.
    pub fn asset(&self) -> f64 {
        if self.index == 0 {
            0.
        } else {
            self.iter_status
                .shares
                .iter()
                .zip(self.transaction.navs().row(self.index - 1))
                .map(|(x, y)| x * y)
                .sum::<f64>()
                + self.cash()
        }
    }

    /// Sequence of dates have iterated.
    pub fn dates(&self) -> &[NaiveDate] {
        &self.transaction.date[..self.index]
    }

    /// A 2-d array of NAVs in history.
    pub fn navs(&self) -> ArrayView2<f64> {
        self.transaction.navs.slice(s![..self.index, ..])
    }

    /// Log of cash.
    pub fn cash_log(&self) -> Option<ArrayView1<f64>> {
        Some(self.iter_log.as_ref()?.cash.slice(s![..self.index]))
    }

    /// Log of shares
    pub fn share_log(&self, idx: usize) -> Option<ArrayView1<f64>> {
        Some(self.iter_log.as_ref()?.shares.slice(s![..self.index, idx]))
    }

    /// Log of asset of specified fund id.
    pub fn fund_asset_log(&self, idx: usize) -> Option<Array1<f64>> {
        Some(
            &self.iter_log.as_ref()?.shares.slice(s![..self.index, idx]) * &self.navs().column(idx),
        )
    }

    /// Log of total asset.
    pub fn asset_log(&self) -> Option<Array1<f64>> {
        Some(
            (&self.iter_log.as_ref()?.shares.slice(s![..self.index, ..]) * &self.navs())
                .sum_axis(Axis(1))
                + self.cash_log()?,
        )
    }

    pub fn inflow(&mut self, amount: f64) -> Result<&mut Self, TransactionError> {
        self.assert_not_finished()?;
        self.iter_buffer.cash += amount;
        Ok(self)
    }

    pub fn inflow_comment(
        &mut self,
        amount: f64,
        comment: &str,
    ) -> Result<&mut Self, TransactionError> {
        self.inflow(amount)?;
        if let Some(ref mut record) = self.iter_record {
            if !record.cash_comment_buffer.is_empty() {
                record.cash_comment_buffer.push_str("; ");
                record.cash_comment_buffer.push_str(comment);
            }
        }
        Ok(self)
    }

    pub fn buy(
        &mut self,
        fundid: usize,
        investment: f64,
        fee: f64,
    ) -> Result<&mut Self, TransactionError> {
        self.assert_not_finished()?;
        self.iter_buffer.cash -= investment;

        self.iter_buffer.shares[fundid] +=
            (investment - fee) / self.transaction.navs[[self.index, fundid]];
        if let Some(ref mut record) = self.iter_record {
            record.investments[fundid] += investment;
        }
        Ok(self)
    }

    pub fn buy_comment(
        &mut self,
        fundid: usize,
        investment: f64,
        fee: f64,
        comment: &str,
    ) -> Result<&mut Self, TransactionError> {
        self.buy(fundid, investment, fee)?;
        if let Some(ref mut record) = self.iter_record {
            if !record.fund_comment_buffer[fundid].is_empty() {
                record.fund_comment_buffer[fundid].push_str("; ");
                record.fund_comment_buffer[fundid].push_str(comment);
            }
        }
        Ok(self)
    }

    pub fn sell(
        &mut self,
        fundid: usize,
        share: f64,
        fee: f64,
    ) -> Result<&mut Self, TransactionError> {
        self.assert_not_finished()?;
        let income = share * self.transaction.navs[[self.index, fundid]] - fee;
        self.iter_buffer.cash += income;
        self.iter_buffer.shares[fundid] -= share;
        if let Some(ref mut record) = self.iter_record {
            record.investments[fundid] -= income;
        }
        Ok(self)
    }

    pub fn sell_comment(
        &mut self,
        fundid: usize,
        share: f64,
        fee: f64,
        comment: &str,
    ) -> Result<&mut Self, TransactionError> {
        self.sell(fundid, share, fee)?;
        if let Some(ref mut record) = self.iter_record {
            if !record.fund_comment_buffer[fundid].is_empty() {
                record.fund_comment_buffer[fundid].push_str("; ");
                record.fund_comment_buffer[fundid].push_str(comment);
            }
        }
        Ok(self)
    }

    /// Finish one day's transaction.
    ///
    /// Calling this method updates cash and shares in `iter_status`
    /// by data recorded in `iter_buffer`. This method should only be
    /// called by `flush_and_step`.
    ///
    /// Note that this method should not be called when the iteration
    /// has finished.
    fn flush(&mut self) {
        self.iter_status.cash += self.iter_buffer.cash;
        for (i, s) in self.iter_buffer.shares.iter().enumerate() {
            self.iter_status.shares[i] += s;
        }

        if let Some(ref mut record) = self.iter_record {
            let today = self.transaction.date[self.index];
            record.cash_record.append(
                today,
                self.iter_buffer.cash,
                self.iter_status.cash,
                &record.cash_comment_buffer,
            );
            for (i, r) in record.fund_records.iter_mut().enumerate() {
                r.append(
                    today,
                    record.investments[i],
                    self.transaction.navs()[[self.index, i]],
                    self.iter_buffer.shares[i],
                    &record.fund_comment_buffer[i],
                );
            }
            record.reset_buffer();
        }

        self.iter_buffer.reset();
    }

    /// Iterate `n` days.
    ///
    /// This method increase iter_status.index to point the iterator
    /// to the following `n` transaction d/// If _iter_log is not
    /// None, it also fills its values of the corresponding days. This
    /// method should only be called by `_flush_and_step`.
    ///
    /// Note that this method should not be called when the iteration
    /// has finished.
    fn step(&mut self, n: usize) {
        let index = usize::min(self.index + n, self.transaction.ndays());
        if let Some(ref mut log) = self.iter_log {
            log.cash
                .slice_mut(s![self.index..index])
                .fill(self.iter_status.cash);
            for i in self.index..index {
                for j in 0..self.transaction.nfunds() {
                    unsafe {
                        log.shares
                            .uget_mut((i, j))
                            .assign_elem(*self.iter_status.shares.get_unchecked(j))
                    };
                }
            }
        }
        self.index = index;
    }

    ///Step `n` days and return whether the iteration reaches the end.
    fn flush_and_step(&mut self, n: usize) -> Option<()> {
        if self.is_finished() {
            return None;
        }
        if n == 0 {
            warning!(
                "Step to the same date is would refresh iter_buffer, which affects self.present_*."
            )
        }
        self.flush();
        self.step(n);
        if self.is_finished() {
            None
        } else {
            Some(())
        }
    }

    /// Step to the next day and returns whether it has reached the end.
    pub fn next_day(&mut self) -> Option<&mut Self> {
        self.flush_and_step(1)?;
        Some(self)
    }

    /// Step to next `weekday`, return if the iteration reaches the end.

    /// # Arguments
    ///
    /// * `weekday`/// If not given, it will de derived from `today`.
    pub fn next_weekday(&mut self, weekday: Option<Weekday>) -> Option<&mut Self> {
        let weekday = weekday.unwrap_or(self.today().weekday());
        let mut n = self.transaction.date.len() - self.index;
        for (i, day) in self.transaction.date[self.index..].iter().enumerate() {
            if i == 0 {
                continue;
            }
            if weekday == day.weekday() {
                n = i;
                break;
            }
        }
        self.flush_and_step(n)?;
        Some(self)
    }

    /// Step to user defined date.
    pub fn goto(&mut self, date: NaiveDate) -> Option<&mut Self> {
        let n = search_sorted(
            &self.transaction.date[usize::min(self.index, self.transaction.date.len())..],
            &date,
            |d| *d,
            None,
        );
        self.flush_and_step(n)?;
        Some(self)
    }

    /// Step to given day in the next month.
    ///
    /// If the day specified is not a transaction day, the transaction
    /// following the given date will be stepped to. It `day` is None,
    /// the month day of today will be the default for `day`. Note
    /// that `day` begins from 1.
    pub fn next_month(&mut self, day: Option<u32>) -> Option<&mut Self> {
        let day = day.unwrap_or(self.today().day());
        let mut year = self.today().year();
        let mut month = self.today().month() + 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
        let date = if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            date
        } else {
            let date =
                NaiveDate::from_ymd_opt(year, month, 1).unwrap() + Duration::days((day - 1) as i64);
            warning!(
                "day is out of range for {}-{}-{}, offset to {}",
                year,
                month,
                day,
                date
            );
            date
        };

        let n = search_sorted(
            &self.transaction.date[usize::min(self.index + 1, self.transaction.date.len())
                ..usize::min(self.index + 61, self.transaction.date.len())],
            &date,
            |d| *d,
            None,
        ) + 1;
        self.flush_and_step(n)?;
        Some(self)
    }

    pub fn cash_record(&self) -> Option<&ConciseRecord> {
        if let Some(ref record) = self.iter_record {
            Some(&record.cash_record)
        } else {
            None
        }
    }

    pub fn fund_record(&self, idx: usize) -> Option<&DetailedRecord> {
        if let Some(ref record) = self.iter_record {
            Some(&record.fund_records[idx])
        } else {
            None
        }
    }

    pub fn record(&self) -> Option<ConciseRecord> {
        if let Some(ref record) = self.iter_record {
            let res = ConciseRecord::new("Combined Record", "");
            let mut res = merge_records!(&res, &record.cash_record);
            for rec in record.fund_records.iter() {
                res = merge_records!(&res, rec)
            }
            Some(res)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_transaction_new() {
        use crate::read_gta;
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("2020-01-01", "%Y-%m-%d").unwrap();
        let t = Transaction::new(&[&hs300, &gz2000], Some(start_date), None);
        assert!((t.navs()[[0, 0]] - 4152.24).abs() < 1e-3);
        assert!((t.navs()[[1, 0]] - 4144.97).abs() < 1e-3);
        assert!((t.navs()[[0, 1]] - 6262.91).abs() < 1e-3);
        assert!((t.navs()[[1, 1]] - 6299.53).abs() < 1e-3);
    }

    /// Test iter `next_day`.
    #[test]
    fn test_transaction1() {
        use crate::read_gta;
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("2020-01-01", "%Y-%m-%d").unwrap();
        let t = Transaction::new(&[&hs300, &gz2000], Some(start_date), None);
        let mut it = t.iter(false, false);
        let mut idx = 0;
        while let Some(_) = it.next_day() {
            it.inflow(1.0).unwrap();
            assert!(it.cash() == idx as f64);
            assert!(it.asset() == idx as f64);
            idx += 1;
        }
        assert!(it.cash() > 980.)
    }

    /// Test iter `next_weekday`.
    #[test]
    fn test_transaction2() {
        use crate::read_gta;
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
        let end_date = NaiveDate::parse_from_str("2024-01-20", "%Y-%m-%d").unwrap();
        let t = Transaction::new(&[&hs300, &gz2000], Some(start_date), Some(end_date));
        let mut it = t.iter(false, false);
        it.inflow(100.).unwrap();
        assert_eq!(it.asset(), 0.);
        while let Some(_) = it.next_weekday(Some(Weekday::Wed)) {
            assert!(it.asset() > 90.);
            assert!(it.asset() < 110.);
            it.buy(0, 10., 0.).unwrap();
            it.buy(1, 10., 0.).unwrap();
        }
        assert!(it.cash() == 40.);
        assert!((it.share(0) - 0.009108).abs() < 1e-6);
    }

    /// Test iter `next_month`.
    #[test]
    fn test_transaction3() {
        use crate::read_gta;
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("2023-11-01", "%Y-%m-%d").unwrap();
        let end_date = NaiveDate::parse_from_str("2024-01-22", "%Y-%m-%d").unwrap();
        let t = Transaction::new(&[&hs300, &gz2000], Some(start_date), Some(end_date));
        let mut it = t.iter(false, false);
        it.inflow(100.).unwrap();
        let nav = 7459.99;
        assert_eq!(it.asset(), 0.);
        while let Some(_) = it.next_month(Some(28)) {
            it.buy(1, 100., 0.1).unwrap();
        }
        assert_eq!(it.cash(), 0.);
        assert!((it.share(1) - 99.9 / nav).abs() < 1e-6);
        assert!((it.asset() - it.share(1) * 6842.75).abs() < 1e-6);
    }

    /// Test iter `goto`.
    #[test]
    fn test_transaction4() {
        use crate::read_gta;
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
        let end_date = NaiveDate::parse_from_str("2024-01-22", "%Y-%m-%d").unwrap();
        let t = Transaction::new(&[&hs300, &gz2000], Some(start_date), Some(end_date));
        let mut it = t.iter(true, false);
        it.inflow(100.).unwrap();
        assert_eq!(
            it.today(),
            NaiveDate::parse_from_str("2024-01-02", "%Y-%m-%d").unwrap()
        );
        it.buy(1, 100., 1.).unwrap(); // nav = 7562.02
        it.goto(NaiveDate::parse_from_str("2024-01-04", "%Y-%m-%d").unwrap());
        assert_eq!(
            it.today(),
            NaiveDate::parse_from_str("2024-01-04", "%Y-%m-%d").unwrap()
        );
        assert_eq!(it.cash_log().unwrap().len(), 2);
        assert_eq!(it.navs().shape(), [2, 2]);

        it.goto(NaiveDate::parse_from_str("2024-01-13", "%Y-%m-%d").unwrap());
        assert_eq!(
            it.today(),
            NaiveDate::parse_from_str("2024-01-15", "%Y-%m-%d").unwrap()
        );
        it.sell(1, it.share(1), 1.).unwrap(); // nav = 7195.25
        it.next_day();
        assert_eq!(it.cash(), it.asset());
        assert!(f64::abs(99. / 7562.02 * 7195.25 - 1. - it.asset()) < 1e-6);
        assert!(it
            .goto(NaiveDate::parse_from_str("2024-01-20", "%Y-%m-%d").unwrap())
            .is_none());
    }

    /// Test iter `next_month`.
    #[test]
    #[should_panic]
    fn test_trans_after_finish() {
        use crate::read_gta;
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("2023-11-01", "%Y-%m-%d").unwrap();
        let end_date = NaiveDate::parse_from_str("2024-01-22", "%Y-%m-%d").unwrap();
        let t = Transaction::new(&[&hs300, &gz2000], Some(start_date), Some(end_date));
        let mut it = t.iter(false, false);
        it.inflow(100.).unwrap();
        while let Some(_) = it.next_month(Some(28)) {
            it.buy(1, 100., 0.1).unwrap();
        }
        it.sell(1, it.share(1), 0.2).unwrap();
    }
}
