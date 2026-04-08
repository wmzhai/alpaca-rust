#![forbid(unsafe_code)]

/// Phase 1 placeholder for shared primitives.
pub const CRATE_NAME: &str = "alpaca-core";

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;

    #[test]
    fn crate_name_is_stable() {
        assert_eq!(CRATE_NAME, "alpaca-core");
    }
}
