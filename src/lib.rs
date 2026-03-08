//! # Candybase — Procedural Database Access for Rust
//!
//! Candy brings the simplicity of PHP's `mysqli_*` functions to Rust — flat,
//! procedural functions you can call one after another, with no builders,
//! no traits, and no boilerplate. Safety is preserved internally through
//! `Result<T, CandyError>`.
//!
//! ## Quick Start
//!
//! Add the crate to `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! candybase = { version = "0.1", features = ["sqlite"] }
//! ```
//!
//! Then write flat, immediate code:
//!
//! ```rust,no_run
//! use candybase::*;
//!
//! fn main() -> Result<(), CandyError> {
//!     let conn = candy_connect("localhost", "user", "pass", "dbname")?;
//!
//!     // SELECT
//!     let res   = candy_query(&conn, "SELECT * FROM users")?;
//!     let users = candy_fetch_all(res)?;
//!     println!("{:?}", users);
//!
//!     // INSERT / UPDATE / DELETE
//!     candy_insert(&conn, "INSERT INTO users (name) VALUES ('Alice')")?;
//!     candy_update(&conn, "UPDATE users SET name='Bob' WHERE id=1")?;
//!     candy_delete(&conn, "DELETE FROM users WHERE id=2")?;
//!
//!     // Atomic transaction
//!     candy_transaction(&conn, vec![
//!         "INSERT INTO orders (user_id) VALUES (1)",
//!         "UPDATE stock SET qty = qty - 1 WHERE id = 5",
//!     ])?;
//!
//!     candy_close(conn)?;
//!     Ok(())
//! }
//! ```
//!
//! ## Backend Selection
//!
//! Candy detects which backend to use from the URL scheme or the
//! `CANDY_DB_URL` environment variable:
//!
//! | Scheme | Backend |
//! |--------|---------|
//! | `mysql://…`      | MySQL / MariaDB |
//! | `postgres://…`   | PostgreSQL      |
//! | `sqlite://…`     | SQLite          |
//! | *(no env var)*   | SQLite `:memory:` |
//!
//! ## Feature Flags
//!
//! Enable only the backends you need to keep compile times short:
//!
//! ```toml
//! candybase = { version = "0.1", features = ["mysql", "postgres", "sqlite"] }
//! ```
//!
//! The default feature set is `["sqlite"]`.

#![warn(missing_docs)]

pub mod error;

#[cfg(feature = "mysql")]
pub mod mysql;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use error::CandyError;

use std::collections::HashMap;

// ── Internal backend discriminant ────────────────────────────────────────────

/// Internal connection state — one variant per compiled backend.
#[allow(dead_code)]
enum Inner {
    #[cfg(feature = "mysql")]
    Mysql(mysql::MysqlConn),
    #[cfg(feature = "postgres")]
    Postgres(postgres::PostgresConn),
    #[cfg(feature = "sqlite")]
    Sqlite(sqlite::SqliteConn),
}

/// Internal result state — one variant per compiled backend.
#[allow(dead_code)]
enum InnerResult {
    #[cfg(feature = "mysql")]
    Mysql(mysql::MysqlResult),
    #[cfg(feature = "postgres")]
    Postgres(postgres::PostgresResult),
    #[cfg(feature = "sqlite")]
    Sqlite(sqlite::SqliteResult),
}

// ── Public opaque handles ─────────────────────────────────────────────────────

/// An opaque connection handle returned by [`candy_connect`].
///
/// Pass it (by reference where possible) to every subsequent Candy function.
/// Close it explicitly with [`candy_close`] when done.
pub struct CandyConn {
    inner: std::cell::UnsafeCell<Inner>,
}

// SAFETY: `CandyConn` is not `Send` by default because the raw database
// connection objects are not always `Send`.  Users who need to share a
// connection across threads should wrap it in `Arc<Mutex<…>>` themselves,
// which is the same pattern required by the underlying driver crates.
unsafe impl Send for CandyConn {}

/// An opaque result handle returned by [`candy_query`].
///
/// Consume it with [`candy_fetch_all`] or [`candy_fetch_one`].
pub struct CandyResult {
    inner: InnerResult,
}

// ── URL helpers ───────────────────────────────────────────────────────────────

/// Detect the database backend from the URL scheme.
fn scheme_of(url: &str) -> &str {
    if url.starts_with("mysql://") {
        return "mysql";
    }
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        return "postgres";
    }
    if url.starts_with("sqlite://") {
        return "sqlite";
    }
    "unknown"
}

