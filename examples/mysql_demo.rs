//! # MySQL Demo
//!
//! Requires a running MySQL / MariaDB server.
//!
//! Set the `CANDY_DB_URL` environment variable before running:
//!
//! ```bash
//! export CANDY_DB_URL="mysql://root:secret@localhost/candy_demo"
//! cargo run --example mysql_demo --features mysql
//! ```
//!
//! Or pass credentials directly via command-line args:
//!
//! ```bash
//! cargo run --example mysql_demo --features mysql -- localhost root secret candy_demo
//! ```

use candybase::*;
use std::env;

fn main() -> Result<(), CandyError> {
    println!("=== Candy MySQL Demo ===\n");

    // Gather connection parameters (args or environment)
    let args: Vec<String> = env::args().collect();

    let conn = if args.len() >= 5 {
        candy_connect(&args[1], &args[2], &args[3], &args[4])?
    } else if env::var("CANDY_DB_URL").is_ok() {
        candy_connect("", "", "", "")? // reads CANDY_DB_URL internally
    } else {
        eprintln!("Usage: mysql_demo <host> <user> <pass> <db>");
        eprintln!("   or: CANDY_DB_URL=mysql://user:pass@host/db cargo run --example mysql_demo");
        std::process::exit(1);
    };

    println!("[+] Connected to MySQL");

    // ── Schema setup ─────────────────────────────────────────────────────────
    candy_query(&conn, "DROP TABLE IF EXISTS candy_users")?;
    candy_query(
        &conn,
        "CREATE TABLE candy_users (
             id   INT AUTO_INCREMENT PRIMARY KEY,
             name VARCHAR(100) NOT NULL,
             age  INT
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
    let res = candy_query(&conn, "SELECT * FROM candy_users WHERE name = 'Bob'")?;
    let bob = candy_fetch_one(res)?;
    println!("\n--- Fetch one (Bob) ---\n  {:?}", bob);

    // ── Update ────────────────────────────────────────────────────────────────
    candy_update(&conn, "UPDATE candy_users SET age = 26 WHERE name = 'Bob'")?;
    println!("\n[+] Bob's age updated to 26");

    // ── Delete ────────────────────────────────────────────────────────────────
    candy_delete(&conn, "DELETE FROM candy_users WHERE name = 'Carol'")?;
    println!("[+] Carol deleted");

    // ── Transaction ───────────────────────────────────────────────────────────
    candy_transaction(
        &conn,
        vec![
            "INSERT INTO candy_users (name, age) VALUES ('Dave', 40)",
            "UPDATE candy_users SET age = age + 1 WHERE name = 'Alice'",
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
