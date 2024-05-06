use encoding_rs;
use std::error::Error;
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

#[derive(Debug)]
pub struct Data<Ds: DataSlice> {
    pub name: String,
    pub code: String,
    data: Vec<Ds>,
}

pub type Fund = Data<FundSlice>;
pub type Stock = Data<StockSlice>;

impl<Ds: DataSlice> Data<Ds> {
    /// Constructs an empty Fund object.
    ///
    /// # Examples
    /// ```
    /// # use eatmud::Fund;
    /// let mut fund = Fund::new("hs300", "123456");
    /// assert!(fund.name() == "hs300");
    /// assert!(fund.code() == "123456");
    /// assert!(fund.data().is_empty());
    /// ```
    pub fn new(name: &str, code: &str) -> Self {
        Data {
            name: String::from(name),
            code: String::from(code),
            data: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn data(&self) -> &[Ds] {
        &self.data
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Data<FundSlice> {
    /// Appends fund records to the end of data storage.
    ///
    /// # Examples
    /// ```
    /// # use chrono::NaiveDate;
    /// # use eatmud::Fund;
    /// let mut fund = Fund::new("hs300", "123456");
    /// let date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d")
    ///     .unwrap();
    /// fund.append(date, 1.0);
    /// assert!(fund[0].date == date);
    /// assert!(fund[0].value == 1.0);
    /// ```
    pub fn append(&mut self, date: NaiveDate, value: f64) {
        self.data.push(FundSlice { date, value });
    }
}

impl Data<StockSlice> {
    /// Appends stock records to the end of data storage.
    ///
    /// # Examples
    /// ```
    /// # use chrono::NaiveDate;
    /// # use eatmud::prelude::*;
    /// # use eatmud::Stock;
    /// let mut stock = Stock::new("hs300", "123456");
    /// let date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d")
    ///     .unwrap();
    /// stock.append(date, 1.0, 2.0, 0.5, 0.8, 100f64);
    /// assert!(stock[0].date() == date);
    /// assert!(stock[0].value() == 0.8);
    /// ```
    pub fn append(
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

impl<Ds: DataSlice> Index<usize> for Data<Ds> {
    type Output = Ds;
    fn index(&self, index: usize) -> &Ds {
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
pub struct ReadDataError(&'static str);

impl std::fmt::Display for ReadDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Read Data Error: {}", self.0)
    }
}

impl std::error::Error for ReadDataError {}

/// Read stock data from GuoTaiAn's txt output file.
pub fn read_gta(path: &str) -> Result<Stock, Box<dyn Error>> {
    let file = fs::File::open(path)?;
    let mut reader = io::BufReader::new(file);

    let mut buffer = Vec::<u8>::new();
    reader.read_until(b'\n', &mut buffer)?;
    let (results, _encoding, _error) = encoding_rs::GBK.decode(&buffer);
    let line = results.to_string();
    buffer.clear();
    let first_line = line.split_whitespace().collect::<Vec<&str>>();
    let [name, code] = first_line.as_slice() else {
        return Err(Box::new(ReadDataError(
            "Wrong file format: cannot parse header",
        )));
    };
    let code = &code.to_string()[1..code.len() - 1];
    let mut stock = Data::<StockSlice>::new(name, code);

    while let Ok(size) = reader.read_until(b'\n', &mut buffer) {
        if size == 0 {
            break;
        }
        let (results, _encoding, _error) = encoding_rs::GBK.decode(&buffer);
        let line = results.to_string();
        buffer.clear();

        let mut iter_words = line.split_whitespace();
        let Some(date) = iter_words.next() else {
            continue;
        };
        let Some(open) = iter_words.next() else {
            continue;
        };
        let Some(high) = iter_words.next() else {
            continue;
        };
        let Some(low) = iter_words.next() else {
            continue;
        };
        let Some(close) = iter_words.next() else {
            continue;
        };
        let Some(volume) = iter_words.next() else {
            continue;
        };

        let Ok(date): Result<NaiveDate, _> = NaiveDate::parse_from_str(date, "%Y/%m/%d") else {
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
        stock.append(date, open, high, low, close, volume);
    }
    Ok(stock)
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
        assert_eq!(fund[0].value(), 982.79);
        assert_eq!(fund.code(), "000300");
        let date = NaiveDate::parse_from_str("2024-01-01", "%Y-%m-%d").unwrap();
        fund.append(date, 2.0);
        assert_eq!(fund[n].date, date);
        assert_eq!(fund[n].value, 2.0);
    }

    #[test]
    fn test_stock() {
        let mut stock = Stock::new("ndsd", "SZ300750");
        let date = NaiveDate::parse_from_str("2024-01-11", "%Y-%m-%d").unwrap();
        stock.append(date, 150.66, 151.37, 148.51, 154.82, 10000.);
        stock.append(date + Days::new(1), 157.34, 159.87, 148.51, 153.45, 9754.);
        assert_eq!(stock.len(), 2);
        assert_eq!(stock[0].date(), date);
        assert_eq!(stock[0].value(), 154.82);
    }
}
