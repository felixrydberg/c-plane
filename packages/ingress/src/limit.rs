use std::{
    collections::HashMap,
    fmt,
    net::IpAddr,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use pingora_limits::rate::Rate;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TrafficClass {
    Auth,
    Ui,
    ApiRead,
    ApiWrite,
    Storage,
    Registry,
}

impl fmt::Display for TrafficClass {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Auth => "auth",
            Self::Ui => "ui",
            Self::ApiRead => "api_read",
            Self::ApiWrite => "api_write",
            Self::Storage => "storage",
            Self::Registry => "registry",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LimitMode {
    Off,
    Observe,
    Enforce,
}

impl FromStr for LimitMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "observe" => Ok(Self::Observe),
            "enforce" => Ok(Self::Enforce),
            _ => Err(format!(
                "INGRESS_RATE_MODE must be off, observe, or enforce; got {value:?}"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LimitPolicy {
    pub per_second: usize,
    pub per_minute: usize,
    pub in_flight: usize,
}

impl LimitPolicy {
    pub const fn new(per_second: usize, per_minute: usize, in_flight: usize) -> Self {
        Self {
            per_second,
            per_minute,
            in_flight,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LimitSettings {
    pub mode: LimitMode,
    pub auth: LimitPolicy,
    pub ui: LimitPolicy,
    pub api_read: LimitPolicy,
    pub api_write: LimitPolicy,
    pub storage: LimitPolicy,
    pub registry: LimitPolicy,
}

impl LimitSettings {
    pub fn policy(&self, class: TrafficClass) -> LimitPolicy {
        match class {
            TrafficClass::Auth => self.auth,
            TrafficClass::Ui => self.ui,
            TrafficClass::ApiRead => self.api_read,
            TrafficClass::ApiWrite => self.api_write,
            TrafficClass::Storage => self.storage,
            TrafficClass::Registry => self.registry,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Key {
    class: TrafficClass,
    client: IpAddr,
}

#[derive(Clone)]
pub struct LocalLimiter {
    mode: LimitMode,
    per_second: Arc<Rate>,
    per_minute: Arc<Rate>,
    in_flight: Arc<Mutex<HashMap<TrafficClass, usize>>>,
}

pub struct Permit {
    class: Option<TrafficClass>,
    counts: Arc<Mutex<HashMap<TrafficClass, usize>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectionKind {
    RateLimit,
    Saturated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rejection {
    pub kind: RejectionKind,
    pub retry_after_seconds: u64,
}

impl LocalLimiter {
    pub fn new(mode: LimitMode) -> Self {
        Self {
            mode,
            per_second: Arc::new(Rate::new(Duration::from_secs(1))),
            per_minute: Arc::new(Rate::new(Duration::from_secs(60))),
            in_flight: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn check(
        &self,
        class: TrafficClass,
        client: IpAddr,
        policy: LimitPolicy,
    ) -> Result<Permit, Rejection> {
        if self.mode == LimitMode::Off {
            return Ok(Permit::disabled(self.in_flight.clone()));
        }

        let key = Key { class, client };
        let second = self.per_second.observe(&key, 1) as usize;
        let minute = self.per_minute.observe(&key, 1) as usize;
        let current = {
            let mut counts = self
                .in_flight
                .lock()
                .expect("in-flight rate-limit lock poisoned");
            let current = counts.entry(class).or_default();
            *current += 1;
            *current
        };

        let exceeded = if second > policy.per_second {
            Some(Rejection {
                kind: RejectionKind::RateLimit,
                retry_after_seconds: 1,
            })
        } else if minute > policy.per_minute {
            Some(Rejection {
                kind: RejectionKind::RateLimit,
                retry_after_seconds: 60,
            })
        } else if current > policy.in_flight {
            Some(Rejection {
                kind: RejectionKind::Saturated,
                retry_after_seconds: 1,
            })
        } else {
            None
        };

        if let Some(rejection) = exceeded {
            if self.mode == LimitMode::Enforce {
                drop(Permit {
                    class: Some(class),
                    counts: self.in_flight.clone(),
                });
                return Err(rejection);
            }
            tracing::warn!(class = %class, client = %client, second, minute, current, "local ingress limit exceeded");
        }

        Ok(Permit {
            class: Some(class),
            counts: self.in_flight.clone(),
        })
    }
}

impl Permit {
    fn disabled(counts: Arc<Mutex<HashMap<TrafficClass, usize>>>) -> Self {
        Self {
            class: None,
            counts,
        }
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let Some(class) = self.class.take() else {
            return;
        };
        let mut counts = self
            .counts
            .lock()
            .expect("in-flight rate-limit lock poisoned");
        if let Some(current) = counts.get_mut(&class) {
            *current -= 1;
            if *current == 0 {
                counts.remove(&class);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_replica_wide_in_flight_limit_per_class() {
        let limiter = LocalLimiter::new(LimitMode::Enforce);
        let client = "192.0.2.1".parse().unwrap();
        let policy = LimitPolicy::new(100, 100, 1);
        let first = limiter
            .check(TrafficClass::ApiRead, client, policy)
            .unwrap();

        assert_eq!(
            limiter
                .check(TrafficClass::ApiRead, "192.0.2.2".parse().unwrap(), policy,)
                .err(),
            Some(Rejection {
                kind: RejectionKind::Saturated,
                retry_after_seconds: 1
            })
        );
        assert!(
            limiter
                .check(TrafficClass::ApiWrite, client, policy)
                .is_ok()
        );

        drop(first);
        assert!(limiter.check(TrafficClass::ApiRead, client, policy).is_ok());
    }

    #[test]
    fn enforces_request_rate_per_client() {
        let limiter = LocalLimiter::new(LimitMode::Enforce);
        let client = "192.0.2.1".parse().unwrap();
        let policy = LimitPolicy::new(1, 100, 100);

        drop(
            limiter
                .check(TrafficClass::ApiRead, client, policy)
                .unwrap(),
        );
        assert_eq!(
            limiter.check(TrafficClass::ApiRead, client, policy).err(),
            Some(Rejection {
                kind: RejectionKind::RateLimit,
                retry_after_seconds: 1,
            })
        );
        assert!(
            limiter
                .check(TrafficClass::ApiRead, "192.0.2.2".parse().unwrap(), policy,)
                .is_ok()
        );
    }

    #[test]
    fn observe_mode_keeps_balanced_in_flight_counts() {
        let limiter = LocalLimiter::new(LimitMode::Observe);
        let client = "192.0.2.1".parse().unwrap();
        let policy = LimitPolicy::new(100, 100, 0);

        drop(
            limiter
                .check(TrafficClass::ApiRead, client, policy)
                .unwrap(),
        );
        drop(
            limiter
                .check(TrafficClass::ApiRead, client, policy)
                .unwrap(),
        );
    }
}
