use crate::Error;

pub fn non_empty_string(label: &str, value: impl Into<String>) -> Result<String, Error> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(Error::InvalidConfiguration(format!(
            "{label} must not be empty or whitespace"
        )));
    }

    Ok(value)
}

pub fn valid_env_name(label: &str, value: &str) -> Result<(), Error> {
    if value.trim().is_empty() {
        return Err(Error::InvalidConfiguration(format!(
            "{label} must not be empty or whitespace"
        )));
    }

    Ok(())
}

pub fn valid_header_value(label: &str, value: &str) -> Result<(), Error> {
    if value.bytes().any(|byte| byte < 32 || byte == 127) {
        return Err(Error::InvalidConfiguration(format!(
            "{label} must be a valid HTTP header value"
        )));
    }

    Ok(())
}
