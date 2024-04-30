use std::cmp::Ordering;
pub const DAYS_PER_YEAR: f64 = 360.;

pub enum SIDE {
    LEFT,
    RIGHT,
}

/// Find indice of element in the sorted array by provided key function.
///
/// Assuming that `a` is sorted by `key`:
///
///  ------ | ----------------------------
///  `side` | returned index `i` satisfies
///  ------ | ----------------------------
///  left   | ``key(a[i-1]) < v <= key(a[i])``
///  right  | ``key(a[i-1]) <= v < key(a[i])``
///  ------ | ----------------------------
///
/// Note that if `a` is unsorted, the result may be ambigious.
pub(crate) fn search_sorted<T, U: Ord>(
    a: &[T],
    v: &U,
    key: impl Fn(&T) -> U,
    side: Option<SIDE>,
) -> usize {
    if a.is_empty() {
        return 0;
    }
    let side = side.unwrap_or(SIDE::LEFT);
    let (mut lo, mut hi) = (0, a.len() - 1);
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        match key(&a[mid]).cmp(v) {
            Ordering::Less => lo = mid,
            Ordering::Greater => hi = mid,
            Ordering::Equal => match side {
                SIDE::LEFT => hi = mid,
                SIDE::RIGHT => lo = mid,
            },
        }
    }
    let lv = key(&a[lo]);
    let rv = key(&a[hi]);
    // if v in a
    if *v == lv || *v == rv {
        match side {
            SIDE::LEFT => {
                if *v == lv {
                    lo
                } else {
                    hi
                }
            }

            SIDE::RIGHT => {
                if *v == rv {
                    lo
                } else {
                    hi
                }
            }
        }
    }
    // if v not in a
    else if *v < lv {
        lo
    } else if *v > rv {
        hi + 1
    } else {
        hi
    }
}

/// Calaulate internal rate of return.
///
/// An gradient-based iteration method is used for solving internal
/// rate of return (IRR). The IRR is represented as:
/// ```ignore
///     end_value = sum(investment_i * (1 + x) ** t_i)
/// ```
/// where investment_i is the items in `investment` and `t_i` is the
/// duration of the date of `investment_i` to `end_value` in years,
/// and `x` is the IRR value to be solved. `x` is in the interval of
/// (-1, +∞), its initial value is given by `x0`. To make the solution
/// always in this interval, let `x = exp(p) - 1`, thus p ∈ (-∞, ∞):
/// ```ignore
///     end_value = sum(investment_i * exp(p * t_i))
/// ```
/// and its derivation is:
/// ```ignore
///     d(end_value) / d(p) = sum(investment_i * t_i * exp(p * t_i))
/// ```
pub(crate) fn irr(
    days_array: &[f64],
    investment_array: &[f64],
    end_value: f64,
    x0: f64,
) -> Option<f64> {
    // Avoid sigularity.
    let mut all_zeros = true;
    for i in investment_array {
        if *i != 0. {
            all_zeros = false;
            break;
        }
    }
    if all_zeros {
        return None;
    }

    let t_list = days_array
        .iter()
        .map(|x| x / DAYS_PER_YEAR)
        .collect::<Vec<_>>();

    let f = |p: f64| -> f64 {
        end_value
            - t_list
                .iter()
                .zip(investment_array)
                .map(|(&t, &x)| x * f64::exp(p * t))
                .sum::<f64>()
    };
    let g = |p: f64| {
        t_list
            .iter()
            .zip(investment_array)
            .map(|(&t, &x)| -x * t * f64::exp(p * t))
            .sum()
    };
    let p0 = f64::ln(x0 + 1.);
    let p = newton1d(f, g, p0, 1e-6, 1000)?;
    Some(f64::exp(p) - 1.)
}

/// Find root of function using Newton's method.
///
/// The Newton's method uses the target funciton and its derivation to
/// find the root of the function.
///
/// # Arguments
///
/// * `f` - The target function which takes exactly one argument.
/// * `d` - The derivation of `f`.
/// * `x0` - The initial guess of the root.
/// * `tol` - The absolute tolerance for root finding.
/// * `maxiter` - The maximum number of iterations to find the root.
pub(crate) fn newton1d(
    f: impl Fn(f64) -> f64,
    d: impl Fn(f64) -> f64,
    x0: f64,
    tol: f64,
    maxiter: usize,
) -> Option<f64> {
    let mut x = x0;
    for _ in 0..maxiter {
        let new_x = x - f(x) / d(x);
        if (new_x - x).abs() < tol {
            return Some(new_x);
        }
        x = new_x;
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_search_sorted() {
        let a = vec![2, 4, 6, 8, 10, 12, 14, 16];
        let idx1 = search_sorted(&a, &9, |&x| x, None);
        let idx2 = search_sorted(&a, &9, |&x| x, Some(SIDE::RIGHT));
        let idx3 = search_sorted(&a, &8, |&x| x, None);
        let idx4 = search_sorted(&a, &8, |&x| x, Some(SIDE::RIGHT));
        let idx5 = search_sorted(&a, &0, |&x| x, None);
        let idx6 = search_sorted(&a, &0, |&x| x, Some(SIDE::RIGHT));
        let idx7 = search_sorted(&a, &20, |&x| x, None);
        let idx8 = search_sorted(&a, &20, |&x| x, Some(SIDE::RIGHT));
        assert!(idx1 == 4);
        assert!(idx2 == 4);
        assert!(idx3 == 3);
        assert!(idx4 == 4);
        assert!(idx5 == 0);
        assert!(idx6 == 0);
        assert!(idx7 == 8);
        assert!(idx8 == 8);
    }

    #[test]
    fn test_newton1d() {
        fn f(x: f64) -> f64 {
            x * x + 2. * x + 1.
        }
        fn g(x: f64) -> f64 {
            2. * x + 2.
        }
        let x0 = newton1d(f, g, 0.0, 1e-6, 100).unwrap();
        assert!((x0 + 1.).abs() < 1e-3);
    }

    #[test]
    fn test_irr() {
        let days_array = [720., 360., 0.];
        let investment_array = [1., 2., 0.];
        let end_value = 8.;
        let x0 = 0.0;
        let res = irr(&days_array, &investment_array, end_value, x0).unwrap();
        assert!((res - 1.).abs() < 1e-2);
    }
}
