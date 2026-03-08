//! # PostgreSQL Backend
//!
//! Internal implementation using the [`postgres`](https://crates.io/crates/postgres) crate.
//! Only compiled when the `postgres` feature flag is enabled.

use crate::error::CandyError;
use postgres::{Client, NoTls, Row};
use std::collections::HashMap;

/// An open PostgreSQL connection handle.
///
/// Obtain via [`crate::candy_connect`] and release with [`crate::candy_close`].
pub struct PostgresConn {
    pub(crate) client: Client,
}

/// A pending PostgreSQL result set.
///
/// Produced by [`crate::candy_query`]; consumed by
/// [`crate::candy_fetch_all`] or [`crate::candy_fetch_one`].
pub struct PostgresResult {
    pub(crate) rows: Vec<Row>,
}

// ── Connection ────────────────────────────────────────────────────────────────

/// Open a synchronous PostgreSQL connection from individual parameters.
pub fn postgres_connect(
    host: &str,
    user: &str,
    pass: &str,
    db: &str,
) -> Result<PostgresConn, CandyError> {
    let url = format!(
        "host={} user={} password={} dbname={}",
        host, user, pass, db
    );
    postgres_connect_url(&url)
}

/// Open a synchronous PostgreSQL connection from a full connection string.
///
/// Accepts both keyword-value (`host=localhost user=foo`) and URI
/// (`postgresql://foo:bar@localhost/mydb`) formats.
pub fn postgres_connect_url(url: &str) -> Result<PostgresConn, CandyError> {
    let client = Client::connect(url, NoTls).map_err(|e| CandyError::Connection(e.to_string()))?;
    Ok(PostgresConn { client })
}

// ── Query ─────────────────────────────────────────────────────────────────────

/// Execute a SELECT-style query and buffer all rows.
pub fn postgres_query(conn: &mut PostgresConn, sql: &str) -> Result<PostgresResult, CandyError> {
    let rows = conn
        .client
        .query(sql, &[])
        .map_err(|e| CandyError::Query(e.to_string()))?;
    Ok(PostgresResult { rows })
}

// ── Fetch ─────────────────────────────────────────────────────────────────────

/// Convert a [`PostgresResult`] into a vector of string maps.
pub fn postgres_fetch_all(res: PostgresResult) -> Result<Vec<HashMap<String, String>>, CandyError> {
    res.rows.iter().map(row_to_map).collect()
}

/// Return only the first row of a [`PostgresResult`].
pub fn postgres_fetch_one(res: PostgresResult) -> Result<HashMap<String, String>, CandyError> {
    match res.rows.first() {
        Some(row) => row_to_map(row),
        None => Err(CandyError::Fetch("Result set is empty".into())),
    }
}

// ── Mutating statements ───────────────────────────────────────────────────────

/// Execute INSERT / UPDATE / DELETE and return the number of affected rows.
pub fn postgres_exec(conn: &mut PostgresConn, sql: &str) -> Result<u64, CandyError> {
    let n = conn
        .client
        .execute(sql, &[])
        .map_err(|e| CandyError::Query(e.to_string()))?;
    Ok(n)
}

// ── Transaction ───────────────────────────────────────────────────────────────

/// Execute a slice of SQL statements inside a single transaction.
///
/// Rolls back automatically on any failure.
pub fn postgres_transaction(conn: &mut PostgresConn, queries: &[&str]) -> Result<(), CandyError> {
    let mut tx = conn
        .client
        .transaction()
        .map_err(|e| CandyError::Transaction(e.to_string()))?;

    for sql in queries {
        if let Err(e) = tx.execute(*sql, &[]) {
            let _ = tx.rollback();
            return Err(CandyError::Transaction(e.to_string()));
        }
    }

    tx.commit()
        .map_err(|e| CandyError::Transaction(e.to_string()))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_map(row: &Row) -> Result<HashMap<String, String>, CandyError> {
    let mut map = HashMap::new();
    for (i, col) in row.columns().iter().enumerate() {
        let s = pg_col_to_string(row, i, col.type_()).map_err(|e| CandyError::Fetch(e))?;
        map.insert(col.name().to_string(), s);
    }
    Ok(map)
}

fn pg_col_to_string(row: &Row, idx: usize, ty: &postgres::types::Type) -> Result<String, String> {
    use postgres::types::Type;

    macro_rules! try_get {
        ($t:ty) => {{
            let v: Option<$t> = row.try_get(idx).map_err(|e| e.to_string())?;
            return Ok(v
                .map(|x| x.to_string())
                .unwrap_or_else(|| "NULL".to_string()));
        }};
    }

    match *ty {
        Type::BOOL => try_get!(bool),
        Type::INT2 => try_get!(i16),
        Type::INT4 => try_get!(i32),
        Type::INT8 => try_get!(i64),
        Type::FLOAT4 => try_get!(f32),
        Type::FLOAT8 => try_get!(f64),
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME => try_get!(String),
        Type::BYTEA => {
            let v: Option<Vec<u8>> = row.try_get(idx).map_err(|e| e.to_string())?;
            Ok(v.map(|b| format!("{:?}", b))
                .unwrap_or_else(|| "NULL".to_string()))
        }
        // Fall back to text representation for everything else
        _ => {
            let v: Option<String> = row.try_get(idx).ok().flatten();
            Ok(v.unwrap_or_else(|| "NULL".to_string()))
        }
    }
}
