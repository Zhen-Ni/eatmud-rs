use std::time::Instant;

use eatmud::{read_gta, strategy, Fund, NaiveDate, Transaction, Weekday};

fn bench_kelly(start_date: &str, end_date: &str) {
    let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
    let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
    let start_date = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").unwrap();
    let end_date = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").unwrap();
    let trans = Transaction::new(&[&hs300, &gz2000], None, Some(end_date));
    let ns = [1300, 1600];
    let inflations = [0.015, 0.015];
    let risk_bounds = [0.01, 0.01];

    let now = Instant::now();
    let mut res: Vec<f64> = Vec::new();
    for i in 1..=5 {
        let resi = kelly(
            &trans,
            start_date,
            Weekday::try_from(i as u8 - 1).unwrap(),
            &ns,
            &inflations,
            &risk_bounds,
        );
        res.push(resi);
    }
    let elapsed_time = now.elapsed();
    println!(
        "Running kelly took {} seconds.",
        elapsed_time.as_secs_f32()
    );
    println!("{:?}", res);
}

fn kelly(
    trans: &Transaction,
    start_date: NaiveDate,
    weekday: Weekday,
    ns: &[usize],
    inflations: &[f64],
    risk_bounds: &[f64],
) -> f64 {
    let mut it = trans.iter_rec();
    it.goto(start_date);
    it.inflow(1.0).unwrap();
    let it = strategy::kelly_weekly(it, weekday, ns, inflations, risk_bounds).unwrap();
    it.present_asset()
}


fn main() {
    bench_kelly("2017-01-01", "2024-01-01");
}