// ── candy_connect ─────────────────────────────────────────────────────────────

/// Establish a database connection.
///
/// Candy selects the backend automatically based on the `CANDY_DB_URL`
/// environment variable (if set) or the host string (if it begins with a
/// scheme like `sqlite://`).  For plain hostname strings it defaults to
/// SQLite in `:memory:` mode when compiled without MySQL or PostgreSQL.
///
/// # Arguments
///
/// * `host` — Hostname, IP, or full URL (e.g. `"localhost"` or `"sqlite://:memory:"`).
/// * `user` — Database username.  Ignored for SQLite.
/// * `pass` — Database password.  Ignored for SQLite.
/// * `db`   — Database name or file path for SQLite.
///
/// # Errors
///
/// Returns [`CandyError::Connection`] if the connection cannot be established.
///
/// # Examples
///
/// ```rust,no_run
/// use candybase::{candy_connect, CandyError};
///
/// let conn = candy_connect("localhost", "root", "secret", "shop")?;
/// # Ok::<(), CandyError>(())
/// ```
pub fn candy_connect(
    host: &str,
    user: &str,
    pass: &str,
    db: &str,
) -> Result<CandyConn, CandyError> {
    // Check environment variable first
    if let Ok(url) = std::env::var("CANDY_DB_URL") {
        return candy_connect_url(&url);
    }

    let scheme = scheme_of(host);

    match scheme {
        "mysql" => {
            #[cfg(feature = "mysql")]
            {
                let c = mysql::mysql_connect_url(host)?;
                return Ok(CandyConn {
                    inner: std::cell::UnsafeCell::new(Inner::Mysql(c)),
                });
            }
            #[cfg(not(feature = "mysql"))]
            return Err(CandyError::Connection(
                "MySQL URL detected but the `mysql` feature is not enabled".into(),
            ));
        }
        "postgres" => {
            #[cfg(feature = "postgres")]
            {
                let c = postgres::postgres_connect_url(host)?;
                return Ok(CandyConn {
                    inner: std::cell::UnsafeCell::new(Inner::Postgres(c)),
                });
            }
            #[cfg(not(feature = "postgres"))]
            return Err(CandyError::Connection(
                "PostgreSQL URL detected but the `postgres` feature is not enabled".into(),
            ));
        }
        "sqlite" => {
            #[cfg(feature = "sqlite")]
            {
                let c = sqlite::sqlite_connect_url(host)?;
                return Ok(CandyConn {
                    inner: std::cell::UnsafeCell::new(Inner::Sqlite(c)),
                });
            }
            #[cfg(not(feature = "sqlite"))]
            return Err(CandyError::Connection(
                "SQLite URL detected but the `sqlite` feature is not enabled".into(),
            ));
        }
        _ => {}
    }

    // No scheme detected — use individual params
    // Priority: mysql > postgres > sqlite
    #[cfg(feature = "mysql")]
    {
        let c = mysql::mysql_connect(host, user, pass, db)?;
        return Ok(CandyConn {
            inner: std::cell::UnsafeCell::new(Inner::Mysql(c)),
        });
    }

    #[cfg(all(feature = "postgres", not(feature = "mysql")))]
    {
        let c = postgres::postgres_connect(host, user, pass, db)?;
        return Ok(CandyConn {
            inner: std::cell::UnsafeCell::new(Inner::Postgres(c)),
        });
    }

    #[cfg(all(feature = "sqlite", not(feature = "mysql"), not(feature = "postgres")))]
    {
        // Use `db` as the file path for SQLite; fall back to :memory:
        let path = if db.is_empty() { ":memory:" } else { db };
        let c = sqlite::sqlite_connect(path)?;
        return Ok(CandyConn {
            inner: std::cell::UnsafeCell::new(Inner::Sqlite(c)),
        });
    }

    #[allow(unreachable_code)]
    Err(CandyError::Connection(
        "No database backend is enabled. Enable at least one of: mysql, postgres, sqlite".into(),
    ))
}

