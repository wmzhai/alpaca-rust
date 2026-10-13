use crate::error::{OptionError, OptionResult};
use alpaca_core::float;

const SQRT_2: f64 = 1.414_213_562_373_095_1;
const SQRT_2PI: f64 = 2.506_628_274_631_000_2;
const SQRT_PI: f64 = 1.772_453_850_905_516;
const ERF_EPSILON: f64 = 1e-18;
const ERF_MAX_ITERATIONS: usize = 200;

fn ensure_finite(name: &str, value: f64) -> OptionResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_numeric_input",
            format!("{name} must be finite: {value}"),
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeExtrema {
    pub min_spot: f64,
    pub min_value: f64,
    pub max_spot: f64,
    pub max_value: f64,
}

fn erf_series(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let abs_x = x.abs();
    let mut term = abs_x;
    let mut total = abs_x;

    for n in 1..ERF_MAX_ITERATIONS {
        term *= -(abs_x * abs_x) / n as f64;
        let delta = term / (2 * n + 1) as f64;
        total += delta;
        if delta.abs() < ERF_EPSILON {
            break;
        }
    }

    sign * 2.0 * total / SQRT_PI
}

fn normal_cdf_tail(x: f64) -> f64 {
    let mut fraction = 0.0;
    for k in (1..=ERF_MAX_ITERATIONS).rev() {
        fraction = k as f64 / (x + fraction);
    }
    normal_pdf(x) / (x + fraction)
}

pub fn normal_cdf(x: f64) -> f64 {
    if x == 0.0 {
        return 0.5;
    }

    if x.abs() <= 4.0 {
        return 0.5 * (1.0 + erf_series(x / SQRT_2));
    }

    let tail = normal_cdf_tail(x.abs());
    if x > 0.0 { 1.0 - tail } else { tail }
}

pub fn normal_pdf(x: f64) -> f64 {
    (-0.5 * x * x).exp() / SQRT_2PI
}

// Positive half of the 16-point Gauss-Legendre rule on [-1, 1].
const GAUSS_16_NODE: [f64; 8] = [
    0.095_012_509_837_637_69,
    0.281_603_550_779_258_9,
    0.458_016_777_657_227_1,
    0.617_876_244_402_643_9,
    0.755_404_408_355_003,
    0.865_631_202_387_831_6,
    0.944_575_023_073_232_4,
    0.989_400_934_991_650_3,
];
const GAUSS_16_WEIGHT: [f64; 8] = [
    0.189_450_610_455_068_47,
    0.182_603_415_044_923_56,
    0.169_156_519_395_002_62,
    0.149_595_988_816_576_68,
    0.124_628_971_255_533_92,
    0.095_158_511_682_492_83,
    0.062_253_523_938_648_23,
    0.027_152_459_411_753_09,
];

/// P(X <= x, Y <= y) for standard normals with correlation `rho`.
///
/// Uses the Drezner-Wesolowsky arcsine integral with 16-point Gauss-Legendre
/// quadrature. Bjerksund-Stensland 2002 calls this with correlation near ±0.786.
pub fn bivariate_normal_cdf(x: f64, y: f64, rho: f64) -> f64 {
    if !x.is_finite() || !y.is_finite() || !rho.is_finite() {
        return f64::NAN;
    }
    if rho >= 0.999_999 {
        return normal_cdf(x.min(y));
    }
    if rho <= -0.999_999 {
        return (normal_cdf(x) - normal_cdf(-y)).max(0.0);
    }
    if x <= -8.0 || y <= -8.0 {
        return 0.0;
    }
    if x >= 8.0 && y >= 8.0 {
        return 1.0;
    }

    let angle = rho.asin();
    let half_square = 0.5 * (x * x + y * y);
    let product = x * y;
    let mut sum = 0.0;
    for index in 0..GAUSS_16_NODE.len() {
        let node = GAUSS_16_NODE[index];
        let weight = GAUSS_16_WEIGHT[index];
        sum += weight * bivariate_angle_density(angle * (node + 1.0) * 0.5, product, half_square);
        sum += weight * bivariate_angle_density(angle * (1.0 - node) * 0.5, product, half_square);
    }
    (normal_cdf(x) * normal_cdf(y) + angle * sum / (4.0 * std::f64::consts::PI)).clamp(0.0, 1.0)
}

