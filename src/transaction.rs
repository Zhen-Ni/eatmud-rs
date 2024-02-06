use crate::{
    merge_records, utility::search_sorted, ConciseRecord, DataSlice, DetailedRecord, Fund,
};
use chrono::{Datelike, Duration, NaiveDate};
use ndarray::{s, Array1, Array2, ArrayView1, ArrayView2, AssignElem, Axis, ShapeBuilder};

pub struct Transaction {
    names: Vec<String>,
    codes: Vec<String>,
    date: Vec<NaiveDate>,
    navs: Array2<f64>, // net assert value
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
    /// let t = Transaction::new(&[hs300, gz2000], Some(start_date), None);
    /// ```
    pub fn new(
        funds: &[Fund],
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Transaction {
        let names: Vec<_> = funds.iter().map(|d| d.name().to_string()).collect();
        let codes: Vec<_> = funds.iter().map(|d| d.code().to_string()).collect();

        let start_date = start_date.unwrap_or(funds.iter().map(|d| d[0].date()).max().unwrap());
        let end_date =
            end_date.unwrap_or(funds.iter().map(|d| d[d.len() - 1].date()).min().unwrap());
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

    pub fn from_funds(funds: &[Fund]) -> Self {
        Self::new(funds, None, None)
    }

    pub fn start_date(&self) -> NaiveDate {
        self.start_date
    }

    pub fn end_date(&self) -> NaiveDate {
        self.end_date
    }

    pub fn navs(&self) -> &Array2<f64> {
        &self.navs
    }

    pub fn iter(&self) -> TransactionIterator {
        TransactionIterator::new(self, false)
    }

    pub fn iter_rec(&self) -> TransactionIterator {
        TransactionIterator::new(self, true)
    }
}

pub struct TransactionLog {
    index: usize,
    pub cash: Array1<f64>,
    pub share: Array2<f64>,
}

impl TransactionLog {
    fn reset(&mut self) {
        self.index = 0;
        self.cash.fill(0.);
        self.share.fill(0.);
    }
}

struct IterBuffer {
    cash: f64,
    investment: Vec<f64>,
    share: Vec<f64>,
}

impl IterBuffer {
    fn reset(&mut self) {
        self.cash = 0.;
        self.investment.fill(0.0);
        self.share.fill(0.0);
    }
}

struct IterRecord {
    cash_comment_buffer: String,
    fund_comment_buffer: Vec<String>,
    cash_record: ConciseRecord,
    fund_record: Vec<DetailedRecord>,
}

impl IterRecord {
    fn reset_buffer(&mut self) {
        self.cash_comment_buffer.clear();
        for item in &mut self.fund_comment_buffer {
            item.clear();
        }
    }
    fn reset(&mut self) {
        self.reset_buffer();
        self.cash_record.clear();
        for item in &mut self.fund_record {
            item.clear();
        }
    }
}

pub use chrono::Weekday;

pub struct TransactionIterator<'a> {
    transaction: &'a Transaction,
    iter_buffer: IterBuffer,
    iter_log: TransactionLog,
    record: Option<IterRecord>,
}

impl<'a> TransactionIterator<'a> {
    pub(crate) fn new(trans: &'a Transaction, save_record: bool) -> Self {
        let ndays = trans.date.len();
        let nfunds = trans.names.len();
        TransactionIterator {
            transaction: trans,
            iter_buffer: IterBuffer {
                cash: 0.,
                investment: vec![0.0; nfunds],
                share: vec![0.0; nfunds],
            },
            iter_log: TransactionLog {
                index: 0,
                cash: Array1::zeros(ndays),
                share: Array2::zeros((ndays, nfunds)),
            },
            record: if save_record {
                Some(IterRecord {
                    cash_comment_buffer: String::new(),
                    fund_comment_buffer: vec!["".to_string(); nfunds],
                    cash_record: ConciseRecord::new("cash", ""),
                    fund_record: Vec::from_iter(
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

    pub fn reset(&mut self) -> &mut Self {
        self.iter_log.reset();
        self.iter_buffer.reset();
        if let Some(ref mut r) = self.record {
            r.reset()
        }
        self
    }

    #[inline]
    fn index(&self) -> usize {
        self.iter_log.index
    }

    #[inline]
    fn is_finished(&self) -> bool {
        self.index() == self.transaction.date.len()
    }

    #[inline]
    fn assert_not_finished(&self) {
        if self.is_finished() {
            panic!("transaction iteration reaches the end")
        }
    }

    pub fn today(&self) -> NaiveDate {
        if self.is_finished() {
            self.transaction.date[self.index() - 1]
        } else {
            self.transaction.date[self.index()]
        }
    }

    /// Cash at the *beginning* of the day.
    pub fn present_cash(&self) -> f64 {
        if self.is_finished() {
            self.iter_log.cash[self.index() - 1]
        } else {
            self.iter_log.cash[self.index()]
        }
    }

    /// Share at the *beginning* of the day.
    pub fn present_share(&self, idx: usize) -> f64 {
        if self.is_finished() {
            self.iter_log.share[[self.index() - 1, idx]]
        } else {
            self.iter_log.share[[self.index(), idx]]
        }
    }

    /// Total assert at the *beginning* of the day.
    pub fn present_assert(&self) -> f64 {
        if self.index() == 0 {
            0.
        } else if self.is_finished() {
            (self.iter_log.share.row(self.index() - 1))
                .dot(&self.transaction.navs().row(self.index() - 1))
                + self.present_cash()
        } else {
            (self.iter_log.share.row(self.index()))
                .dot(&self.transaction.navs().row(self.index() - 1))
                + self.present_cash()
        }
    }

    /// Dates
    pub fn date(&self) -> &[NaiveDate] {
        &self.transaction.date[..self.index()]
    }

    /// Net assert values.
    pub fn nav(&self) -> ArrayView2<f64> {
        self.transaction.navs.slice(s![..self.index(), ..])
    }

    /// Log of cash.
    pub fn cash(&self) -> ArrayView1<f64> {
        self.iter_log.cash.slice(s![..self.index()])
    }

    /// Log of share
    pub fn share(&self) -> ArrayView2<f64> {
        self.iter_log.share.slice(s![..self.index(), ..])
    }

    pub fn assert(&self) -> Array1<f64> {
        let assert = &self.share() * &self.nav();
        let assert = assert.sum_axis(Axis(1));
        assert + self.cash()
    }

    pub fn inflow(&mut self, amount: f64) -> &mut Self {
        self.assert_not_finished();
        self.iter_buffer.cash += amount;
        self
    }

    pub fn inflow_comment(&mut self, amount: f64, comment: &str) -> &mut Self {
        self.inflow(amount);
        if let Some(ref mut record) = self.record {
            if !record.cash_comment_buffer.is_empty() {
                record.cash_comment_buffer.push_str("; ");
                record.cash_comment_buffer.push_str(comment);
            }
        }
        self
    }

    pub fn buy(&mut self, fundid: usize, investment: f64, fee: f64) -> &mut Self {
        self.assert_not_finished();
        self.iter_buffer.share[fundid] +=
            (investment - fee) / self.transaction.navs[[self.index(), fundid]];
        self.iter_buffer.cash -= investment;
        self.iter_buffer.investment[fundid] += investment;
        self
    }

    pub fn buy_comment(
        &mut self,
        fundid: usize,
        investment: f64,
        fee: f64,
        comment: &str,
    ) -> &mut Self {
        self.buy(fundid, investment, fee);
        if let Some(ref mut record) = self.record {
            if !record.fund_comment_buffer[fundid].is_empty() {
                record.fund_comment_buffer[fundid].push_str("; ");
                record.fund_comment_buffer[fundid].push_str(comment);
            }
        }
        self
    }

    pub fn sell(&mut self, fundid: usize, share: f64, fee: f64) -> &mut Self {
        self.assert_not_finished();
        let income = share * self.transaction.navs[[self.index(), fundid]] - fee;
        self.iter_buffer.share[fundid] -= share;
        self.iter_buffer.cash += income;
        self.iter_buffer.investment[fundid] += income;
        self
    }

    pub fn sell_comment(
        &mut self,
        fundid: usize,
        share: f64,
        fee: f64,
        comment: &str,
    ) -> &mut Self {
        self.sell(fundid, share, fee);
        if let Some(ref mut record) = self.record {
            if !record.fund_comment_buffer[fundid].is_empty() {
                record.fund_comment_buffer[fundid].push_str("; ");
                record.fund_comment_buffer[fundid].push_str(comment);
            }
        }
        self
    }

    // This method is only called by `flush_and_step`.
    fn flush(&mut self) {
        let index = self.index();
        self.iter_log.cash[index] += self.iter_buffer.cash;
        for i in 0..self.transaction.names.len() {
            self.iter_log.share[[index, i]] += self.iter_buffer.share[i];
        }
        // Record.
        if let Some(ref mut record) = self.record {
            record.cash_record.append(
                self.transaction.date[index],
                self.iter_buffer.cash,
                self.iter_log.cash[index],
                &record.cash_comment_buffer,
            );
            for (i, r) in record.fund_record.iter_mut().enumerate() {
                r.append(
                    self.transaction.date[index],
                    self.iter_buffer.investment[i],
                    self.transaction.navs()[[index, i]],
                    self.iter_buffer.share[i],
                    &record.fund_comment_buffer[i],
                )
            }
            record.reset_buffer();
        }
        self.iter_buffer.reset();
    }

    // This method is only called by `flush_and_step`.
    fn step(&mut self, n: usize) {
        let mut index = self.index();
        let (upper, mut lower) = self.iter_log.share.view_mut().split_at(Axis(0), index + 1);
        let share = upper.row(index);
        for i in 0..n {
            index += 1;
            // Check if iteration reachs the end.
            if index == self.transaction.date.len() {
                break;
            } else {
                // self.iter_log.cash[index] = self.iter_log.cash[index - 1];
                unsafe {
                    let v = *self.iter_log.cash.uget(index - 1);
                    self.iter_log.cash.uget_mut(index).assign_elem(v);
                }
                // lower.row_mut(i).assign(&share);
                for j in 0..self.transaction.names.len() {
                    unsafe {
                        lower.uget_mut((i, j)).assign_elem(*share.uget(j));
                    }
                }
            }
        }
        self.iter_log.index = index;
    }

    /// Return whether the iteration reaches the end.
    fn flush_and_step(&mut self, n: usize) -> bool {
        if n == 0 {
            eprintln!(
                "Step to the same date is would refresh iter_buffer, which affects self.present_*."
            )
        }
        self.flush();
        self.step(n);
        self.is_finished()
    }

    pub fn next_day(&mut self) -> Option<&mut Self> {
        if self.flush_and_step(1) {
            None
        } else {
            Some(self)
        }
    }

    pub fn next_weekday(&mut self, weekday: Option<Weekday>) -> Option<&mut Self> {
        let weekday = weekday.unwrap_or(self.today().weekday());
        let mut n = self.transaction.date.len() - self.index();
        for (i, day) in self.transaction.date[self.index()..].iter().enumerate() {
            if i == 0 {
                continue;
            }
            if weekday == day.weekday() {
                n = i;
                break;
            }
        }
        if self.flush_and_step(n) {
            None
        } else {
            Some(self)
        }
    }

    pub fn goto(&mut self, date: NaiveDate) -> Option<&mut Self> {
        let n = search_sorted(
            &self.transaction.date[usize::min(self.index(), self.transaction.date.len())..],
            &date,
            |d| *d,
            None,
        );
        if self.flush_and_step(n) {
            None
        } else {
            Some(self)
        }
    }

    pub fn next_month(&mut self, day: Option<u32>) -> Option<&mut Self> {
        let day = day.unwrap_or(self.today().day());
        let mut year = self.today().year();
        let mut month = self.today().month() + 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
        let date =
            NaiveDate::from_ymd_opt(year, month, 1).unwrap() + Duration::days(day as i64 - 1);
        let n = search_sorted(
            &self.transaction.date[usize::min(self.index() + 1, self.transaction.date.len())
                ..usize::min(self.index() + 61, self.transaction.date.len())],
            &date,
            |d| *d,
            None,
        ) + 1;
        if self.flush_and_step(n) {
            None
        } else {
            Some(self)
        }
    }

    pub fn cash_record(&self) -> Option<&ConciseRecord> {
        if let Some(ref record) = self.record {
            Some(&record.cash_record)
        } else {
            None
        }
    }

    pub fn fund_record(&self) -> Option<&[DetailedRecord]> {
        if let Some(ref record) = self.record {
            Some(&record.fund_record)
        } else {
            None
        }
    }

    pub fn record(&self) -> Option<ConciseRecord> {
        if let Some(ref record) = self.record {
            let res = ConciseRecord::new("Combined Record", "");
            let mut res = merge_records!(&res, &record.cash_record);
            for rec in record.fund_record.iter() {
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
        let t = Transaction::new(&[hs300, gz2000], Some(start_date), None);
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
        let t = Transaction::new(&[hs300, gz2000], Some(start_date), None);
        let mut it = t.iter();
        let mut idx = 0;
        while let Some(_) = it.next_day() {
            it.inflow(1.0);
            assert!(it.present_cash() == idx as f64);
            assert!(it.present_assert() == idx as f64);
            idx += 1;
        }
        assert!(it.present_cash() > 980.)
    }

    /// Test iter `next_weekday`.
    #[test]
    fn test_transaction2() {
        use crate::read_gta;
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
        let end_date = NaiveDate::parse_from_str("2024-01-20", "%Y-%m-%d").unwrap();
        let t = Transaction::new(&[hs300, gz2000], Some(start_date), Some(end_date));
        let mut it = t.iter();
        it.inflow(100.);
        assert_eq!(it.present_assert(), 0.);
        while let Some(_) = it.next_weekday(Some(Weekday::Wed)) {
            assert!(it.present_assert() > 90.);
            assert!(it.present_assert() < 110.);
            it.buy(0, 10., 0.);
            it.buy(1, 10., 0.);
        }
        assert!(it.present_cash() == 40.);
        assert!((it.present_share(0) - 0.009108).abs() < 1e-6);
    }

    /// Test iter `next_month`.
    #[test]
    fn test_transaction3() {
        use crate::read_gta;
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("2023-11-01", "%Y-%m-%d").unwrap();
        let end_date = NaiveDate::parse_from_str("2024-01-22", "%Y-%m-%d").unwrap();
        let t = Transaction::new(&[hs300, gz2000], Some(start_date), Some(end_date));
        let mut it = t.iter();
        it.inflow(100.);
        let nav = 7459.99;
        assert_eq!(it.present_assert(), 0.);
        while let Some(_) = it.next_month(Some(28)) {
            it.buy(1, 100., 0.1);
        }
        assert_eq!(it.present_cash(), 0.);
        assert!((it.present_share(1) - 99.9 / nav).abs() < 1e-6);
        assert!((it.present_assert() - it.present_share(1) * 6842.75).abs() < 1e-6);
    }

    /// Test iter `goto`.
    #[test]
    fn test_transaction4() {
        use crate::read_gta;
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
        let end_date = NaiveDate::parse_from_str("2024-01-22", "%Y-%m-%d").unwrap();
        let t = Transaction::new(&[hs300, gz2000], Some(start_date), Some(end_date));
        let mut it = t.iter();
        it.inflow(100.);
        assert_eq!(
            it.today(),
            NaiveDate::parse_from_str("2024-01-02", "%Y-%m-%d").unwrap()
        );
        it.buy(1, 100., 1.); // nav = 7562.02
        it.goto(NaiveDate::parse_from_str("2024-01-04", "%Y-%m-%d").unwrap());
        assert_eq!(
            it.today(),
            NaiveDate::parse_from_str("2024-01-04", "%Y-%m-%d").unwrap()
        );
        assert_eq!(it.cash().len(), 2);
        assert_eq!(it.nav().shape(), [2, 2]);

        it.goto(NaiveDate::parse_from_str("2024-01-13", "%Y-%m-%d").unwrap());
        assert_eq!(
            it.today(),
            NaiveDate::parse_from_str("2024-01-15", "%Y-%m-%d").unwrap()
        );
        it.sell(1, it.present_share(1), 1.); // nav = 7195.25
        it.next_day();
        assert_eq!(it.present_cash(), it.present_assert());
        assert!(f64::abs(99. / 7562.02 * 7195.25 - 1. - it.present_assert()) < 1e-6);
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
        let t = Transaction::new(&[hs300, gz2000], Some(start_date), Some(end_date));
        let mut it = t.iter();
        it.inflow(100.);
        while let Some(_) = it.next_month(Some(28)) {
            it.buy(1, 100., 0.1);
        }
        it.sell(1, it.present_share(1), 0.2);
    }
}
