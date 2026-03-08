//! # SQLite Backend
//!
//! Internal implementation using [`rusqlite`](https://crates.io/crates/rusqlite)
//! with the `bundled` feature so no system SQLite is required.
//! Only compiled when the `sqlite` feature flag is enabled.

use crate::error::CandyError;
use rusqlite::{types::ValueRef, Connection};
use std::collections::HashMap;

/// An open SQLite connection handle.
///
/// Obtain via [`crate::candy_connect`] and release with [`crate::candy_close`].
pub struct SqliteConn {
    pub(crate) conn: Connection,
}

/// A pending SQLite result set.
///
/// Produced by [`crate::candy_query`]; consumed by
/// [`crate::candy_fetch_all`] or [`crate::candy_fetch_one`].
pub struct SqliteResult {
    pub(crate) rows: Vec<HashMap<String, String>>,
}

// ── Connection ────────────────────────────────────────────────────────────────

/// Open a SQLite connection.
///
/// For file-based databases `host` is unused; pass `db` as the file path
/// (e.g., `"./myapp.db"` or `":memory:"`).
///
/// When using [`crate::candy_connect`], `host`, `user`, and `pass` are
/// silently ignored for SQLite.
pub fn sqlite_connect(path: &str) -> Result<SqliteConn, CandyError> {
    let conn = Connection::open(path).map_err(|e| CandyError::Connection(e.to_string()))?;
    // Enable WAL mode for better concurrent read performance
    conn.execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(|e| CandyError::Connection(e.to_string()))?;
    Ok(SqliteConn { conn })
}

/// Open a SQLite connection from a URL.
///
/// Accepts `sqlite://./path/to/file.db` or `sqlite://:memory:`.
pub fn sqlite_connect_url(url: &str) -> Result<SqliteConn, CandyError> {
    let path = url.strip_prefix("sqlite://").unwrap_or(url);
    sqlite_connect(path)
}

// ── Query ─────────────────────────────────────────────────────────────────────

/// Execute a SELECT-style query and buffer all rows into memory.
pub fn sqlite_query(conn: &SqliteConn, sql: &str) -> Result<SqliteResult, CandyError> {
    let mut stmt = conn
        .conn
        .prepare(sql)
        .map_err(|e| CandyError::Query(e.to_string()))?;

    let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let rows: Vec<HashMap<String, String>> = stmt
        .query_map([], |row| {
            let mut map = HashMap::new();
            for (i, col) in col_names.iter().enumerate() {
                let val = match row.get_ref_unwrap(i) {
                    ValueRef::Null => "NULL".to_string(),
                    ValueRef::Integer(n) => n.to_string(),
                    ValueRef::Real(f) => f.to_string(),
                    ValueRef::Text(b) => String::from_utf8_lossy(b).to_string(),
                    ValueRef::Blob(b) => format!("{:?}", b),
                };
                map.insert(col.clone(), val);
            }
            Ok(map)
        })
        .map_err(|e| CandyError::Query(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| CandyError::Fetch(e.to_string()))?;

    Ok(SqliteResult { rows })
}

// ── Fetch ─────────────────────────────────────────────────────────────────────

/// Convert a [`SqliteResult`] into a vector of string maps.
pub fn sqlite_fetch_all(res: SqliteResult) -> Result<Vec<HashMap<String, String>>, CandyError> {
    Ok(res.rows)
}

/// Return only the first row of a [`SqliteResult`].
pub fn sqlite_fetch_one(res: SqliteResult) -> Result<HashMap<String, String>, CandyError> {
    res.rows
        .into_iter()
        .next()
        .ok_or_else(|| CandyError::Fetch("Result set is empty".into()))
}

// ── Mutating statements ───────────────────────────────────────────────────────

/// Execute INSERT / UPDATE / DELETE and return the number of affected rows.
pub fn sqlite_exec(conn: &SqliteConn, sql: &str) -> Result<u64, CandyError> {
    let n = conn
        .conn
        .execute(sql, [])
        .map_err(|e| CandyError::Query(e.to_string()))?;
    Ok(n as u64)
}

// ── Transaction ───────────────────────────────────────────────────────────────

/// Execute a slice of SQL statements inside a single transaction.
///
/// Rolls back automatically on any failure.
pub fn sqlite_transaction(conn: &SqliteConn, queries: &[&str]) -> Result<(), CandyError> {
    conn.conn
        .execute_batch("BEGIN")
        .map_err(|e| CandyError::Transaction(e.to_string()))?;

    for sql in queries {
        if let Err(e) = conn.conn.execute_batch(sql) {
            let _ = conn.conn.execute_batch("ROLLBACK");
            return Err(CandyError::Transaction(e.to_string()));
        }
    }

    conn.conn
        .execute_batch("COMMIT")
        .map_err(|e| CandyError::Transaction(e.to_string()))
}
