//! Awaitable retries within the caller's task, without persistence or rollback.
//!
//! Operations must be safe to retry: cancellation cannot undo external side
//! effects, and this wrapper provides no cross-system atomicity. Operations and
//! predicates must not block, since Tokio timeouts require cooperative yielding.

use std::{future::Future, time::Duration};

/// Immutable configuration; every call to [`Self::run`] has its own retry budget.
#[derive(Clone, Copy, Debug)]
pub struct Retry {
    max_retries: u32,
    timeout: Duration,
    delay: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum RetryError<E> {
    #[error("operation failed: {0}")]
    Failed(#[source] E),
    #[error("retry operation timed out")]
    TimedOut,
}

impl Retry {
    /// `max_retries` excludes the first attempt; zero permits one attempt.
    /// `timeout` covers all attempts and delays; `delay` separates retries.
    pub const fn new(max_retries: u32, timeout: Duration, delay: Duration) -> Self {
        Self {
            max_retries,
            timeout,
            delay,
        }
    }

    /// Run immediately, creating a fresh future for each attempt.
    ///
    /// Returns the last error if retries are exhausted or `should_retry` rejects
    /// it. A zero timeout does not invoke `operation`. Dropping this future or
    /// reaching the deadline drops the current attempt and stops further retries.
    pub async fn run<T, E, F, Fut, P>(
        &self,
        mut operation: F,
        mut should_retry: P,
    ) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        P: FnMut(&E) -> bool,
    {
        if self.timeout.is_zero() {
            return Err(RetryError::TimedOut);
        }

        tokio::time::timeout(self.timeout, async {
            let mut retries_left = self.max_retries;
            loop {
                match operation().await {
                    Ok(value) => return Ok(value),
                    Err(error) if retries_left == 0 || !should_retry(&error) => {
                        return Err(RetryError::Failed(error));
                    }
                    Err(_) => {
                        retries_left -= 1;
                        tokio::time::sleep(self.delay).await;
                    }
                }
            }
        })
        .await
        .map_err(|_| RetryError::TimedOut)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn retries_transient_failure_and_returns_value() {
        let mut attempts = 0;
        let result = Retry::new(1, Duration::from_secs(1), Duration::from_millis(1))
            .run(
                || {
                    attempts += 1;
                    std::future::ready(if attempts == 1 {
                        Err("transient")
                    } else {
                        Ok(42)
                    })
                },
                |error| *error == "transient",
            )
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn times_out_pending_attempt() {
        let result = Retry::new(3, Duration::from_millis(10), Duration::from_millis(1))
            .run(std::future::pending::<Result<(), &str>>, |_| true)
            .await;

        assert!(matches!(result, Err(RetryError::TimedOut)));
    }
}
