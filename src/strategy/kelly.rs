use crate::DAYS_PER_YEAR;
use crate::{TransactionIterator, Weekday};
use chrono::Datelike;
use ndarray::Array;
use ndarray::{s, Array1};

#[derive(Debug)]
pub struct KellyError(&'static str);

impl std::fmt::Display for KellyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "KellyError: {}", self.0)
    }
}

impl std::error::Error for KellyError {}

pub fn kelly_weekly<'a>(
    mut it: TransactionIterator<'a>,
    weekday: Weekday,
    ns: &[usize],
    inflations: &[f64],
    risk_bounds: &[f64],
) -> Result<TransactionIterator<'a>, Box<dyn std::error::Error>> {
    if it.navs().shape()[0] < *ns.iter().max().unwrap() {
        return Err(Box::new(KellyError(
            "ns too large for transaction simulation",
        )));
    }
    let mut inflation_arrays = Vec::new();
    for i in 0..it.nfunds() {
        let arr = Array::linspace((ns[i] - 1) as f64, 0., ns[i]);
        let arr = arr.mapv(|x| (1. + inflations[i]).powf(x / DAYS_PER_YEAR));
        inflation_arrays.push(arr);
    }
    while it.next_weekday(Some(weekday)).is_some() {
        for j in 0..it.nfunds() {
            let n = ns[j];
            let navs = it.navs();
            // Net assert value of the last n days.
            let y0 = navs.slice(s![-(n as isize).., j as isize]);
            // Net assert value considering inflation.
            // y = y0 * (1 + inflation) ** number_of_years_to_today
            let y = &y0 * &inflation_arrays[j];
            // Get winning rate.
            let y_weekly = y
                .iter()
                .zip(it.date())
                .filter(|(_yi, &di)| di.weekday() == weekday)
                .map(|(&yi, _di)| yi)
                .collect::<Array1<_>>();
            let dy = &y_weekly.slice(s![1..]) - &y_weekly.slice(s![..-1]);
            let p = dy.iter().filter(|&&x| x > 0.).count() as f64 / dy.len() as f64;
            // Kelly.
            let y_max = *y
                .iter()
                .max_by(|&x, &y| f64::total_cmp(x, y))
                .expect("fail to find maximal equivalent NAV");
            let y_min = *y
                .iter()
                .min_by(|&x, &y| f64::total_cmp(x, y))
                .expect("fail to find minimal equivalent NAV");
            let b = y_max / y[y.len() - 1] - 1.;
            let c = 1. - y_min / y[y.len() - 1];
            let f = kelly_equation(p, b, c);

            // Risk control.
            let y0_max = *y0
                .iter()
                .max_by(|&x, &y| f64::total_cmp(x, y))
                .expect("fail to find maximal NAV");
            let y0_min = *y0
                .iter()
                .min_by(|&x, &y| f64::total_cmp(x, y))
                .expect("fail to find minimal NAV");
            let bound_width = (y0_max - y0_min) * risk_bounds[j];
            let f = if y0[y0.len() - 1] >= y0_max - bound_width {
                1.
            } else if y0[y0.len() - 1] <= y0_min + bound_width {
                0.
            } else {
                f
            };

            // Adjust position
            let total = it.present_asset() / it.nfunds() as f64;
            let total = total * f;
            let current = it.present_fund_asset(j);
            let amount = total - current;
            it.buy_comment(j, amount, 0.0, &format!("position = {:.2}%", 100. * f))?;
        }
    }
    Ok(it)
}

fn kelly_equation(p: f64, b: f64, c: f64) -> f64 {
    let q = 1. - p;
    let f = if b == 0. {
        f64::NEG_INFINITY
    } else if c == 0. {
        f64::INFINITY
    } else {
        (b * p - c * q) / (b * c)
    };
    if f > 1. {
        1.
    } else if f < 0. {
        0.
    } else {
        f
    }
}
