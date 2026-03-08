//! # MySQL Backend
//!
//! Internal implementation using the [`mysql`](https://crates.io/crates/mysql) crate.
//! This module is only compiled when the `mysql` feature flag is enabled.

use crate::error::CandyError;
use mysql::{prelude::*, Pool, PooledConn, Row, Value};
use std::collections::HashMap;

/// An open MySQL connection handle.
///
/// Obtain one via [`crate::candy_connect`] and release it with
/// [`crate::candy_close`].
pub struct MysqlConn {
    pub(crate) conn: PooledConn,
}

/// A pending MySQL result set.
///
/// Produced by [`crate::candy_query`]; consumed by
/// [`crate::candy_fetch_all`] or [`crate::candy_fetch_one`].
pub struct MysqlResult {
    pub(crate) rows: Vec<Row>,
}

// ── Connection ───────────────────────────────────────────────────────────────

/// Open a pooled MySQL connection from individual parameters.
pub fn mysql_connect(
    host: &str,
    user: &str,
    pass: &str,
    db: &str,
) -> Result<MysqlConn, CandyError> {
    let url = format!("mysql://{}:{}@{}/{}", user, pass, host, db);
    mysql_connect_url(&url)
}

/// Open a pooled MySQL connection from a full URL.
///
/// Example URL: `mysql://user:pass@localhost:3306/mydb`
pub fn mysql_connect_url(url: &str) -> Result<MysqlConn, CandyError> {
    let pool = Pool::new(url).map_err(|e| CandyError::Connection(e.to_string()))?;
    let conn = pool
        .get_conn()
        .map_err(|e| CandyError::Connection(e.to_string()))?;
    Ok(MysqlConn { conn })
}

// ── Query ────────────────────────────────────────────────────────────────────

/// Execute a SELECT-style query and buffer all rows.
pub fn mysql_query(conn: &mut MysqlConn, sql: &str) -> Result<MysqlResult, CandyError> {
    let rows: Vec<Row> = conn
        .conn
        .query(sql)
        .map_err(|e| CandyError::Query(e.to_string()))?;
    Ok(MysqlResult { rows })
}

// ── Fetch ────────────────────────────────────────────────────────────────────

/// Convert a [`MysqlResult`] into a vector of string maps.
pub fn mysql_fetch_all(res: MysqlResult) -> Result<Vec<HashMap<String, String>>, CandyError> {
    res.rows.into_iter().map(row_to_map).collect()
}

/// Return only the first row of a [`MysqlResult`].
pub fn mysql_fetch_one(res: MysqlResult) -> Result<HashMap<String, String>, CandyError> {
    let mut iter = res.rows.into_iter();
    match iter.next() {
        Some(row) => row_to_map(row),
        None => Err(CandyError::Fetch("Result set is empty".into())),
    }
}

// ── Mutating statements ───────────────────────────────────────────────────────

/// Execute INSERT / UPDATE / DELETE and return the number of affected rows.
pub fn mysql_exec(conn: &mut MysqlConn, sql: &str) -> Result<u64, CandyError> {
    conn.conn
        .query_drop(sql)
        .map_err(|e| CandyError::Query(e.to_string()))?;
    Ok(conn.conn.affected_rows())
}

// ── Transaction ───────────────────────────────────────────────────────────────

/// Execute a slice of SQL statements inside a single transaction.
///
/// Rolls back automatically on any failure.
pub fn mysql_transaction(conn: &mut MysqlConn, queries: &[&str]) -> Result<(), CandyError> {
    conn.conn
        .query_drop("START TRANSACTION")
        .map_err(|e| CandyError::Transaction(e.to_string()))?;

    for sql in queries {
        if let Err(e) = conn.conn.query_drop(*sql) {
            let _ = conn.conn.query_drop("ROLLBACK");
            return Err(CandyError::Transaction(e.to_string()));
        }
    }

    conn.conn
        .query_drop("COMMIT")
        .map_err(|e| CandyError::Transaction(e.to_string()))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_map(row: Row) -> Result<HashMap<String, String>, CandyError> {
    let columns = row
        .columns_ref()
        .iter()
        .map(|c| c.name_str().to_string())
        .collect::<Vec<_>>();

    let mut map = HashMap::new();
    for (i, col_name) in columns.iter().enumerate() {
        let val: Value = row
            .get(i)
            .ok_or_else(|| CandyError::Fetch(format!("Missing column index {}", i)))?;
        map.insert(col_name.clone(), value_to_string(val));
    }
    Ok(map)
}

fn value_to_string(val: Value) -> String {
    match val {
        Value::NULL => "NULL".to_string(),
        Value::Bytes(b) => String::from_utf8_lossy(&b).to_string(),
        Value::Int(i) => i.to_string(),
        Value::UInt(u) => u.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(d) => d.to_string(),
        Value::Date(y, mo, d, h, mi, s, _) => {
            format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, d, h, mi, s)
        }
        Value::Time(neg, d, h, m, s, _) => {
            let sign = if neg { "-" } else { "" };
            format!("{}{}:{:02}:{:02}", sign, d * 24 + h as u32, m, s)
        }
    }
}