fn bivariate_angle_density(theta: f64, product: f64, half_square: f64) -> f64 {
    let cosine = theta.cos();
    let denominator = cosine * cosine;
    if denominator <= 1e-18 {
        return 0.0;
    }
    ((theta.sin() * product - half_square) / denominator).exp()
}

pub fn round(value: f64, decimals: u32) -> OptionResult<f64> {
    ensure_finite("value", value)?;
    Ok(float::round(value, decimals))
}

pub fn linspace(start: f64, end: f64, count: usize) -> OptionResult<Vec<f64>> {
    ensure_finite("start", start)?;
    ensure_finite("end", end)?;
    if count == 0 {
        return Err(OptionError::new(
            "invalid_numeric_input",
            "count must be greater than zero",
        ));
    }
    if count == 1 {
        return Ok(vec![start]);
    }

    let step = (end - start) / (count - 1) as f64;
    Ok((0..count)
        .map(|index| start + step * index as f64)
        .collect())
}

fn validate_brent_params(
    lower_bound: f64,
    upper_bound: f64,
    tolerance: Option<f64>,
    max_iterations: Option<usize>,
) -> OptionResult<(f64, usize)> {
    ensure_finite("lower_bound", lower_bound)?;
    ensure_finite("upper_bound", upper_bound)?;
    if lower_bound >= upper_bound {
        return Err(OptionError::new(
            "invalid_numeric_input",
            format!("lower_bound must be less than upper_bound: {lower_bound} >= {upper_bound}"),
        ));
    }

    let tolerance = tolerance.unwrap_or(1e-10);
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(OptionError::new(
            "invalid_numeric_input",
            format!("tolerance must be positive: {tolerance}"),
        ));
    }

    let max_iterations = max_iterations.unwrap_or(100);
    if max_iterations == 0 {
        return Err(OptionError::new(
            "invalid_numeric_input",
            "max_iterations must be greater than zero",
        ));
    }

    Ok((tolerance, max_iterations))
}

fn brent_solve_impl<F>(
    lower_bound: f64,
    upper_bound: f64,
    evaluate: F,
    tolerance: Option<f64>,
    max_iterations: Option<usize>,
) -> OptionResult<f64>
where
    F: Fn(f64) -> OptionResult<f64>,
{
    let (tolerance, max_iterations) =
        validate_brent_params(lower_bound, upper_bound, tolerance, max_iterations)?;

    let mut a = lower_bound;
    let mut b = upper_bound;
    let mut fa = evaluate(a)?;
    let mut fb = evaluate(b)?;
    ensure_finite("f(lower_bound)", fa)?;
    ensure_finite("f(upper_bound)", fb)?;

    if fa.abs() <= tolerance {
        return Ok(a);
    }
    if fb.abs() <= tolerance {
        return Ok(b);
    }
    if fa * fb > 0.0 {
        return Err(OptionError::new(
            "root_not_bracketed",
            format!("root is not bracketed: f({a})={fa}, f({b})={fb}"),
        ));
    }

    if fa.abs() < fb.abs() {
        std::mem::swap(&mut a, &mut b);
        std::mem::swap(&mut fa, &mut fb);
    }

    let mut c = a;
    let mut fc = fa;
    let mut d = b - a;
    let mut mflag = true;

    for _ in 0..max_iterations {
        let mut s = if fa != fc && fb != fc {
            a * fb * fc / ((fa - fb) * (fa - fc))
                + b * fa * fc / ((fb - fa) * (fb - fc))
                + c * fa * fb / ((fc - fa) * (fc - fb))
        } else {
            b - fb * (b - a) / (fb - fa)
        };

        let lower_window = (3.0 * a + b) / 4.0;
        let outside_window = if a < b {
            s <= lower_window || s >= b
        } else {
            s >= lower_window || s <= b
        };
        let cond2 = mflag && (s - b).abs() >= (b - c).abs() / 2.0;
        let cond3 = !mflag && (s - b).abs() >= (c - d).abs() / 2.0;
        let cond4 = mflag && (b - c).abs() < tolerance;
        let cond5 = !mflag && (c - d).abs() < tolerance;

        if outside_window || cond2 || cond3 || cond4 || cond5 {
            s = (a + b) / 2.0;
            mflag = true;
        } else {
            mflag = false;
        }

        let fs = evaluate(s)?;
        ensure_finite("f(candidate)", fs)?;

        d = c;
        c = b;
        fc = fb;

        if fa * fs < 0.0 {
            b = s;
            fb = fs;
        } else {
            a = s;
            fa = fs;
        }

        if fa.abs() < fb.abs() {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut fa, &mut fb);
        }

        if fb.abs() <= tolerance || (b - a).abs() <= tolerance {
            return Ok(b);
        }
    }

    Err(OptionError::new(
        "root_not_converged",
        format!("root solver did not converge in {max_iterations} iterations"),
    ))
}

