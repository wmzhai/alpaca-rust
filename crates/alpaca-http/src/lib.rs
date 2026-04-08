#![forbid(unsafe_code)]

/// Phase 1 placeholder for the internal HTTP transport crate.
pub const CRATE_NAME: &str = "alpaca-http";

#[cfg(test)]
mod tests {
    use super::CRATE_NAME;

    #[test]
    fn crate_name_is_stable() {
        assert_eq!(CRATE_NAME, "alpaca-http");
    }
}
