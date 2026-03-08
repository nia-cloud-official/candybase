//! # SQLite Demo
//!
//! Run with:
//! ```
//! cargo run --example sqlite_demo --features sqlite
//! ```

use candybase::*;

fn main() -> Result<(), CandyError> {
    println!("=== Candy SQLite Demo ===\n");

    // Connect to an in-memory SQLite database
    let conn = candy_connect("", "", "", ":memory:")?;
    println!("[+] Connected to SQLite :memory:");

    // ── Schema setup ──────────────────────────────────────────────────────────
    candy_query(
        &conn,
        "CREATE TABLE users (
             id   INTEGER PRIMARY KEY AUTOINCREMENT,
             name TEXT    NOT NULL,
             age  INTEGER
         )",
    )?;
    println!("[+] Table `users` created");

    // ── Inserts ───────────────────────────────────────────────────────────────
    let n = candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Alice', 30)")?;
    println!("[+] Inserted {} row (Alice)", n);

    candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Bob', 25)")?;
    candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Carol', 35)")?;
    println!("[+] Inserted Bob and Carol");

    // ── Select all ───────────────────────────────────────────────────────────
    let res = candy_query(&conn, "SELECT * FROM users")?;
    let users = candy_fetch_all(res)?;
    println!("\n--- All users ---");
    for u in &users {
        println!("  id={} name={} age={}", u["id"], u["name"], u["age"]);
    }

    // ── Select one ───────────────────────────────────────────────────────────
    let res = candy_query(&conn, "SELECT * FROM users WHERE name = 'Alice'")?;
    let alice = candy_fetch_one(res)?;
    println!("\n--- Fetch one (Alice) ---");
    println!("  {:?}", alice);

    // ── Update ───────────────────────────────────────────────────────────────
    let updated = candy_update(&conn, "UPDATE users SET age = 31 WHERE name = 'Alice'")?;
    println!("\n[+] Updated {} row(s) — Alice is now 31", updated);

    // ── Delete ───────────────────────────────────────────────────────────────
    let deleted = candy_delete(&conn, "DELETE FROM users WHERE name = 'Bob'")?;
    println!("[+] Deleted {} row(s) — Bob removed", deleted);

    // ── Transaction ──────────────────────────────────────────────────────────
    println!("\n--- Running transaction ---");
    candy_transaction(
        &conn,
        vec![
            "INSERT INTO users (name, age) VALUES ('Dave', 40)",
            "INSERT INTO users (name, age) VALUES ('Eve', 28)",
            "UPDATE users SET age = age + 1 WHERE name = 'Carol'",
        ],
    )?;
    println!("[+] Transaction committed");

    // ── Final state ───────────────────────────────────────────────────────────
    let res = candy_query(&conn, "SELECT * FROM users ORDER BY id")?;
    let users = candy_fetch_all(res)?;
    println!("\n--- Final state ---");
    for u in &users {
        println!("  id={} name={} age={}", u["id"], u["name"], u["age"]);
    }

    // ── Count ────────────────────────────────────────────────────────────────
    let res = candy_query(&conn, "SELECT COUNT(*) AS cnt FROM users")?;
    let row = candy_fetch_one(res)?;
    println!("\n[+] Total users: {}", row["cnt"]);

    // ── Close ────────────────────────────────────────────────────────────────
    candy_close(conn)?;
    println!("\n[+] Connection closed. Done!");

    Ok(())
}
