use crate::error::{OptionError, OptionResult};
use crate::math::round_to_fixture_years;
use crate::numeric::normal_cdf;
use crate::types::OptionRight;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarrierType {
    DownIn,
    DownOut,
    UpIn,
    UpOut,
}

fn ensure_finite(name: &str, value: f64) -> OptionResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_math_input",
            format!("{name} must be finite: {value}"),
        ))
    }
}

fn ensure_positive(name: &str, value: f64) -> OptionResult<()> {
    ensure_finite(name, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_math_input",
            format!("{name} must be greater than zero: {value}"),
        ))
    }
}

fn ensure_non_negative(name: &str, value: f64) -> OptionResult<()> {
    ensure_finite(name, value)?;
    if value >= 0.0 {
        Ok(())
    } else {
        Err(OptionError::new(
            "invalid_math_input",
            format!("{name} must be non-negative: {value}"),
        ))
    }
}

fn parse_option_right(option_right: &str) -> OptionResult<OptionRight> {
    OptionRight::from_str(option_right).map_err(|_| {
        OptionError::new(
            "invalid_math_input",
            format!("invalid option right: {option_right}"),
        )
    })
}

fn parse_barrier_type(barrier_type: &str) -> OptionResult<BarrierType> {
    match barrier_type {
        "down_in" => Ok(BarrierType::DownIn),
        "down_out" => Ok(BarrierType::DownOut),
        "up_in" => Ok(BarrierType::UpIn),
        "up_out" => Ok(BarrierType::UpOut),
        _ => Err(OptionError::new(
            "invalid_math_input",
            format!("invalid barrier_type: {barrier_type}"),
        )),
    }
}

fn is_triggered(spot: f64, barrier: f64, barrier_type: BarrierType) -> bool {
    match barrier_type {
        BarrierType::DownIn | BarrierType::DownOut => spot <= barrier,
        BarrierType::UpIn | BarrierType::UpOut => spot >= barrier,
    }
}

fn safe_scaled_term(power_term: f64, probability_term: f64) -> f64 {
    if probability_term == 0.0 {
        0.0
    } else {
        power_term * probability_term
    }
}

struct BarrierKernel {
    spot: f64,
    strike: f64,
    barrier: f64,
    rebate: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
    option_right: OptionRight,
    barrier_type: BarrierType,
}

impl BarrierKernel {
    fn std_deviation(&self) -> f64 {
        self.volatility * self.years.sqrt()
    }

    fn risk_free_discount(&self) -> f64 {
        (-self.rate * self.years).exp()
    }

    fn dividend_discount(&self) -> f64 {
        (-self.dividend_yield * self.years).exp()
    }

    fn mu(&self) -> f64 {
        (self.rate - self.dividend_yield) / (self.volatility * self.volatility) - 0.5
    }

    fn mu_sigma(&self) -> f64 {
        (1.0 + self.mu()) * self.std_deviation()
    }

    fn a(&self, phi: f64) -> f64 {
        let x1 = (self.spot / self.strike).ln() / self.std_deviation() + self.mu_sigma();
        let n1 = normal_cdf(phi * x1);
        let n2 = normal_cdf(phi * (x1 - self.std_deviation()));
        phi * (self.spot * self.dividend_discount() * n1
            - self.strike * self.risk_free_discount() * n2)
    }

    fn b(&self, phi: f64) -> f64 {
        let x2 = (self.spot / self.barrier).ln() / self.std_deviation() + self.mu_sigma();
        let n1 = normal_cdf(phi * x2);
        let n2 = normal_cdf(phi * (x2 - self.std_deviation()));
        phi * (self.spot * self.dividend_discount() * n1
            - self.strike * self.risk_free_discount() * n2)
    }

    fn c(&self, eta: f64, phi: f64) -> f64 {
        let hs = self.barrier / self.spot;
        let pow_hs0 = hs.powf(2.0 * self.mu());
        let pow_hs1 = pow_hs0 * hs * hs;
        let y1 = (self.barrier * hs / self.strike).ln() / self.std_deviation() + self.mu_sigma();
        let n1 = normal_cdf(eta * y1);
        let n2 = normal_cdf(eta * (y1 - self.std_deviation()));
        phi * (self.spot * self.dividend_discount() * safe_scaled_term(pow_hs1, n1)
            - self.strike * self.risk_free_discount() * safe_scaled_term(pow_hs0, n2))
    }

    fn d(&self, eta: f64, phi: f64) -> f64 {
        let hs = self.barrier / self.spot;
        let pow_hs0 = hs.powf(2.0 * self.mu());
        let pow_hs1 = pow_hs0 * hs * hs;
        let y2 = (self.barrier / self.spot).ln() / self.std_deviation() + self.mu_sigma();
        let n1 = normal_cdf(eta * y2);
        let n2 = normal_cdf(eta * (y2 - self.std_deviation()));
        phi * (self.spot * self.dividend_discount() * safe_scaled_term(pow_hs1, n1)
            - self.strike * self.risk_free_discount() * safe_scaled_term(pow_hs0, n2))
    }

