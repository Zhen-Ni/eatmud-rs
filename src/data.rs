use std::{fs, io};
use std::io::BufRead;
use encoding_rs;

use chrono::{NaiveDate};

pub trait Data {
    // Getters and setters.
    fn name(&self) -> &str;
    fn code(&self) -> &str;
    fn date(&self) -> &[NaiveDate];
    fn value(&self) -> &[f64];
    fn date_mut(&mut self) -> &mut [NaiveDate];
    fn value_mut(&mut self) -> &mut [f64];

    /// Get size of the data records.
    fn len(&self) -> usize {
        self.date().len()
    }

    /// Resize data records.
    fn resize(&mut self, size: usize);
}

#[derive(Debug)]
pub struct Fund {
    name: String,
    code: String,
    date: Vec<NaiveDate>,
    value: Vec<f64>,
}

impl Fund {
    /// Constructs an empty Fund object.
    ///
    /// # Examples
    /// ```
    /// use eatmud::{Data, Fund};
    /// let mut stock = Fund::new("hs300", "123456");
    /// assert!(stock.name() == "hs300");
    /// assert!(stock.code() == "123456");
    /// assert!(stock.date().is_empty());
    /// assert!(stock.value().is_empty());
    /// ```
    pub fn new(name: &str, code: &str) -> Fund {
        Fund {
            name: String::from(name),
            code: String::from(code),
            date: Vec::new(),
            value: Vec::new(),
        }
    }

    /// Appends fund records to the end of the record storage.
    ///
    /// # Examples
    /// ```
    /// use chrono::NaiveDate;
    /// use eatmud::{Data, Fund};
    /// let mut fund = Fund::new("hs300", "123456");
    /// let date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d")
    ///     .unwrap();
    /// fund.push(date, 1.0);
    /// assert!(fund.date()[0] == date);
    /// assert!(fund.value()[0] == 1.0);
    /// ```
    pub fn push(&mut self, date: NaiveDate, value: f64) {
        self.date.push(date);
        self.value.push(value);
    }

    /// Indexing data
    /// # Examples
    /// ```
    /// use chrono::NaiveDate;
    /// use eatmud::{Data, Fund};
    /// let mut fund = Fund::new("hs300", "123456");
    /// let date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d")
    ///     .unwrap();
    /// fund.push(date, 1.0);
    /// let slice = fund.index(0);
    /// assert!(*slice.date == date);
    /// assert!(*slice.value == 1.0);
    /// let slice2 = fund.index_mut(0);
    /// *slice2.date = NaiveDate::MAX;
    /// *slice2.value = 2.0;
    /// assert!(fund.date()[0] == NaiveDate::MAX);
    /// assert!(fund.value()[0] == 2.0);
    /// ```    
    pub fn index(&self, idx: usize) -> FundSlice {
        FundSlice {
            date: &self.date[idx],
            value: &self.value[idx],
        }
    }
    pub fn index_mut(&mut self, idx: usize) -> FundSliceMut {
        FundSliceMut {
            date: &mut self.date[idx],
            value: &mut self.value[idx],
        }
    }
}

pub struct FundSlice<'a> {
    pub date: &'a NaiveDate,
    pub value: &'a f64,
}

pub struct FundSliceMut<'a> {
    pub date: &'a mut NaiveDate,
    pub value: &'a mut f64,
}

impl Data for Fund {
    fn name(&self) -> &str {
        &self.name
    }
    fn code(&self) -> &str {
        &self.code
    }

    fn date(&self) -> &[NaiveDate] {
        &self.date
    }

    fn value(&self) -> &[f64] {
        &self.value
    }

    fn date_mut(&mut self) -> &mut [NaiveDate] {
        &mut self.date
    }

    fn value_mut(&mut self) -> &mut [f64] {
        &mut self.value
    }

    fn resize(&mut self, size: usize) {
        self.date.resize(size, NaiveDate::MIN);
        self.value.resize(size, 0.0);
    }
}

impl From<Stock> for Fund {
    /// Converts a Stock object to Fund object.
    fn from(stock: Stock) -> Self {
        Fund { name: stock.name,
               code: stock.code,
               date: stock.date,
               value: stock.close }
    }
}


