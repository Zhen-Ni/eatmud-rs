//use eatmud;

use std::time::Instant;

use chrono::NaiveDate;
use eatmud::{read_gta, Fund, Transaction};

fn bench_aip(start_date: &str, end_date: &str) {
    let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
    let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
    let start_date = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").unwrap();
    let end_date = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").unwrap();
    let trans = Transaction::new(&[&hs300, &gz2000], Some(start_date), Some(end_date));

    let now = Instant::now();
    let mut res: Vec<f64> = Vec::new();
    for i in 1..29 {
        let resi = aip(&trans, i);
        res.push(resi);
    }
    let elapsed_time = now.elapsed();
    println!(
        "Running aip took {} micro seconds.",
        elapsed_time.as_micros()
    );
    println!("{:?}", res);

    let now = Instant::now();
    let mut res = Vec::new();
    for i in 1..29 {
        let resi = aip_rec(&trans, i);
        res.push(resi);
    }
    let elapsed_time = now.elapsed();
    println!(
        "Running aip_rec took {} micro seconds.",
        elapsed_time.as_micros()
    );
    println!("{:?}", res);
}

fn aip(trans: &Transaction, day: u32) -> f64 {
    let mut it = trans.iter();
    let mut inflow = 0.;
    while it.next_month(Some(day)).is_some() {
        inflow += 2000.;
        it.inflow(2000.).unwrap();
        it.buy(0, 1000., 2.).unwrap();
        it.buy(1, 1000., 2.).unwrap();
    }
    it.present_asset() / inflow
}

fn aip_rec(trans: &Transaction, day: u32) -> f64 {
    let mut it = trans.iter_rec();
    while it.next_month(Some(day)).is_some() {
        it.inflow(2000.).unwrap();
        it.buy(0, 1000., 2.).unwrap();
        it.buy(1, 1000., 2.).unwrap();
    }
    it.record().unwrap().irr_naive()
}

fn main() {
    bench_aip("2011-01-01", "2024-01-01");
}