    fn e(&self, eta: f64) -> f64 {
        if self.rebate <= 0.0 {
            return 0.0;
        }

        let pow_hs0 = (self.barrier / self.spot).powf(2.0 * self.mu());
        let x2 = (self.spot / self.barrier).ln() / self.std_deviation() + self.mu_sigma();
        let y2 = (self.barrier / self.spot).ln() / self.std_deviation() + self.mu_sigma();
        let n1 = normal_cdf(eta * (x2 - self.std_deviation()));
        let n2 = normal_cdf(eta * (y2 - self.std_deviation()));
        self.rebate * self.risk_free_discount() * (n1 - safe_scaled_term(pow_hs0, n2))
    }

    fn f(&self, eta: f64) -> f64 {
        if self.rebate <= 0.0 {
            return 0.0;
        }

        let mu = self.mu();
        let lambda = (mu * mu + 2.0 * self.rate / (self.volatility * self.volatility)).sqrt();
        let hs = self.barrier / self.spot;
        let pow_plus = hs.powf(mu + lambda);
        let pow_minus = hs.powf(mu - lambda);
        let sigma_sqrt_t = self.std_deviation();
        let z = (self.barrier / self.spot).ln() / sigma_sqrt_t + lambda * sigma_sqrt_t;
        let n1 = normal_cdf(eta * z);
        let n2 = normal_cdf(eta * (z - 2.0 * lambda * sigma_sqrt_t));
        self.rebate * (safe_scaled_term(pow_plus, n1) + safe_scaled_term(pow_minus, n2))
    }

    fn value(&self) -> f64 {
        match self.option_right {
            OptionRight::Call => match self.barrier_type {
                BarrierType::DownIn => {
                    if self.strike >= self.barrier {
                        self.c(1.0, 1.0) + self.e(1.0)
                    } else {
                        self.a(1.0) - self.b(1.0) + self.d(1.0, 1.0) + self.e(1.0)
                    }
                }
                BarrierType::UpIn => {
                    if self.strike >= self.barrier {
                        self.a(1.0) + self.e(-1.0)
                    } else {
                        self.b(1.0) - self.c(-1.0, 1.0) + self.d(-1.0, 1.0) + self.e(-1.0)
                    }
                }
                BarrierType::DownOut => {
                    if self.strike >= self.barrier {
                        self.a(1.0) - self.c(1.0, 1.0) + self.f(1.0)
                    } else {
                        self.b(1.0) - self.d(1.0, 1.0) + self.f(1.0)
                    }
                }
                BarrierType::UpOut => {
                    if self.strike >= self.barrier {
                        self.f(-1.0)
                    } else {
                        self.a(1.0) - self.b(1.0) + self.c(-1.0, 1.0) - self.d(-1.0, 1.0)
                            + self.f(-1.0)
                    }
                }
            },
            OptionRight::Put => match self.barrier_type {
                BarrierType::DownIn => {
                    if self.strike >= self.barrier {
                        self.b(-1.0) - self.c(1.0, -1.0) + self.d(1.0, -1.0) + self.e(1.0)
                    } else {
                        self.a(-1.0) + self.e(1.0)
                    }
                }
                BarrierType::UpIn => {
                    if self.strike >= self.barrier {
                        self.a(-1.0) - self.b(-1.0) + self.d(-1.0, -1.0) + self.e(-1.0)
                    } else {
                        self.c(-1.0, -1.0) + self.e(-1.0)
                    }
                }
                BarrierType::DownOut => {
                    if self.strike >= self.barrier {
                        self.a(-1.0) - self.b(-1.0) + self.c(1.0, -1.0) - self.d(1.0, -1.0)
                            + self.f(1.0)
                    } else {
                        self.f(1.0)
                    }
                }
                BarrierType::UpOut => {
                    if self.strike >= self.barrier {
                        self.b(-1.0) - self.d(-1.0, -1.0) + self.f(-1.0)
                    } else {
                        self.a(-1.0) - self.c(-1.0, -1.0) + self.f(-1.0)
                    }
                }
            },
        }
    }
}

pub fn price(
    spot: f64,
    strike: f64,
    barrier: f64,
    rebate: f64,
    years: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
    option_right: &str,
    barrier_type: &str,
) -> OptionResult<f64> {
    ensure_positive("spot", spot)?;
    ensure_positive("strike", strike)?;
    ensure_positive("barrier", barrier)?;
    ensure_non_negative("rebate", rebate)?;
    ensure_positive("years", years)?;
    ensure_finite("rate", rate)?;
    ensure_finite("dividend_yield", dividend_yield)?;
    ensure_positive("volatility", volatility)?;
    let option_right = parse_option_right(option_right)?;
    let barrier_type = parse_barrier_type(barrier_type)?;
    let years = round_to_fixture_years(years);

    if is_triggered(spot, barrier, barrier_type) {
        return Err(OptionError::new(
            "invalid_math_input",
            "barrier touched or crossed at valuation spot",
        ));
    }

    Ok(BarrierKernel {
        spot,
        strike,
        barrier,
        rebate,
        years,
        rate,
        dividend_yield,
        volatility,
        option_right,
        barrier_type,
    }
    .value())
}