impl From<&Stock> for Fund {
    /// Converts a Stock object to Fund object.
    fn from(stock: &Stock) -> Self {
        Fund { name: String::from(stock.name()),
               code: String::from(stock.code()),
               date: Vec::from(stock.date()),
               value: Vec::from(stock.close()) }
    }
}


#[derive(Debug)]
pub struct Stock {
    name: String,
    code: String,
    date: Vec<NaiveDate>,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
}

impl Stock {
    /// Constructs an empty Stock object.
    ///
    /// # Examples
    /// ```
    /// use eatmud::{Data, Stock};
    /// let mut stock = Stock::new("hs300", "123456");
    /// assert!(stock.name() == "hs300");
    /// assert!(stock.code() == "123456");
    /// assert!(stock.date().is_empty());
    /// assert!(stock.open().is_empty());
    /// assert!(stock.high().is_empty());
    /// assert!(stock.low().is_empty());
    /// assert!(stock.close().is_empty());
    /// assert!(stock.volume().is_empty());
    /// assert!(stock.value().is_empty());
    /// ```
    pub fn new(name: &str, code: &str) -> Stock {
        Stock {
            name: String::from(name),
            code: String::from(code),
            date: Vec::new(),
            open: Vec::new(),
            high: Vec::new(),
            low: Vec::new(),
            close: Vec::new(),
            volume: Vec::new(),
        }
    }

    pub fn open(&self) -> &[f64] {
        &self.open
    }
    pub fn high(&self) -> &[f64] {
        &self.high
    }
    pub fn low(&self) -> &[f64] {
        &self.low
    }
    pub fn close(&self) -> &[f64] {
        &self.close
    }
    pub fn volume(&self) -> &[f64] {
        &self.volume
    }

    pub fn open_mut(&mut self) -> &mut [f64] {
        &mut self.open
    }
    pub fn high_mut(&mut self) -> &mut [f64] {
        &mut self.high
    }
    pub fn low_mut(&mut self) -> &mut [f64] {
        &mut self.low
    }
    pub fn close_mut(&mut self) -> &mut [f64] {
        &mut self.close
    }
    pub fn volume_mut(&mut self) -> &mut [f64] {
        &mut self.volume
    }

    /// Appends storck record to the end of the record storage.
    ///
    /// # Examples
    /// ```
    /// use chrono::NaiveDate;
    /// use eatmud::{Data, Stock};
    /// let mut stock = Stock::new("ndsd", "SZ300750");
    /// let date = NaiveDate::parse_from_str("2024-01-11", "%Y-%m-%d")
    ///     .unwrap();
    /// stock.push(date, 150.66, 151.37, 148.51, 154.82, 108789736f64);
    /// ```
    pub fn push(&mut self, date: NaiveDate, open: f64, high: f64, low: f64, close: f64, volume: f64) {
        self.date.push(date);
        self.open.push(open);
        self.high.push(high);
        self.low.push(low);
        self.close.push(close);
        self.volume.push(volume);
    }

    /// Indexing data
    /// # Examples
    /// ```
    /// use chrono::NaiveDate;
    /// use eatmud::{Data, Stock};
    /// let mut stock = Stock::new("ndsd", "SZ300750");
    /// let date = NaiveDate::parse_from_str("2024-01-11", "%Y-%m-%d")
    ///     .unwrap();
    /// stock.push(date, 150.66, 151.37, 148.51, 154.82, 108789736f64);
    /// let slice = stock.index(0);
    /// assert!(*slice.date == date);
    /// assert!(*slice.close == 154.82);
    /// let slice2 = stock.index_mut(0);
    /// *slice2.date = NaiveDate::MAX;
    /// *slice2.close = 2.0;
    /// assert!(stock.date()[0] == NaiveDate::MAX);
    /// assert!(stock.value()[0] == 2.0);
    /// ```    
    pub fn index(&self, idx: usize) -> StockSlice {
        StockSlice {
            date: &self.date[idx],
            open: &self.open[idx],
            high: &self.high[idx],
            low: &self.low[idx],
            close: &self.close[idx],
        }
    }
    pub fn index_mut(&mut self, idx: usize) -> StockSliceMut {
        StockSliceMut {
            date: &mut self.date[idx],
            open: &mut self.open[idx],
            high: &mut self.high[idx],
            low: &mut self.low[idx],
            close: &mut self.close[idx],
        }
    }
}

