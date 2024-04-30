use crate::DAYS_PER_YEAR;
use crate::{TransactionIterator, Weekday};
use chrono::Datelike;
use ndarray::{s, Array, Array1};

#[derive(Debug)]
pub struct KellyError(&'static str);

impl std::fmt::Display for KellyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "KellyError: {}", self.0)
    }
}

impl std::error::Error for KellyError {}

pub struct KellyIndicator {
    pub position: f64,
    pub upper_bound: f64,
    pub lower_bound: f64,
    pub upper_risk_bound: f64,
    pub lower_risk_bound: f64,
}

/// Get the position, upper and lower bounds using Kelly strategy.
///
/// This function provides a detailed inspect into the Kelly strategy
/// during iteration. For a TransactionIterator object, giving the
/// index of the selected fund and other parameters, it returns the
/// current position, the limits where kelly strategy holds a position
/// within 0% to 100% and the risk bounds.
pub fn kelly_hint(
    it: &TransactionIterator,
    fund_index: usize,
    weekday: Weekday,
    n: usize,
    inflation: f64,
    risk_bound: f64,
) -> Result<KellyIndicator, Box<dyn std::error::Error>> {
    if it.navs().shape()[0] < n {
        return Err(Box::new(KellyError("n too large for kelly strategy")));
    }
    let inflation_array = Array::linspace((n - 1) as f64, 0., n);
    let inflation_array = inflation_array.mapv(|x| (1. + inflation).powf(x / DAYS_PER_YEAR));
    let navs = it.navs();
    // Net asset value of the last n days.
    let y0 = navs.slice(s![-(n as isize).., fund_index as isize]);
    let y = &y0 * &inflation_array;
    // Get winning rate.
    let y_weekly = y
        .iter()
        .zip(&it.dates()[it.ndays() - n..])
        .filter(|(_yi, &di)| di.weekday() == weekday)
        .map(|(&yi, _di)| yi)
        .collect::<Array1<_>>();
    let dy = &y_weekly.slice(s![1..]) - &y_weekly.slice(s![..-1]);
    let p = dy.iter().filter(|&&x| x > 0.).count() as f64 / dy.len() as f64;
    let q = 1. - p;

    let y_max = *y
        .iter()
        .max_by(|&x, &y| f64::total_cmp(x, y))
        .expect("fail to find maximal NAV");
    let y_min = *y
        .iter()
        .min_by(|&x, &y| f64::total_cmp(x, y))
        .expect("fail to find minimal NAV");
    let y0_max = *y0
        .iter()
        .max_by(|&x, &y| f64::total_cmp(x, y))
        .expect("fail to find maximal NAV");
    let y0_min = *y0
        .iter()
        .min_by(|&x, &y| f64::total_cmp(x, y))
        .expect("fail to find minimal NAV");

    // Kelly.
    let position = get_kelly_position(*y.last().unwrap(), y_max, y_min, p);
    // Risk control.
    let position = risk_control(position, *y0.last().unwrap(), y0_max, y0_min, risk_bound);

    let upper_bound = y_max * p + y_min * q;
    let lower_bound = y_max * y_min / (y_max * q + y_min * p);
    let bound_width = (y0_max - y0_min) * risk_bound;
    let upper_risk_bound = y0_max - bound_width;
    let lower_risk_bound = y0_min + bound_width;

    Ok(KellyIndicator {
        position,
        upper_bound,
        lower_bound,
        upper_risk_bound,
        lower_risk_bound,
    })
}

/// The Kelly strategy transacts weekly.
pub fn kelly_weekly(
    it: &mut TransactionIterator,
    weekday: Weekday,
    ns: &[usize],
    inflations: &[f64],
    risk_bounds: &[f64],
) -> Result<(), Box<dyn std::error::Error>> {
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
    let mut weekday_cache = it.dates().iter().map(|d| d.weekday()).collect::<Vec<_>>();

    while it.next_weekday(Some(weekday)).is_some() {
        weekday_cache.extend(
            it.dates()[weekday_cache.len()..]
                .iter()
                .map(|d| d.weekday()),
        );
        for j in 0..it.nfunds() {
            let navs = it.navs();
            // Net asset value of the last n days.
            let y0 = navs.slice(s![-(ns[j] as isize).., j as isize]);
            // Net asset value considering inflation: y = y0 * (1 + inflation) ** number_of_years_to_today
            let y = &y0 * &inflation_arrays[j];
            // Get winning rate.
            let y_weekly = y
                .iter()
                .zip(&weekday_cache[weekday_cache.len() - ns[j]..])
                .filter(|(_yi, &di)| di == weekday)
                .map(|(&yi, _di)| yi)
                .collect::<Array1<_>>();
            let dy = &y_weekly.slice(s![1..]) - &y_weekly.slice(s![..-1]);
            let p = dy.iter().filter(|&&x| x > 0.).count() as f64 / dy.len() as f64;

            // Kelly.
            let f = get_kelly_position(
                *y.last().unwrap(),
                *y.iter()
                    .max_by(|&x, &y| f64::total_cmp(x, y))
                    .expect("fail to find maximal NAV"),
                *y.iter()
                    .min_by(|&x, &y| f64::total_cmp(x, y))
                    .expect("fail to find minimal NAV"),
                p,
            );
            // Risk control.
            let f = risk_control(
                f,
                *y0.last().unwrap(),
                *y0.iter()
                    .max_by(|&x, &y| f64::total_cmp(x, y))
                    .expect("fail to find maximal NAV"),
                *y0.iter()
                    .min_by(|&x, &y| f64::total_cmp(x, y))
                    .expect("fail to find minimal NAV"),
                risk_bounds[j],
            );

            // Adjust position
            let total = it.asset() / it.nfunds() as f64 * f;
            let current = it.fund_asset(j);
            let amount = total - current;
            it.buy_comment(j, amount, 0.0, &format!("position = {:.2}%", 100. * f))?;
        }
    }
    Ok(())
}