/// Establish a database connection from a full URL string.
///
/// The URL scheme determines the backend:
/// * `mysql://user:pass@host/db`
/// * `postgres://user:pass@host/db`
/// * `sqlite:///path/to/file.db` or `sqlite://:memory:`
///
/// You can also set `CANDY_DB_URL` in the environment and call
/// [`candy_connect`] with empty strings — it will read the variable
/// automatically.
///
/// # Errors
///
/// Returns [`CandyError::UrlParse`] for unrecognised schemes, or
/// [`CandyError::Connection`] if the driver rejects the URL.
///
/// # Examples
///
/// ```rust,no_run
/// use candybase::{candy_connect_url, CandyError};
///
/// let conn = candy_connect_url("sqlite://:memory:")?;
/// # Ok::<(), CandyError>(())
/// ```
pub fn candy_connect_url(url: &str) -> Result<CandyConn, CandyError> {
    match scheme_of(url) {
        "mysql" => {
            #[cfg(feature = "mysql")]
            {
                let c = mysql::mysql_connect_url(url)?;
                return Ok(CandyConn {
                    inner: std::cell::UnsafeCell::new(Inner::Mysql(c)),
                });
            }
            #[cfg(not(feature = "mysql"))]
            return Err(CandyError::UrlParse(
                "MySQL URL but `mysql` feature not enabled".into(),
            ));
        }
        "postgres" => {
            #[cfg(feature = "postgres")]
            {
                let c = postgres::postgres_connect_url(url)?;
                return Ok(CandyConn {
                    inner: std::cell::UnsafeCell::new(Inner::Postgres(c)),
                });
            }
            #[cfg(not(feature = "postgres"))]
            return Err(CandyError::UrlParse(
                "PostgreSQL URL but `postgres` feature not enabled".into(),
            ));
        }
        "sqlite" => {
            #[cfg(feature = "sqlite")]
            {
                let c = sqlite::sqlite_connect_url(url)?;
                return Ok(CandyConn {
                    inner: std::cell::UnsafeCell::new(Inner::Sqlite(c)),
                });
            }
            #[cfg(not(feature = "sqlite"))]
            return Err(CandyError::UrlParse(
                "SQLite URL but `sqlite` feature not enabled".into(),
            ));
        }
        _ => Err(CandyError::UrlParse(format!(
            "Unrecognised URL scheme in: `{}`",
            url
        ))),
    }
}

// ── candy_query ───────────────────────────────────────────────────────────────

/// Execute a SQL query and return a buffered result handle.
///
/// Use this for `SELECT` statements. For `INSERT`/`UPDATE`/`DELETE` prefer
/// [`candy_insert`], [`candy_update`], or [`candy_delete`] which return the
/// affected-row count.
///
/// # Arguments
///
/// * `conn` — A connection returned by [`candy_connect`].
/// * `sql`  — Any valid SQL `SELECT` statement.
///
/// # Errors
///
/// Returns [`CandyError::Query`] if the statement is rejected by the database.
///
/// # Examples
///
/// ```rust,no_run
/// use candybase::*;
/// # let conn = candy_connect("", "", "", ":memory:").unwrap();
/// let res   = candy_query(&conn, "SELECT 1 AS n")?;
/// let rows  = candy_fetch_all(res)?;
/// # Ok::<(), CandyError>(())
/// ```
pub fn candy_query(conn: &CandyConn, sql: &str) -> Result<CandyResult, CandyError> {
    // SAFETY: We require `&CandyConn` which prevents concurrent mutable borrows
    // at the Candy API level.  The UnsafeCell is needed because some drivers
    // require `&mut self` on their connection objects.
    let inner = unsafe { &mut *conn.inner.get() };
    match inner {
        #[cfg(feature = "mysql")]
        Inner::Mysql(ref mut c) => {
            let r = mysql::mysql_query(c, sql)?;
            Ok(CandyResult {
                inner: InnerResult::Mysql(r),
            })
        }
        #[cfg(feature = "postgres")]
        Inner::Postgres(ref mut c) => {
            let r = postgres::postgres_query(c, sql)?;
            Ok(CandyResult {
                inner: InnerResult::Postgres(r),
            })
        }
        #[cfg(feature = "sqlite")]
        Inner::Sqlite(ref c) => {
            let r = sqlite::sqlite_query(c, sql)?;
            Ok(CandyResult {
                inner: InnerResult::Sqlite(r),
            })
        }
    }
}

// ── candy_fetch_all ───────────────────────────────────────────────────────────

