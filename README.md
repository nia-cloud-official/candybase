# Candybase — Procedural Database Access for Rust

[![Crates.io](https://img.shields.io/crates/v/candybase.svg)](https://crates.io/crates/candybase)
[![Docs.rs](https://docs.rs/candybase/badge.svg)](https://docs.rs/candybase)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20-blue.svg)](LICENSE)

> **As simple as PHP's `mysqli_*` functions. As safe as Rust.**


## The Idea 

Most Rust database crates require you to learn builders, traits, generics, and async runtimes before you can run a single query. Candy takes the opposite approach: **flat functions, immediate results, no ceremony**.

If you have ever written PHP like this:

```php
$conn = mysqli_connect("localhost", "root", "secret", "shop");
$res  = mysqli_query($conn, "SELECT * FROM products");
$rows = mysqli_fetch_all($res, MYSQLI_ASSOC);
```

Then Candy will feel familiar:

```rust
use candybase::*;

let conn  = candy_connect("localhost", "root", "secret", "shop")?;
let res   = candy_query(&conn, "SELECT * FROM products")?;
let rows  = candy_fetch_all(res)?;
```

### Design Goals

| Goal | How Candy achieves it |
|------|----------------------|
| **Simplicity** | Flat procedural functions, no builders, no traits |
| **Safety** | Every function returns `Result<T, CandyError>` |
| **Universality** | MySQL, PostgreSQL, and SQLite through one API |
| **Framework-agnostic** | Works in Axum, Rocket, Yew, or plain `main()` |
| **Familiar** | Mirrors the mental model of `mysqli_*` |

---

## Installation

Add Candy to `Cargo.toml` with the backend(s) you need:

```toml
[dependencies]
# SQLite only (default, no server required)
candybase = "0.1"

# MySQL
candybase = { version = "0.1", features = ["mysql"] }

# PostgreSQL
candybase = { version = "0.1", features = ["postgres"] }

# All three backends
candybase = { version = "0.1", features = ["all"] }
```

### Feature Flags

| Flag | Backend | External dependency? |
|------|---------|---------------------|
| `sqlite` (default) | SQLite | No — bundled via `rusqlite` |
| `mysql` | MySQL / MariaDB | Requires a running server |
| `postgres` | PostgreSQL | Requires a running server |
| `all` | All three | — |

---

## Quick Start

### SQLite (no server needed)

```rust
use candybase::*;

fn main() -> Result<(), CandyError> {
    // Connect to an in-memory database
    let conn = candy_connect("", "", "", ":memory:")?;

    // Create a table
    candy_query(&conn, "
        CREATE TABLE users (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT    NOT NULL,
            age  INTEGER
        )
    ")?;

    // Insert rows
    candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Alice', 30)")?;
    candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Bob',   25)")?;

    // Fetch all rows
    let res   = candy_query(&conn, "SELECT * FROM users")?;
    let users = candy_fetch_all(res)?;

    for u in &users {
        println!("id={} name={} age={}", u["id"], u["name"], u["age"]);
    }

    // Fetch a single row
    let res   = candy_query(&conn, "SELECT * FROM users WHERE name='Alice'")?;
    let alice = candy_fetch_one(res)?;
    println!("Alice's age: {}", alice["age"]);

    // Update and delete
    candy_update(&conn, "UPDATE users SET age = 31 WHERE name='Alice'")?;
    candy_delete(&conn, "DELETE FROM users WHERE name='Bob'")?;

    // Atomic transaction
    candy_transaction(&conn, vec![
        "INSERT INTO users (name, age) VALUES ('Carol', 35)",
        "UPDATE users SET age = age + 1 WHERE name = 'Alice'",
    ])?;

    // Close (optional — Drop cleans up too)
    candy_close(conn)?;
    Ok(())
}
```

### MySQL

```rust
use candybase::*;

fn main() -> Result<(), CandyError> {
    let conn = candy_connect("localhost", "root", "secret", "mydb")?;
    // ... same API as SQLite above ...
    candy_close(conn)
}
```

### PostgreSQL

```rust
use candybase::*;

fn main() -> Result<(), CandyError> {
    let conn = candy_connect("localhost", "postgres", "secret", "mydb")?;
    // ... same API ...
    candy_close(conn)
}
```

---

## Environment Variable

Set `CANDY_DB_URL` and call `candy_connect` with empty strings:

```bash
export CANDY_DB_URL="mysql://root:secret@localhost/shop"
```

```rust
// Reads CANDY_DB_URL automatically
let conn = candy_connect("", "", "", "")?;
```

Or use `candy_connect_url` directly:

```rust
let conn = candy_connect_url("sqlite:///path/to/mydb.sqlite")?;
let conn = candy_connect_url("mysql://user:pass@host/db")?;
let conn = candy_connect_url("postgres://user:pass@host/db")?;
```

---

## API Reference

### `candy_connect`

```rust
pub fn candy_connect(host: &str, user: &str, pass: &str, db: &str)
    -> Result<CandyConn, CandyError>
```

Opens a connection. Backend is inferred from `CANDY_DB_URL`, the `host` scheme, or the enabled feature flags. Returns a `CandyConn` handle.

---

### `candy_connect_url`

```rust
pub fn candy_connect_url(url: &str) -> Result<CandyConn, CandyError>
```

Opens a connection from a full URL string. URL scheme selects the backend:

| URL prefix | Backend |
|------------|---------|
| `mysql://` | MySQL |
| `postgres://` or `postgresql://` | PostgreSQL |
| `sqlite://` | SQLite |

---

### `candy_query`

```rust
pub fn candy_query(conn: &CandyConn, sql: &str) -> Result<CandyResult, CandyError>
```

Executes a SQL statement and returns a buffered `CandyResult`. Intended for `SELECT` statements. All rows are fetched into memory.

---

### `candy_fetch_all`

```rust
pub fn candy_fetch_all(res: CandyResult)
    -> Result<Vec<HashMap<String, String>>, CandyError>
```

Consumes a `CandyResult` and returns every row as a `Vec<HashMap<String, String>>`. All column values are strings; `NULL` becomes `"NULL"`.

---

### `candy_fetch_one`

```rust
pub fn candy_fetch_one(res: CandyResult)
    -> Result<HashMap<String, String>, CandyError>
```

Returns only the first row. Returns `CandyError::Fetch` if the result set is empty.

---

### `candy_insert`

```rust
pub fn candy_insert(conn: &CandyConn, sql: &str) -> Result<u64, CandyError>
```

Executes an `INSERT` statement. Returns the number of rows inserted.

---

### `candy_update`

```rust
pub fn candy_update(conn: &CandyConn, sql: &str) -> Result<u64, CandyError>
```

Executes an `UPDATE` statement. Returns the number of rows affected.

---

### `candy_delete`

```rust
pub fn candy_delete(conn: &CandyConn, sql: &str) -> Result<u64, CandyError>
```

Executes a `DELETE` statement. Returns the number of rows deleted.

---

### `candy_transaction`

```rust
pub fn candy_transaction(conn: &CandyConn, queries: Vec<&str>)
    -> Result<(), CandyError>
```

Executes a list of SQL statements as a single atomic transaction. Automatically rolls back if any statement fails and returns `CandyError::Transaction`.

---

### `candy_close`

```rust
pub fn candy_close(conn: CandyConn) -> Result<(), CandyError>
```

Closes the connection and releases resources. The `CandyConn` is consumed. Dropping a `CandyConn` without calling `candy_close` is also safe.

---

## Error Handling

```rust
use candybase::{candy_connect, CandyError};

match candy_connect("localhost", "root", "wrong_password", "db") {
    Ok(conn)                       => { /* connected */ }
    Err(CandyError::Connection(m)) => eprintln!("Cannot connect: {}", m),
    Err(CandyError::Query(m))      => eprintln!("Query failed: {}", m),
    Err(CandyError::Fetch(m))      => eprintln!("Fetch failed: {}", m),
    Err(CandyError::Transaction(m))=> eprintln!("TX failed: {}", m),
    Err(e)                         => eprintln!("Error: {}", e),
}
```

| Variant | When raised |
|---------|-------------|
| `CandyError::Connection` | Cannot reach the server / bad credentials |
| `CandyError::Query` | SQL statement rejected by the database |
| `CandyError::Fetch` | Row decoding failed / empty result set |
| `CandyError::Transaction` | A statement inside a transaction failed |
| `CandyError::UrlParse` | Unrecognised URL scheme |
| `CandyError::Internal` | Unexpected internal error |

---

## Running the Examples

```bash
# SQLite (no server needed)
cargo run --example sqlite_demo --features sqlite

# MySQL (requires a running server)
export CANDY_DB_URL="mysql://root:secret@localhost/test"
cargo run --example mysql_demo --features mysql

# PostgreSQL (requires a running server)
export CANDY_DB_URL="postgres://postgres:secret@localhost/test"
cargo run --example postgres_demo --features postgres
```

---

## Running Tests

```bash
# SQLite unit tests (always available)
cargo test --features sqlite

# All backends (requires live MySQL + PostgreSQL)
MYSQL_URL="mysql://root:secret@localhost/test" \
PG_URL="postgres://postgres:secret@localhost/test" \
cargo test --features all
```

---

## Using with Web Frameworks

### Axum

```rust
use axum::{Router, routing::get, Json};
use candybase::*;
use std::collections::HashMap;

async fn list_users() -> Json<Vec<HashMap<String, String>>> {
    let conn  = candy_connect("localhost", "root", "secret", "shop").unwrap();
    let res   = candy_query(&conn, "SELECT * FROM users").unwrap();
    let users = candy_fetch_all(res).unwrap();
    Json(users)
}
```

### Rocket

```rust
use rocket::get;
use candybase::*;

#[get("/users")]
fn users() -> String {
    let conn  = candy_connect("localhost", "root", "secret", "shop").unwrap();
    let res   = candy_query(&conn, "SELECT * FROM users").unwrap();
    let rows  = candy_fetch_all(res).unwrap();
    format!("{:?}", rows)
}
```

---

## Comparison with Other Crates

| Crate | Style | Backends | Learning curve |
|-------|-------|----------|----------------|
| **candy** | Procedural functions | MySQL, PG, SQLite | Minimal |
| sqlx | Async, macro-heavy | MySQL, PG, SQLite | Steep |
| diesel | ORM, schema-first | MySQL, PG, SQLite | Very steep |
| rusqlite | Low-level | SQLite only | Moderate |
| tokio-postgres | Async, low-level | PG only | Steep |

Candy is not a replacement for sqlx or Diesel in production systems that need async, connection pooling, migrations, or type-safe queries. It is the right choice when you want to **get something working quickly** with the least possible friction.

---

## Roadmap

- [ ] Async support (`candy_async` companion crate)
- [ ] Prepared statement API (`candy_prepare` / `candy_execute`)
- [ ] Connection pool helpers
- [ ] Named parameter binding (`?name` style)
- [ ] `serde` deserialization into typed structs

---
Licensed Under [MIT License](License)
