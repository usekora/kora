use anyhow::Result;
use std::path::Path;
use std::time::Duration;

use super::traits::AgentOutput;
use super::Provider;

/// Default max retries for rate-limited requests.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default base delay in seconds for exponential backoff.
pub const DEFAULT_BASE_DELAY_SECS: u64 = 2;

/// Check if an error message or agent output indicates a rate limit.
pub fn is_rate_limit_error(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("rate limit")
        || lower.contains("rate_limit")
        || lower.contains("429")
        || lower.contains("too many requests")
        || lower.contains("overloaded")
        || lower.contains("resource_exhausted")
        || lower.contains("capacity")
}

/// Run a provider with exponential backoff retry on rate limit errors.
/// On non-rate-limit failures or success, returns immediately.
/// On rate limit: waits base_delay * 2^attempt seconds, then retries.
pub async fn run_with_retry(
    provider: &dyn Provider,
    prompt: &str,
    working_dir: &Path,
    extra_flags: &[String],
    timeout: Option<Duration>,
    max_retries: u32,
    base_delay_secs: u64,
) -> Result<AgentOutput> {
    let mut last_error: Option<anyhow::Error> = None;

    for attempt in 0..=max_retries {
        let remaining = max_retries - attempt;

        match provider
            .run(prompt, working_dir, extra_flags, timeout)
            .await
        {
            Ok(output) => {
                if output.exit_code == 0 {
                    return Ok(output);
                }

                if is_rate_limit_error(&output.text) && remaining > 0 {
                    let delay = base_delay_secs * 2u64.pow(attempt);
                    eprintln!(
                        "Rate limit detected (attempt {}/{}), retrying in {}s...",
                        attempt + 1,
                        max_retries + 1,
                        delay
                    );
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    continue;
                }

                // Non-rate-limit failure or no retries left — let caller handle it
                return Ok(output);
            }
            Err(e) => {
                if is_rate_limit_error(&e.to_string()) && remaining > 0 {
                    let delay = base_delay_secs * 2u64.pow(attempt);
                    eprintln!(
                        "Rate limit error (attempt {}/{}), retrying in {}s...",
                        attempt + 1,
                        max_retries + 1,
                        delay
                    );
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    last_error = Some(e);
                    continue;
                }

                return Err(e);
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("unknown error"))
        .context(format!(
            "exhausted {} retries due to rate limiting",
            max_retries
        )))
}
