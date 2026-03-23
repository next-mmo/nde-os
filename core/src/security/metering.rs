use anyhow::{anyhow, Result};
use std::time::Instant;

/// Compute metering: limits per-turn token usage, wall-clock time, and tool invocations.
pub struct ComputeMeter {
    pub max_tokens: u64,
    pub max_time_secs: u64,
    pub max_tool_calls: u64,

    tokens_used: u64,
    tool_calls_used: u64,
    start_time: Option<Instant>,
    enabled: bool,
}

impl ComputeMeter {
    pub fn new(max_tokens: u64, max_time_secs: u64, max_tool_calls: u64) -> Self {
        Self {
            max_tokens,
            max_time_secs,
            max_tool_calls,
            tokens_used: 0,
            tool_calls_used: 0,
            start_time: None,
            enabled: true,
        }
    }

    pub fn disabled() -> Self {
        Self {
            max_tokens: u64::MAX,
            max_time_secs: u64::MAX,
            max_tool_calls: u64::MAX,
            tokens_used: 0,
            tool_calls_used: 0,
            start_time: None,
            enabled: false,
        }
    }

    /// Start the meter (call at the beginning of each turn).
    pub fn start(&mut self) {
        self.tokens_used = 0;
        self.tool_calls_used = 0;
        self.start_time = Some(Instant::now());
    }

    /// Record token usage.
    pub fn add_tokens(&mut self, count: u64) {
        self.tokens_used += count;
    }

    /// Record a tool call.
    pub fn add_tool_call(&mut self) {
        self.tool_calls_used += 1;
    }

    /// Check if we're within budget. Returns Err if any limit exceeded.
    pub fn check_budget(&self) -> Result<()> {
        if !self.enabled { return Ok(()); }

        if self.tokens_used > self.max_tokens {
            return Err(anyhow!(
                "Token budget exceeded: {} / {}",
                self.tokens_used, self.max_tokens
            ));
        }

        if self.tool_calls_used > self.max_tool_calls {
            return Err(anyhow!(
                "Tool call limit exceeded: {} / {}",
                self.tool_calls_used, self.max_tool_calls
            ));
        }

        if let Some(start) = self.start_time {
            if start.elapsed().as_secs() > self.max_time_secs {
                return Err(anyhow!(
                    "Time budget exceeded: {}s / {}s",
                    start.elapsed().as_secs(), self.max_time_secs
                ));
            }
        }

        Ok(())
    }

    /// Current usage stats.
    pub fn stats(&self) -> MeterStats {
        MeterStats {
            tokens_used: self.tokens_used,
            tokens_max: self.max_tokens,
            tool_calls_used: self.tool_calls_used,
            tool_calls_max: self.max_tool_calls,
            elapsed_secs: self.start_time.map(|s| s.elapsed().as_secs()).unwrap_or(0),
            time_max_secs: self.max_time_secs,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct MeterStats {
    pub tokens_used: u64,
    pub tokens_max: u64,
    pub tool_calls_used: u64,
    pub tool_calls_max: u64,
    pub elapsed_secs: u64,
    pub time_max_secs: u64,
}
