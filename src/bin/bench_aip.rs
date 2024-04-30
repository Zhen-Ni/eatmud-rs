//use eatmud;

use std::time::Instant;

use chrono::NaiveDate;
use eatmud::{read_gta, Fund, Transaction};

fn bench_aip(trans: &Transaction) -> Vec<Vec<f64>> {
    let mut results = Vec::new();
    for save_log in [true, false] {
        for save_record in [true, false] {
            let name = format!("save_log={}, save_record={}", save_log, save_record);
            let now = Instant::now();
            let mut res = Vec::new();
            for day in 1..29 {
                let mut it = trans.iter(save_log, save_record);
                eatmud::strategy::aip_monthly(&mut it, day, &[1000., 1000.], &[0., 0.]).unwrap();
                res.push(it.asset());
            }
            let total_time = Instant::now() - now;
            println!(
                "running aip({}) took {} milli seconds",
                name,
                total_time.as_micros() as f64 / 1000.
            );
            results.push(res);
        }
    }
    results
}

fn main() {
    let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
    let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
    let start_date = NaiveDate::parse_from_str("20110101", "%Y%m%d").unwrap();
    let end_date = NaiveDate::parse_from_str("20240101", "%Y%m%d").unwrap();
    let trans = Transaction::new(&[&hs300, &gz2000], Some(start_date), Some(end_date));
    let results = bench_aip(&trans);
    assert_eq!(results[0], results[1]);
    assert_eq!(results[0], results[2]);
    assert_eq!(results[0], results[3]);
}
