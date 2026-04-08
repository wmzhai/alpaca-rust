#![forbid(unsafe_code)]

pub const BINARY_NAME: &str = "alpaca-mock";

pub fn startup_message() -> &'static str {
    "alpaca-mock bootstrap"
}

#[cfg(test)]
mod tests {
    use super::{startup_message, BINARY_NAME};

    #[test]
    fn exposes_startup_message() {
        assert_eq!(BINARY_NAME, "alpaca-mock");
        assert_eq!(startup_message(), "alpaca-mock bootstrap");
    }
}
