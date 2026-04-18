pub fn round(value: f64, scale: u32) -> f64 {
    if !value.is_finite() {
        return value;
    }

    let multiplier = 10_f64.powi(scale as i32);
    (value * multiplier).round() / multiplier
}
