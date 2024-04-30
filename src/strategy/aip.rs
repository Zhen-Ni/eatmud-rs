use crate::TransactionIterator;

#[derive(Debug)]
pub struct AIPError(&'static str);

impl std::fmt::Display for AIPError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "AIPError: {}", self.0)
    }
}

impl std::error::Error for AIPError {}

pub fn aip_monthly(
    it: &mut TransactionIterator,
    day: u32,
    amounts: &[f64],
    fees: &[f64],
) -> Result<(), Box<dyn std::error::Error>> {
    let total_amounts = amounts.iter().sum();
    while it.next_month(Some(day)).is_some() {
        it.inflow(total_amounts)?;
        for j in 0..it.nfunds() {
            let amount = amounts[j];
            it.buy(j, amount, fees[j])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn test_aip_1() {
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("20110101", "%Y%m%d").unwrap();
        let end_date = NaiveDate::parse_from_str("20240101", "%Y%m%d").unwrap();
        let trans = Transaction::new(&[&hs300, &gz2000], Some(start_date), Some(end_date));
        let mut results = Vec::new();
        for save_log in [true, false] {
            for save_record in [true, false] {
                let mut res = Vec::new();
                for day in 1..29 {
                    let mut it = trans.iter(save_log, save_record);
                    aip_monthly(&mut it, day, &[1000.; 2], &[0.; 2]).unwrap();
                    res.push(it.asset());
                }
                results.push(res);
            }
        }
        assert_eq!(results[0], results[1]);
        assert_eq!(results[0], results[2]);
        assert_eq!(results[0], results[3]);
    }

    #[test]
    fn test_aip_2() {
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("20110101", "%Y%m%d").unwrap();
        let end_date = NaiveDate::parse_from_str("20240101", "%Y%m%d").unwrap();
        let trans = Transaction::new(&[&hs300, &gz2000], Some(start_date), Some(end_date));
        let mut results = Vec::new();
        for day in 1..29 {
            let mut it = trans.iter(true, true);
            aip_monthly(&mut it, day, &[1000.; 2], &[2.; 2]).unwrap();
            let rec = it.record().unwrap();
            let irr = rec.irr_naive();
            results.push(irr);
        }
        assert!((results[0] - 0.03206).abs() < 1e-5);
        assert!((results[1] - 0.03140).abs() < 1e-5);
        assert!((results[2] - 0.03128).abs() < 1e-5);
        assert!((results[results.len() - 2] - 0.02339).abs() < 1e-5);
        assert!((results[results.len() - 1] - 0.02659).abs() < 1e-5);
    }
}
