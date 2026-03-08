//! # Error Types
//!
//! All Candy functions return `Result<T, CandyError>`.
//! Match on the variant to distinguish connection errors from query errors.
//!
//! ```rust,ignore
//! use candybase::{CandyError, candy_connect};
//!
//! match candy_connect("localhost", "root", "secret", "mydb") {
//!     Ok(conn) => { /* use conn */ }
//!     Err(CandyError::Connection(msg)) => eprintln!("Cannot connect: {}", msg),
//!     Err(e) => eprintln!("Other error: {}", e),
//! }
//! ```

use thiserror::Error;

/// The unified error type for every Candy operation.
///
/// Every variant carries a descriptive `String` message so you can log or
/// display it without additional context.
#[derive(Debug, Error)]
pub enum CandyError {
    /// Returned when the initial database connection cannot be established.
    ///
    /// Common causes: wrong host/port, bad credentials, network firewall.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Returned when a SQL statement fails to execute.
    ///
    /// The message includes the driver-level error for easy debugging.
    #[error("Query error: {0}")]
    Query(String),

    /// Returned when row data cannot be decoded into a `HashMap<String, String>`.
    #[error("Fetch error: {0}")]
    Fetch(String),

    /// Returned when an atomic transaction cannot be completed.
    ///
    /// The whole transaction is rolled back automatically before this error
    /// is returned.
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Returned when the connection URL or DSN cannot be parsed.
    #[error("URL parse error: {0}")]
    UrlParse(String),

    /// Returned for any other unexpected internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

// ── Blanket conversions from driver-specific errors ─────────────────────────

#[cfg(feature = "mysql")]
impl From<::mysql::Error> for CandyError {
    fn from(e: ::mysql::Error) -> Self {
        CandyError::Query(e.to_string())
    }
}

#[cfg(feature = "postgres")]
impl From<::postgres::Error> for CandyError {
    fn from(e: ::postgres::Error) -> Self {
        CandyError::Query(e.to_string())
    }
}

#[cfg(feature = "sqlite")]
impl From<::rusqlite::Error> for CandyError {
    fn from(e: ::rusqlite::Error) -> Self {
        CandyError::Query(e.to_string())
    }
}
