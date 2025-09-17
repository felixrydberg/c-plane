use std::fmt;

/// Custom logger utility for consistent logging across the application
pub struct Logger;

#[derive(Debug, Clone)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Trace => write!(f, "TRACE"),
        }
    }
}

impl Logger {
    /// Log an error message
    pub fn error(message: &str) {
        Self::log(LogLevel::Error, message, None);
    }

    /// Log an error with context
    pub fn error_with_context(message: &str, context: &str) {
        Self::log(LogLevel::Error, message, Some(context));
    }

    /// Log a warning message
    pub fn warn(message: &str) {
        Self::log(LogLevel::Warn, message, None);
    }

    /// Log an info message
    pub fn info(message: &str) {
        Self::log(LogLevel::Info, message, None);
    }

    /// Log a debug message
    pub fn debug(message: &str) {
        Self::log(LogLevel::Debug, message, None);
    }

    /// Log a trace message
    pub fn trace(message: &str) {
        Self::log(LogLevel::Trace, message, None);
    }

    /// Log HTTP request/response information
    pub fn http_request(method: &str, path: &str, status: u16, duration_ms: u64) {
        let message = format!("HTTP {} {} - {} - {}ms", method, path, status, duration_ms);
        Self::log(LogLevel::Info, &message, Some("HTTP"));
    }

    /// Log database operations
    pub fn database_operation(operation: &str, table: &str, duration_ms: u64) {
        let message = format!("DB {} on {} - {}ms", operation, table, duration_ms);
        Self::log(LogLevel::Debug, &message, Some("DB"));
    }

    /// Log external service calls
    pub fn external_service(service: &str, operation: &str, success: bool, duration_ms: u64) {
        let status = if success { "SUCCESS" } else { "FAILED" };
        let message = format!(
            "External {} {} - {} - {}ms",
            service, operation, status, duration_ms
        );
        Self::log(LogLevel::Info, &message, Some("EXT"));
    }

    /// Core logging function with custom formatting
    fn log(level: LogLevel, message: &str, context: Option<&str>) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");

        let formatted_message = match context {
            Some(ctx) => format!("[{}] {} [{}] {}", timestamp, level, ctx, message),
            None => format!("[{}] {} {}", timestamp, level, message),
        };

        // For now, just print to stdout/stderr
        // Later you can extend this to write to files, send to logging services, etc.
        match level {
            LogLevel::Error => eprintln!("{}", formatted_message),
            _ => println!("{}", formatted_message),
        }
    }
}

/// Convenience macros for easier logging
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::utils::logger::Logger::error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::utils::logger::Logger::warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::utils::logger::Logger::info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::utils::logger::Logger::debug(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        $crate::utils::logger::Logger::trace(&format!($($arg)*))
    };
}

/// Custom Actix-web middleware for logging HTTP requests
use actix_web::{
    Error,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{Ready, ready},
    time::Instant,
};

pub struct CustomLogger;

impl<S, B> Transform<S, ServiceRequest> for CustomLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = CustomLoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CustomLoggerMiddleware { service }))
    }
}

pub struct CustomLoggerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for CustomLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Instant::now();
        let method = req.method().to_string();
        let path = req.path().to_string();

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            let duration = start_time.elapsed();
            let status = res.status().as_u16();

            Logger::http_request(&method, &path, status, duration.as_millis() as u64);

            Ok(res)
        })
    }
}