/// Fetch every row from a [`CandyResult`] as a `Vec<HashMap<String, String>>`.
///
/// All column values are converted to their string representation. `NULL`
/// values become the string `"NULL"`.
///
/// The [`CandyResult`] is consumed; call [`candy_query`] again if you need
/// to iterate a second time.
///
/// # Errors
///
/// Returns [`CandyError::Fetch`] if a column value cannot be decoded.
///
/// # Examples
///
/// ```rust,no_run
/// use candybase::*;
/// # let conn = candy_connect("", "", "", ":memory:").unwrap();
/// # candy_query(&conn, "CREATE TABLE t (id INTEGER)").unwrap();
/// let res  = candy_query(&conn, "SELECT * FROM t")?;
/// let rows = candy_fetch_all(res)?;
/// for row in &rows {
///     println!("{:?}", row);
/// }
/// # Ok::<(), CandyError>(())
/// ```
pub fn candy_fetch_all(res: CandyResult) -> Result<Vec<HashMap<String, String>>, CandyError> {
    match res.inner {
        #[cfg(feature = "mysql")]
        InnerResult::Mysql(r) => mysql::mysql_fetch_all(r),
        #[cfg(feature = "postgres")]
        InnerResult::Postgres(r) => postgres::postgres_fetch_all(r),
        #[cfg(feature = "sqlite")]
        InnerResult::Sqlite(r) => sqlite::sqlite_fetch_all(r),
    }
}

// ── candy_fetch_one ───────────────────────────────────────────────────────────

/// Fetch the first (and typically only) row from a [`CandyResult`].
///
/// Returns [`CandyError::Fetch`] if the result set is empty.
/// All values are coerced to `String`; `NULL` becomes `"NULL"`.
///
/// # Errors
///
/// * [`CandyError::Fetch`] — result set is empty or a column cannot be decoded.
///
/// # Examples
///
/// ```rust,no_run
/// use candybase::*;
/// # let conn = candy_connect("", "", "", ":memory:").unwrap();
/// let res = candy_query(&conn, "SELECT COUNT(*) AS cnt FROM users")?;
/// let row = candy_fetch_one(res)?;
/// println!("Count = {}", row["cnt"]);
/// # Ok::<(), CandyError>(())
/// ```
pub fn candy_fetch_one(res: CandyResult) -> Result<HashMap<String, String>, CandyError> {
    match res.inner {
        #[cfg(feature = "mysql")]
        InnerResult::Mysql(r) => mysql::mysql_fetch_one(r),
        #[cfg(feature = "postgres")]
        InnerResult::Postgres(r) => postgres::postgres_fetch_one(r),
        #[cfg(feature = "sqlite")]
        InnerResult::Sqlite(r) => sqlite::sqlite_fetch_one(r),
    }
}

// ── Internal exec helper ──────────────────────────────────────────────────────

fn exec_sql(conn: &CandyConn, sql: &str) -> Result<u64, CandyError> {
    let inner = unsafe { &mut *conn.inner.get() };
    match inner {
        #[cfg(feature = "mysql")]
        Inner::Mysql(ref mut c) => mysql::mysql_exec(c, sql),
        #[cfg(feature = "postgres")]
        Inner::Postgres(ref mut c) => postgres::postgres_exec(c, sql),
        #[cfg(feature = "sqlite")]
        Inner::Sqlite(ref c) => sqlite::sqlite_exec(c, sql),
    }
}

// ── candy_insert ──────────────────────────────────────────────────────────────

/// Execute an `INSERT` statement and return the number of inserted rows.
///
/// Semantically identical to [`candy_update`] and [`candy_delete`]; provided
/// as separate functions so call sites read like the intent of the SQL.
///
/// # Arguments
///
/// * `conn` — Active connection handle.
/// * `sql`  — A valid `INSERT` statement.
///
/// # Errors
///
/// Returns [`CandyError::Query`] if the statement fails.
///
/// # Examples
///
/// ```rust,no_run
/// use candybase::*;
/// # let conn = candy_connect("", "", "", ":memory:").unwrap();
/// let affected = candy_insert(&conn, "INSERT INTO users (name) VALUES ('Alice')")?;
/// assert_eq!(affected, 1);
/// # Ok::<(), CandyError>(())
/// ```
pub fn candy_insert(conn: &CandyConn, sql: &str) -> Result<u64, CandyError> {
    exec_sql(conn, sql)
}

// ── candy_update ──────────────────────────────────────────────────────────────

