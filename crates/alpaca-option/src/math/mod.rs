pub mod american;
pub mod barrier;
pub mod bachelier;
pub mod black76;
pub mod geometric_asian;

pub(crate) fn round_to_fixture_years(years: f64) -> f64 {
    // option-test projects maturities onto an Actual365Fixed grid via Python's banker's rounding.
    let scaled = years * 365.0;
    let base = scaled.floor();
    let fraction = scaled - base;

    let rounded_days = if fraction < 0.5 {
        base
    } else if fraction > 0.5 {
        base + 1.0
    } else if (base as i64) % 2 == 0 {
        base
    } else {
        base + 1.0
    };

    rounded_days / 365.0
}