pub fn brent_solve<F>(
    lower_bound: f64,
    upper_bound: f64,
    evaluate: F,
    tolerance: Option<f64>,
    max_iterations: Option<usize>,
) -> OptionResult<f64>
where
    F: Fn(f64) -> f64,
{
    brent_solve_impl(
        lower_bound,
        upper_bound,
        |spot| Ok(evaluate(spot)),
        tolerance,
        max_iterations,
    )
}

pub fn refine_bracketed_root<F>(
    lower_bound: f64,
    upper_bound: f64,
    evaluate: F,
    tolerance: Option<f64>,
    max_iterations: Option<usize>,
) -> OptionResult<f64>
where
    F: Fn(f64) -> OptionResult<f64>,
{
    brent_solve_impl(
        lower_bound,
        upper_bound,
        evaluate,
        tolerance,
        max_iterations,
    )
}

pub fn evaluate_points<F>(points: &[f64], evaluate: F) -> OptionResult<Vec<f64>>
where
    F: Fn(f64) -> OptionResult<f64>,
{
    let mut values = Vec::with_capacity(points.len());
    for point in points {
        ensure_finite("point", *point)?;
        let value = evaluate(*point)?;
        ensure_finite("f(point)", value)?;
        values.push(value);
    }
    Ok(values)
}

pub fn scan_range_extrema<F>(
    lower_bound: f64,
    upper_bound: f64,
    step: Option<f64>,
    evaluate: F,
) -> OptionResult<RangeExtrema>
where
    F: Fn(f64) -> OptionResult<f64>,
{
    ensure_finite("lower_bound", lower_bound)?;
    ensure_finite("upper_bound", upper_bound)?;
    if lower_bound > upper_bound {
        return Err(OptionError::new(
            "invalid_numeric_input",
            format!(
                "lower_bound must be less than or equal to upper_bound: {lower_bound} > {upper_bound}"
            ),
        ));
    }

    let step = step.unwrap_or(1.0);
    if !step.is_finite() || step <= 0.0 {
        return Err(OptionError::new(
            "invalid_numeric_input",
            format!("step must be positive: {step}"),
        ));
    }

    let mut spot = lower_bound;
    let mut value = evaluate(spot)?;
    ensure_finite("f(lower_bound)", value)?;

    let mut extrema = RangeExtrema {
        min_spot: spot,
        min_value: value,
        max_spot: spot,
        max_value: value,
    };

    while spot < upper_bound {
        spot = (spot + step).min(upper_bound);
        value = evaluate(spot)?;
        ensure_finite("f(point)", value)?;
        if value < extrema.min_value {
            extrema.min_value = value;
            extrema.min_spot = spot;
        }
        if value > extrema.max_value {
            extrema.max_value = value;
            extrema.max_spot = spot;
        }
    }

    Ok(extrema)
}