/// Execute an `UPDATE` statement and return the number of affected rows.
///
/// # Arguments
///
/// * `conn` — Active connection handle.
/// * `sql`  — A valid `UPDATE` statement.
///
/// # Errors
///
/// Returns [`CandyError::Query`] if the statement fails.
///
/// # Examples
///
/// ```rust,no_run
/// use candybase::*;
/// # let conn = candy_connect("", "", "", ":memory:").unwrap();
/// let affected = candy_update(&conn, "UPDATE users SET name='Bob' WHERE id=1")?;
/// println!("{} rows updated", affected);
/// # Ok::<(), CandyError>(())
/// ```
pub fn candy_update(conn: &CandyConn, sql: &str) -> Result<u64, CandyError> {
    exec_sql(conn, sql)
}

// ── candy_delete ──────────────────────────────────────────────────────────────

/// Execute a `DELETE` statement and return the number of deleted rows.
///
/// # Arguments
///
/// * `conn` — Active connection handle.
/// * `sql`  — A valid `DELETE` statement.
///
/// # Errors
///
/// Returns [`CandyError::Query`] if the statement fails.
///
/// # Examples
///
/// ```rust,no_run
/// use candybase::*;
/// # let conn = candy_connect("", "", "", ":memory:").unwrap();
/// candy_delete(&conn, "DELETE FROM users WHERE id=2")?;
/// # Ok::<(), CandyError>(())
/// ```
pub fn candy_delete(conn: &CandyConn, sql: &str) -> Result<u64, CandyError> {
    exec_sql(conn, sql)
}

// ── candy_transaction ─────────────────────────────────────────────────────────

/// Execute multiple SQL statements atomically.
///
/// All statements are wrapped in a single database transaction.  If any
/// statement fails the entire transaction is rolled back and a
/// [`CandyError::Transaction`] is returned.  On success every statement is
/// committed together.
///
/// # Arguments
///
/// * `conn`    — Active connection handle.
/// * `queries` — Ordered list of SQL statements to execute.
///
/// # Errors
///
/// Returns [`CandyError::Transaction`] on failure (after automatic rollback).
///
/// # Examples
///
/// ```rust,no_run
/// use candybase::*;
/// # let conn = candy_connect("", "", "", ":memory:").unwrap();
/// candy_transaction(&conn, vec![
///     "INSERT INTO orders (user_id, total) VALUES (1, 99)",
///     "UPDATE accounts SET balance = balance - 99 WHERE user_id = 1",
/// ])?;
/// # Ok::<(), CandyError>(())
/// ```
pub fn candy_transaction(conn: &CandyConn, queries: Vec<&str>) -> Result<(), CandyError> {
    let inner = unsafe { &mut *conn.inner.get() };
    let qs: Vec<&str> = queries;
    match inner {
        #[cfg(feature = "mysql")]
        Inner::Mysql(ref mut c) => mysql::mysql_transaction(c, &qs),
        #[cfg(feature = "postgres")]
        Inner::Postgres(ref mut c) => postgres::postgres_transaction(c, &qs),
        #[cfg(feature = "sqlite")]
        Inner::Sqlite(ref c) => sqlite::sqlite_transaction(c, &qs),
    }
}

// ── candy_close ───────────────────────────────────────────────────────────────

/// Close the database connection and release all associated resources.
///
/// The [`CandyConn`] is consumed and cannot be used after this call.
/// For connection-pool–backed drivers (MySQL) this returns the connection
/// to the pool rather than performing a TCP close.
///
/// Dropping a [`CandyConn`] without calling `candy_close` is safe — the
/// underlying driver will clean up on `Drop`.  `candy_close` exists for
/// explicit, readable resource management mirroring PHP's `mysqli_close`.
///
/// # Errors
///
/// Currently always returns `Ok(())`.  The signature uses `Result` for
/// forward-compatibility with drivers that perform network I/O on close.
///
/// # Examples
///
/// ```rust,no_run
/// use candybase::*;
/// # let conn = candy_connect("", "", "", ":memory:").unwrap();
/// candy_close(conn)?;
/// # Ok::<(), CandyError>(())
/// ```
pub fn candy_close(_conn: CandyConn) -> Result<(), CandyError> {
    // CandyConn is moved here and dropped at end of scope.
    // Each inner driver implements Drop which closes the connection / returns
    // to pool automatically.
    Ok(())
}
