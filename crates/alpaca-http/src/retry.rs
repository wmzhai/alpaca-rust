use std::time::Duration;

use reqwest::{Method, StatusCode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryConfig {
    retryable_methods: Vec<Method>,
    max_retries: u32,
    retry_on_429: bool,
    respect_retry_after: bool,
    base_backoff: Duration,
    max_backoff: Duration,
    total_retry_budget: Option<Duration>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryDecision {
    DoNotRetry,
    RetryAfter(Duration),
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            retryable_methods: vec![Method::GET],
            max_retries: 3,
            retry_on_429: false,
            respect_retry_after: false,
            base_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_secs(5),
            total_retry_budget: None,
        }
    }
}

impl RetryConfig {
    #[must_use]
    pub fn with_retryable_methods<I>(mut self, methods: I) -> Self
    where
        I: IntoIterator<Item = Method>,
    {
        self.retryable_methods = methods.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_retry_on_429(mut self, retry_on_429: bool) -> Self {
        self.retry_on_429 = retry_on_429;
        self
    }

    #[must_use]
    pub fn with_respect_retry_after(mut self, respect_retry_after: bool) -> Self {
        self.respect_retry_after = respect_retry_after;
        self
    }

    #[must_use]
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    #[must_use]
    pub fn classify_response(
        &self,
        method: &Method,
        status: StatusCode,
        attempt: u32,
        retry_after: Option<Duration>,
        elapsed: Duration,
    ) -> RetryDecision {
        if attempt >= self.max_retries || !self.retryable_methods.iter().any(|item| item == method)
        {
            return RetryDecision::DoNotRetry;
        }

        let wait = if status == StatusCode::TOO_MANY_REQUESTS {
            if !self.retry_on_429 {
                return RetryDecision::DoNotRetry;
            }

            if self.respect_retry_after {
                retry_after.unwrap_or_else(|| self.backoff(attempt + 1))
            } else {
                self.backoff(attempt + 1)
            }
        } else if status.is_server_error() {
            self.backoff(attempt + 1)
        } else {
            return RetryDecision::DoNotRetry;
        };

        let wait = wait.min(self.max_backoff);

        if let Some(total_retry_budget) = self.total_retry_budget {
            let Some(remaining_budget) = total_retry_budget.checked_sub(elapsed) else {
                return RetryDecision::DoNotRetry;
            };
            if remaining_budget.is_zero() {
                return RetryDecision::DoNotRetry;
            }
            return RetryDecision::RetryAfter(wait.min(remaining_budget));
        }

        RetryDecision::RetryAfter(wait)
    }

    #[must_use]
    pub fn classify_transport_error(
        &self,
        method: &Method,
        attempt: u32,
        elapsed: Duration,
    ) -> RetryDecision {
        if attempt >= self.max_retries || !self.retryable_methods.iter().any(|item| item == method)
        {
            return RetryDecision::DoNotRetry;
        }

        let wait = self.backoff(attempt + 1).min(self.max_backoff);

        if let Some(total_retry_budget) = self.total_retry_budget {
            let Some(remaining_budget) = total_retry_budget.checked_sub(elapsed) else {
                return RetryDecision::DoNotRetry;
            };
            if remaining_budget.is_zero() {
                return RetryDecision::DoNotRetry;
            }
            return RetryDecision::RetryAfter(wait.min(remaining_budget));
        }

        RetryDecision::RetryAfter(wait)
    }

    fn backoff(&self, attempt: u32) -> Duration {
        let factor = 1u32
            .checked_shl(attempt.saturating_sub(1))
            .unwrap_or(u32::MAX);
        let millis = self.base_backoff.as_millis();
        let scaled = millis.saturating_mul(u128::from(factor));
        let bounded = scaled.min(self.max_backoff.as_millis());
        Duration::from_millis(u64::try_from(bounded).unwrap_or(u64::MAX))
    }
}