pub struct StockSlice<'a> {
    pub date: &'a NaiveDate,
    pub open: &'a f64,
    pub high: &'a f64,
    pub low: &'a f64,
    pub close: &'a f64,
}

pub struct StockSliceMut<'a> {
    pub date: &'a mut NaiveDate,
    pub open: &'a mut f64,
    pub high: &'a mut f64,
    pub low: &'a mut f64,
    pub close: &'a mut f64,
}

impl Data for Stock {
    fn name(&self) -> &str {
        &self.name
    }
    fn code(&self) -> &str {
        &self.code
    }

    fn date(&self) -> &[NaiveDate] {
        &self.date
    }

    fn value(&self) -> &[f64] {
        &self.close
    }

    fn date_mut(&mut self) -> &mut [NaiveDate] {
        &mut self.date
    }

    fn value_mut(&mut self) -> &mut [f64] {
        &mut self.close
    }

    fn resize(&mut self, size: usize) {
        self.date.resize(size, NaiveDate::MIN);
        self.open.resize(size, 0.0);
        self.high.resize(size, 0.0);
        self.low.resize(size, 0.0);
        self.close.resize(size, 0.0);
    }
}


/// Read stock data from GuoTaiAn's txt output file.
pub fn read_gta(path: &str) -> Option<Stock> {
    let file = fs::File::open(path).expect("file not found");
    let mut reader = io::BufReader::new(file);

    let mut buffer = Vec::<u8>::new();
    reader.read_until(b'\n', &mut buffer).expect("fail to read file");
    let (results, _encoding, _error) = encoding_rs::GBK.decode(&buffer);
    let line = results.to_string();
    buffer.clear();
    let first_line = line.split_whitespace().collect::<Vec<&str>>();
    let [name, code] = first_line.as_slice() else {
            panic!("wrong file format: cannot read header");
        };
    let code = &code.to_string()[1..code.len()-1];
    let mut stock = Stock::new(name, &code);

    while let Ok(size) = reader.read_until(b'\n', &mut buffer) {
        if size == 0 {
            break
        }
        let (results, _encoding, _error) = encoding_rs::GBK.decode(&buffer);
        let line = results.to_string();
        buffer.clear();
        let words = line.split_whitespace().collect::<Vec<&str>>();
        if words.len() < 6 {
            continue
        }
        match words.as_slice()[..6] {
            [date, open, high, low, close, volume] => {
                let Ok(date): Result<NaiveDate, _> = NaiveDate::parse_from_str(&date, "%Y/%m/%d") else {continue;};
                let Ok(open): Result<f64, _> = open.parse() else {continue;};
                let Ok(high): Result<f64, _> = high.parse() else {continue;};
                let Ok(low): Result<f64, _> = low.parse() else {continue;};
                let Ok(close): Result<f64, _> = close.parse() else {continue;};
                let Ok(volume): Result<f64, _> = volume.parse() else {continue;};
                stock.push(date, open, high, low, close, volume);                    }
            _ => {}
        }
    }
    Some(stock)
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fund() {
        let stock = Stock::new("hs300", "123456");
        let mut fund = Fund::from(stock);
        fund.resize(10);
        assert!(fund.len() == 10);
        assert!(fund.date()[9] == NaiveDate::MIN);
        assert!(fund.value()[9] == 0.0);
        let date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
        fund.date_mut()[9] = date;
        fund.value_mut()[9] = 2.0;
        assert!(*fund.index(9).date == date);
        assert!(fund.value()[9] == 2.0);
    }

    #[test]
    fn test_stock() {
        let mut stock = Stock::new("ndsd", "SZ300750");
        let date = NaiveDate::parse_from_str("2024-01-11", "%Y-%m-%d").unwrap();
        stock.push(date, 150.66, 151.37, 148.51, 154.82, 10000.);
        stock.resize(10);
        assert!(stock.len() == 10);
        assert!(stock.date()[0] == date);
        assert!(stock.value()[0] == 154.82);
        assert!(*stock.index(9).date == NaiveDate::MIN);
        assert!(stock.value()[9] == 0.0);
    }

    #[test]
    fn test_read_gta() {
        let filename = "hs300.txt";
        let res = read_gta(filename).expect("failed to read file");
        let n = res.len();
        assert!(res.value()[n-1] == 3347.45);
    }
}
