use std::time::Instant;

use eatmud::{read_gta, Fund, NaiveDate, Transaction, Weekday};

fn bench_kelly(trans: &Transaction, start_date: NaiveDate) -> Vec<Vec<f64>> {
    let ns = [1300, 1600];
    let inflations = [0.015, 0.015];
    let risk_bounds = [0.01, 0.01];
    let mut results = Vec::new();
    for save_log in [true, false] {
        for save_record in [true, false] {
            let name = format!("save_log={}, save_record={}", save_log, save_record);
            let now = Instant::now();
            let mut res = Vec::new();
            for weekday in 0..5 {
                let mut it = trans.iter(save_log, save_record);
                it.goto(start_date);
                it.inflow(1.).unwrap();
                eatmud::strategy::kelly_weekly(
                    &mut it,
                    Weekday::try_from(weekday).unwrap(),
                    &ns,
                    &inflations,
                    &risk_bounds,
                )
                .unwrap();
                res.push(it.asset());
            }
            let total_time = Instant::now() - now;
            println!(
                "running kelly({}) took {} milli seconds",
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
    let start_date = NaiveDate::parse_from_str("20170101", "%Y%m%d").unwrap();
    let end_date = NaiveDate::parse_from_str("20240101", "%Y%m%d").unwrap();
    let trans = Transaction::new(&[&hs300, &gz2000], None, Some(end_date));
    let results = bench_kelly(&trans, start_date);
    println!("{:?}", results);
    assert_eq!(results[0], results[1]);
    assert_eq!(results[0], results[2]);
    assert_eq!(results[0], results[3]);
}
