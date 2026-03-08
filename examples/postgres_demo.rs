//! # PostgreSQL Demo
//!
//! Requires a running PostgreSQL server.
//!
//! ```bash
//! export CANDY_DB_URL="postgres://postgres:secret@localhost/candy_demo"
//! cargo run --example postgres_demo --features postgres
//! ```

use candybase::*;
use std::env;

fn main() -> Result<(), CandyError> {
    println!("=== Candy PostgreSQL Demo ===\n");

    let args: Vec<String> = env::args().collect();

    let conn = if args.len() >= 5 {
        candy_connect(&args[1], &args[2], &args[3], &args[4])?
    } else if env::var("CANDY_DB_URL").is_ok() {
        candy_connect("", "", "", "")?
    } else {
        eprintln!("Usage: postgres_demo <host> <user> <pass> <db>");
        eprintln!(
            "   or: CANDY_DB_URL=postgres://user:pass@host/db cargo run --example postgres_demo"
        );
        std::process::exit(1);
    };

    println!("[+] Connected to PostgreSQL");

    // ── Schema setup ─────────────────────────────────────────────────────────
    candy_query(&conn, "DROP TABLE IF EXISTS candy_users")?;
    candy_query(
        &conn,
        "CREATE TABLE candy_users (
             id   SERIAL PRIMARY KEY,
             name VARCHAR(100) NOT NULL,
             age  INTEGER
         )",
    )?;
    println!("[+] Table `candy_users` created");

    // ── Inserts ───────────────────────────────────────────────────────────────
    candy_insert(
        &conn,
        "INSERT INTO candy_users (name, age) VALUES ('Alice', 30)",
    )?;
    candy_insert(
        &conn,
        "INSERT INTO candy_users (name, age) VALUES ('Bob', 25)",
    )?;
    candy_insert(
        &conn,
        "INSERT INTO candy_users (name, age) VALUES ('Carol', 35)",
    )?;
    println!("[+] Inserted 3 users");

    // ── Select all ────────────────────────────────────────────────────────────
    let res = candy_query(&conn, "SELECT * FROM candy_users")?;
    let users = candy_fetch_all(res)?;
    println!("\n--- All users ---");
    for u in &users {
        println!("  id={} name={} age={}", u["id"], u["name"], u["age"]);
    }

    // ── Fetch one ─────────────────────────────────────────────────────────────
    let res = candy_query(&conn, "SELECT * FROM candy_users WHERE name = 'Alice'")?;
    let alice = candy_fetch_one(res)?;
    println!("\n--- Fetch one (Alice) ---\n  {:?}", alice);

    // ── Update + Delete ───────────────────────────────────────────────────────
    candy_update(
        &conn,
        "UPDATE candy_users SET age = 31 WHERE name = 'Alice'",
    )?;
    candy_delete(&conn, "DELETE FROM candy_users WHERE name = 'Carol'")?;
    println!("[+] Alice updated, Carol deleted");

    // ── Transaction ───────────────────────────────────────────────────────────
    candy_transaction(
        &conn,
        vec![
            "INSERT INTO candy_users (name, age) VALUES ('Dave', 40)",
            "UPDATE candy_users SET age = age + 1 WHERE name = 'Bob'",
        ],
    )?;
    println!("[+] Transaction committed");

    // ── Final state ───────────────────────────────────────────────────────────
    let res = candy_query(&conn, "SELECT * FROM candy_users ORDER BY id")?;
    let users = candy_fetch_all(res)?;
    println!("\n--- Final state ---");
    for u in &users {
        println!("  id={} name={} age={}", u["id"], u["name"], u["age"]);
    }

    candy_close(conn)?;
    println!("\n[+] Done!");
    Ok(())
}
