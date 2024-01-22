use encoding_rs;
use std::io::BufRead;
use std::ops::Index;
use std::{fs, io};

use chrono::NaiveDate;

pub trait DataSlice {
    fn date(&self) -> NaiveDate;
    fn value(&self) -> f64;
}

#[derive(Debug, Clone)]
pub struct FundSlice {
    pub date: NaiveDate,
    pub value: f64,
}

impl DataSlice for FundSlice {
    fn date(&self) -> NaiveDate {
        self.date
    }
    fn value(&self) -> f64 {
        self.value
    }
}

#[derive(Debug, Clone)]
pub struct StockSlice {
    pub date: NaiveDate,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl DataSlice for StockSlice {
    fn date(&self) -> NaiveDate {
        self.date
    }

    fn value(&self) -> f64 {
        self.close
    }
}

pub trait Data {
    // Getters and setters.
    fn name(&self) -> &str;
    fn code(&self) -> &str;
    fn data(&self) -> &[impl DataSlice];

    /// Get size of the data records.
    fn len(&self) -> usize {
        self.data().len()
    }
}

#[derive(Debug)]
pub struct Fund {
    name: String,
    code: String,
    data: Vec<FundSlice>,
}

impl Fund {
    /// Constructs an empty Fund object.
    ///
    /// # Examples
    /// ```
    /// use eatmud::{Data, Fund};
    /// let mut fund = Fund::new("hs300", "123456");
    /// assert!(fund.name() == "hs300");
    /// assert!(fund.code() == "123456");
    /// assert!(fund.data().is_empty());
    /// ```
    pub fn new(name: &str, code: &str) -> Fund {
        Fund {
            name: String::from(name),
            code: String::from(code),
            data: Vec::new(),
        }
    }

    /// Appends fund records to the end of data storage.
    ///
    /// # Examples
    /// ```
    /// use chrono::NaiveDate;
    /// use eatmud::{Data, Fund};
    /// let mut fund = Fund::new("hs300", "123456");
    /// let date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d")
    ///     .unwrap();
    /// fund.push(date, 1.0);
    /// assert!(fund[0].date == date);
    /// assert!(fund[0].value == 1.0);
    /// ```
    pub fn push(&mut self, date: NaiveDate, value: f64) {
        self.data.push(FundSlice { date, value });
    }
}

impl Data for Fund {
    fn name(&self) -> &str {
        &self.name
    }
    fn code(&self) -> &str {
        &self.code
    }

    fn data(&self) -> &[impl DataSlice] {
        &self.data
    }
}

impl Index<usize> for Fund {
    type Output = FundSlice;
    fn index(&self, index: usize) -> &FundSlice {
        &self.data[index]
    }
}

impl From<&Stock> for Fund {
    fn from(stock: &Stock) -> Fund {
        let mut fund = Fund::new(stock.name(), stock.code());
        fund.data = stock
            .data()
            .iter()
            .map(|ss| FundSlice {
                date: ss.date(),
                value: ss.value(),
            })
            .collect();
        fund
    }
}

#[derive(Debug)]
pub struct Stock {
    name: String,
    code: String,
    data: Vec<StockSlice>,
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
    /// assert!(stock.data().is_empty());
    /// ```
    pub fn new(name: &str, code: &str) -> Stock {
        Stock {
            name: String::from(name),
            code: String::from(code),
            data: Vec::new(),
        }
    }

    /// Appends stock records to the end of data storage.
    ///
    /// # Examples
    /// ```
    /// use chrono::NaiveDate;
    /// use eatmud::{Data, DataSlice, Stock};
    /// let mut stock = Stock::new("hs300", "123456");
    /// let date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d")
    ///     .unwrap();
    /// stock.push(date, 1.0, 2.0, 0.5, 0.8, 100f64);
    /// assert!(stock[0].date() == date);
    /// assert!(stock[0].value() == 0.8);
    /// ```
    pub fn push(
        &mut self,
        date: NaiveDate,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) {
        self.data.push(StockSlice {
            date,
            open,
            high,
            low,
            close,
            volume,
        });
    }
}

impl Data for Stock {
    fn name(&self) -> &str {
        &self.name
    }
    fn code(&self) -> &str {
        &self.code
    }

    fn data(&self) -> &[impl DataSlice] {
        &self.data
    }
}

impl Index<usize> for Stock {
    type Output = StockSlice;
    fn index(&self, index: usize) -> &StockSlice {
        &self.data[index]
    }
}

/// Read stock data from GuoTaiAn's txt output file.
pub fn read_gta(path: &str) -> Option<Stock> {
    let file = fs::File::open(path).ok()?;
    let mut reader = io::BufReader::new(file);

    let mut buffer = Vec::<u8>::new();
    reader
        .read_until(b'\n', &mut buffer)
        .ok()?;
    let (results, _encoding, _error) = encoding_rs::GBK.decode(&buffer);
    let line = results.to_string();
    buffer.clear();
    let first_line = line.split_whitespace().collect::<Vec<&str>>();
    let [name, code] = first_line.as_slice() else {
        // Wrong file format: cannot read header
        return None;
    };
    let code = &code.to_string()[1..code.len() - 1];
    let mut stock = Stock::new(name, &code);

    while let Ok(size) = reader.read_until(b'\n', &mut buffer) {
        if size == 0 {
            break;
        }
        let (results, _encoding, _error) = encoding_rs::GBK.decode(&buffer);
        let line = results.to_string();
        buffer.clear();
        let words = line.split_whitespace().collect::<Vec<&str>>();
        if words.len() < 6 {
            continue;
        }
        match words.as_slice()[..6] {
            [date, open, high, low, close, volume] => {
                let Ok(date): Result<NaiveDate, _> = NaiveDate::parse_from_str(&date, "%Y/%m/%d")
                else {
                    continue;
                };
                let Ok(open): Result<f64, _> = open.parse() else {
                    continue;
                };
                let Ok(high): Result<f64, _> = high.parse() else {
                    continue;
                };
                let Ok(low): Result<f64, _> = low.parse() else {
                    continue;
                };
                let Ok(close): Result<f64, _> = close.parse() else {
                    continue;
                };
                let Ok(volume): Result<f64, _> = volume.parse() else {
                    continue;
                };
                stock.push(date, open, high, low, close, volume);
            }
            _ => {}
        }
    }
    Some(stock)
}

#[cfg(test)]
mod test {
    use chrono::Days;

    use super::*;

    #[test]
    fn test_fund() {
        let filename = "hs300.txt";
        let stock = read_gta(filename).expect("failed to read file");
        let mut fund = Fund::from(&stock);
        let n = fund.len();
        assert!(fund[n - 1].value() == 3347.45);
        let date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
        fund.push(date, 2.0);
        assert!(fund[n].date == date);
        assert!(fund[n].value == 2.0);
    }

    #[test]
    fn test_stock() {
        let mut stock = Stock::new("ndsd", "SZ300750");
        let date = NaiveDate::parse_from_str("2024-01-11", "%Y-%m-%d").unwrap();

        stock.push(date, 150.66, 151.37, 148.51, 154.82, 10000.);
        stock.push(date + Days::new(1), 157.34, 159.87, 148.51, 153.45, 9754.);
        assert!(stock.len() == 2);
        assert!(stock[0].date() == date);
        assert!(stock[0].value() == 154.82);
    }
}