/// Calculate the position given by kelly startegy.
///
/// The expected income when win is estimated by the current position
/// and the maximal net value in history. The expected loss is
/// estimated by the current position and the minimal net value in
/// history.
///
/// # Arguments
///
/// * `y` - The net assert values of the past.
/// * `p` - Estimated winning rate.
fn get_kelly_position(current_y: f64, max_y: f64, min_y: f64, p: f64) -> f64 {
    let b = max_y / current_y - 1.;
    let c = 1. - min_y / current_y;
    kelly_equation(p, b, c)
}

/// Adjust position to control risk.
///
/// A simple risk control strategy. If the current NAV is very low,
/// adjust position to zero. If the current NAV is very high, adjust
/// position to 1.0.
///
/// # Arguments
///
/// * `f` - The reference position.
/// * `y` - The net asset values of the past.
/// * `risk_bound` - The bound for controlling risk.
fn risk_control(f: f64, current_y: f64, max_y: f64, min_y: f64, risk_bound: f64) -> f64 {
    let bound_width = (max_y - min_y) * risk_bound;
    // let current_y = *y.last().unwrap();
    if current_y >= max_y - bound_width {
        1.
    } else if current_y <= min_y + bound_width {
        0.
    } else {
        f
    }
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
    f.clamp(0., 1.)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn test_kelly_1() {
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("20170101", "%Y%m%d").unwrap();
        let end_date = NaiveDate::parse_from_str("20240101", "%Y%m%d").unwrap();
        let trans = Transaction::new(&[&hs300, &gz2000], None, Some(end_date));

        let ns = [1300, 1600];
        let inflations = [0.015, 0.015];
        let risk_bounds = [0.01, 0.01];
        let mut results = Vec::new();
        for save_log in [true, false] {
            for save_record in [true, false] {
                let mut res = Vec::new();
                for weekday in 0..5 {
                    let mut it = trans.iter(save_log, save_record);
                    it.goto(start_date);
                    it.inflow(1.).unwrap();
                    kelly_weekly(
                        &mut it,
                        Weekday::try_from(weekday).unwrap(),
                        &ns,
                        &inflations,
                        &risk_bounds,
                    )
                    .unwrap();
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
    fn test_kelly_2() {
        let hs300 = Fund::from(&read_gta("hs300.txt").unwrap());
        let gz2000 = Fund::from(&read_gta("gz2000.txt").unwrap());
        let start_date = NaiveDate::parse_from_str("20170101", "%Y%m%d").unwrap();
        let end_date = NaiveDate::parse_from_str("20240101", "%Y%m%d").unwrap();
        let trans = Transaction::new(&[&hs300, &gz2000], None, Some(end_date));

        let ns = [1300, 1600];
        let inflations = [0.015, 0.015];
        let risk_bounds = [0.01, 0.01];
        let mut result = Vec::new();
        for weekday in 0..5 {
            let mut it = trans.iter(false, false);
            it.goto(start_date);
            it.inflow(1.).unwrap();
            kelly_weekly(
                &mut it,
                Weekday::try_from(weekday).unwrap(),
                &ns,
                &inflations,
                &risk_bounds,
            )
            .unwrap();
            result.push(it.asset());
        }

        assert!((result[0] - 1.538617807495912).abs() < 1e-6);
        assert!((result[1] - 1.6655186489198273).abs() < 1e-6);
        assert!((result[2] - 1.5012221777553958).abs() < 1e-6);
        assert!((result[3] - 1.489728842992303).abs() < 1e-6);
        assert!((result[4] - 1.3982690518133718).abs() < 1e-6);
    }
}
