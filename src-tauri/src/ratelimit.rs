//! Token-bucket rate limiter used for global and per-task speed limits.
//!
//! The bucket capacity is one second of traffic, which allows a short burst but
//! converges on the configured average. `consume` is async because exceeding
//! the bucket sleeps the caller (spec §4.2, §14.4).

use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Bucket {
    tokens: f64,
    last_refill: Instant,
    rate: f64, // tokens per second
}

pub struct SpeedLimiter {
    limit_bps: Mutex<u64>,
    bucket: Mutex<Bucket>,
}

impl SpeedLimiter {
    pub fn new(limit_bps: u64) -> Self {
        SpeedLimiter {
            limit_bps: Mutex::new(limit_bps),
            bucket: Mutex::new(Bucket {
                tokens: 0.0,
                last_refill: Instant::now(),
                rate: limit_bps as f64,
            }),
        }
    }

    /// Update the limit at runtime. A value of 0 disables limiting.
    pub fn set_limit(&self, limit_bps: u64) {
        let mut limit = self.limit_bps.lock().expect("speed limit poisoned");
        let mut bucket = self.bucket.lock().expect("speed bucket poisoned");
        bucket.rate = limit_bps as f64;
        bucket.tokens = bucket.tokens.min(bucket.rate);
        *limit = limit_bps;
    }

    pub fn limit(&self) -> u64 {
        *self.limit_bps.lock().expect("speed limit poisoned")
    }

    /// Account for `bytes` and wait if the bucket is exhausted.
    pub async fn consume(&self, bytes: u64) {
        if bytes == 0 {
            return;
        }
        let rate = {
            let limit = self.limit_bps.lock().expect("speed limit poisoned");
            if *limit == 0 {
                return;
            }
            *limit as f64
        };

        let wait = {
            let mut bucket = self.bucket.lock().expect("speed bucket poisoned");
            let now = Instant::now();
            let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
            bucket.last_refill = now;
            bucket.tokens = (bucket.tokens + elapsed * rate).min(rate);

            let needed = bytes as f64;
            if bucket.tokens >= needed {
                bucket.tokens -= needed;
                None
            } else {
                let deficit = needed - bucket.tokens;
                bucket.tokens = 0.0;
                Some(Duration::from_secs_f64(deficit / rate))
            }
        };

        if let Some(d) = wait {
            tokio::time::sleep(d).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlimited_limiter_never_blocks() {
        use futures_util::FutureExt;
        let lim = SpeedLimiter::new(0);
        assert!(lim.consume(1_000_000).now_or_never().is_some(), "unlimited consumption must be ready without yielding to a timer");
    }

    #[test]
    fn limited_limiter_delays() {
        let lim = SpeedLimiter::new(1_000); // 1 KB/s
        let start = Instant::now();
        futures_executor_block_on(lim.consume(500));
        futures_executor_block_on(lim.consume(500));
        futures_executor_block_on(lim.consume(500)); // should wait ~0.5s
        assert!(start.elapsed() >= Duration::from_millis(300));
    }

    #[test]
    fn changing_limit_takes_effect() {
        let lim = SpeedLimiter::new(1_000_000);
        lim.set_limit(0);
        assert_eq!(lim.limit(), 0);
    }
}

/// Tiny helper so the sync tests above can drive an async fn without pulling in
/// a second runtime crate.
#[cfg(test)]
fn futures_executor_block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime")
        .block_on(fut)
}
