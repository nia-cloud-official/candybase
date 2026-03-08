//! # Integration Tests
//!
//! SQLite tests run automatically (no server required).
//! MySQL and PostgreSQL tests require live servers and the matching feature flags.
//!
//! ## Running SQLite tests only
//! ```bash
//! cargo test --features sqlite
//! ```
//!
//! ## Running all tests (requires running MySQL + PostgreSQL)
//! ```bash
//! MYSQL_URL="mysql://root:secret@localhost/test"   \
//! PG_URL="postgres://postgres:secret@localhost/test" \
//! cargo test --features all
//! ```

// ─────────────────────────────────────────────────────────────────────────────
// SQLite integration tests (always run when `sqlite` feature is enabled)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use candybase::*;

    fn setup() -> CandyConn {
        let conn = candy_connect("", "", "", ":memory:").expect("SQLite connect");
        candy_query(
            &conn,
            "CREATE TABLE users (
                 id   INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT    NOT NULL,
                 age  INTEGER
             )",
        )
        .expect("CREATE TABLE");
        conn
    }

    #[test]
    fn test_insert_and_fetch_all() {
        let conn = setup();
        let n = candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Alice', 30)")
            .expect("insert");
        assert_eq!(n, 1);

        candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Bob', 25)")
            .expect("insert bob");

        let res = candy_query(&conn, "SELECT * FROM users ORDER BY id").expect("query");
        let rows = candy_fetch_all(res).expect("fetch_all");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[1]["name"], "Bob");
    }

    #[test]
    fn test_fetch_one() {
        let conn = setup();
        candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Carol', 35)").expect("insert");

        let res = candy_query(&conn, "SELECT * FROM users WHERE name='Carol'").expect("query");
        let row = candy_fetch_one(res).expect("fetch_one");
        assert_eq!(row["name"], "Carol");
        assert_eq!(row["age"], "35");
    }

    #[test]
    fn test_fetch_one_empty_errors() {
        let conn = setup();
        let res = candy_query(&conn, "SELECT * FROM users").expect("query");
        let err = candy_fetch_one(res);
        assert!(matches!(err, Err(CandyError::Fetch(_))));
    }

    #[test]
    fn test_update() {
        let conn = setup();
        candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Dave', 20)").expect("insert");

        let n =
            candy_update(&conn, "UPDATE users SET age = 21 WHERE name = 'Dave'").expect("update");
        assert_eq!(n, 1);

        let res = candy_query(&conn, "SELECT age FROM users WHERE name='Dave'").expect("query");
        let row = candy_fetch_one(res).expect("fetch");
        assert_eq!(row["age"], "21");
    }

    #[test]
    fn test_delete() {
        let conn = setup();
        candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Eve', 22)").expect("insert");

        let n = candy_delete(&conn, "DELETE FROM users WHERE name='Eve'").expect("delete");
        assert_eq!(n, 1);

        let res = candy_query(&conn, "SELECT * FROM users").expect("query");
        let rows = candy_fetch_all(res).expect("fetch");
        assert!(rows.is_empty());
    }

    #[test]
    fn test_transaction_commit() {
        let conn = setup();
        candy_transaction(
            &conn,
            vec![
                "INSERT INTO users (name, age) VALUES ('Frank', 40)",
                "INSERT INTO users (name, age) VALUES ('Grace', 38)",
            ],
        )
        .expect("transaction");

        let res = candy_query(&conn, "SELECT * FROM users ORDER BY id").expect("query");
        let rows = candy_fetch_all(res).expect("fetch");
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_transaction_rollback_on_error() {
        let conn = setup();
        candy_insert(&conn, "INSERT INTO users (name, age) VALUES ('Henry', 50)").expect("insert");

        let result = candy_transaction(
            &conn,
            vec![
                "INSERT INTO users (name, age) VALUES ('Ivy', 29)",
                "THIS IS NOT VALID SQL !!!", // <-- will fail
            ],
        );
        assert!(result.is_err());

        // Henry should still be there, but Ivy should not (rolled back)
        let res = candy_query(&conn, "SELECT * FROM users").expect("query");
        let rows = candy_fetch_all(res).expect("fetch");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Henry");
    }

    #[test]
    fn test_candy_close() {
        let conn = setup();
        let result = candy_close(conn);
        assert!(result.is_ok());
    }

    #[test]
    fn test_env_url_sqlite() {
        std::env::set_var("CANDY_DB_URL", "sqlite://:memory:");
        let conn = candy_connect("", "", "", "").expect("env url connect");
        candy_close(conn).expect("close");
        std::env::remove_var("CANDY_DB_URL");
    }

    #[test]
    fn test_candy_connect_url_sqlite() {
        let conn = candy_connect_url("sqlite://:memory:").expect("connect_url");
        let res = candy_query(&conn, "SELECT 42 AS answer").expect("query");
        let row = candy_fetch_one(res).expect("fetch");
        assert_eq!(row["answer"], "42");
        candy_close(conn).expect("close");
    }

    #[test]
    fn test_multiple_queries_same_conn() {
        let conn = setup();
        for i in 0..10 {
            candy_insert(
                &conn,
                &format!(
                    "INSERT INTO users (name, age) VALUES ('User{}', {})",
                    i,
                    i + 20
                ),
            )
            .expect("insert");
        }
        let res = candy_query(&conn, "SELECT COUNT(*) AS cnt FROM users").expect("query");
        let row = candy_fetch_one(res).expect("fetch");
        assert_eq!(row["cnt"], "10");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MySQL integration tests (require live server + `mysql` feature)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "mysql")]
mod mysql_tests {
    use candybase::*;

    fn mysql_url() -> Option<String> {
        std::env::var("MYSQL_URL").ok()
    }

    fn setup(url: &str) -> CandyConn {
        let conn = candy_connect_url(url).expect("MySQL connect");
        candy_query(&conn, "DROP TABLE IF EXISTS candy_test_users").ok();
        candy_query(
            &conn,
            "CREATE TABLE candy_test_users (
                 id   INT AUTO_INCREMENT PRIMARY KEY,
                 name VARCHAR(100),
                 age  INT
             )",
        )
        .expect("CREATE TABLE");
        conn
    }

    #[test]
    fn test_mysql_basic() {
        let url = match mysql_url() {
            Some(u) => u,
            None => {
                eprintln!("Skipping MySQL tests: MYSQL_URL not set");
                return;
            }
        };
        let conn = setup(&url);

        candy_insert(
            &conn,
            "INSERT INTO candy_test_users (name, age) VALUES ('Alice', 30)",
        )
        .expect("insert");

        let res = candy_query(&conn, "SELECT * FROM candy_test_users").expect("query");
        let rows = candy_fetch_all(res).expect("fetch");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Alice");

        candy_close(conn).expect("close");
    }

    #[test]
    fn test_mysql_transaction() {
        let url = match mysql_url() {
            Some(u) => u,
            None => return,
        };
        let conn = setup(&url);

        candy_transaction(
            &conn,
            vec![
                "INSERT INTO candy_test_users (name, age) VALUES ('Bob', 25)",
                "INSERT INTO candy_test_users (name, age) VALUES ('Carol', 35)",
            ],
        )
        .expect("transaction");

        let res =
            candy_query(&conn, "SELECT COUNT(*) AS cnt FROM candy_test_users").expect("query");
        let row = candy_fetch_one(res).expect("fetch");
        assert_eq!(row["cnt"], "2");

        candy_close(conn).expect("close");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PostgreSQL integration tests (require live server + `postgres` feature)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "postgres")]
mod postgres_tests {
    use candybase::*;

    fn pg_url() -> Option<String> {
        std::env::var("PG_URL").ok()
    }

    fn setup(url: &str) -> CandyConn {
        let conn = candy_connect_url(url).expect("PG connect");
        candy_query(&conn, "DROP TABLE IF EXISTS candy_test_users").ok();
        candy_query(
            &conn,
            "CREATE TABLE candy_test_users (
                 id   SERIAL PRIMARY KEY,
                 name VARCHAR(100),
                 age  INTEGER
             )",
        )
        .expect("CREATE TABLE");
        conn
    }

    #[test]
    fn test_pg_basic() {
        let url = match pg_url() {
            Some(u) => u,
            None => {
                eprintln!("Skipping PG tests: PG_URL not set");
                return;
            }
        };
        let conn = setup(&url);

        candy_insert(
            &conn,
            "INSERT INTO candy_test_users (name, age) VALUES ('Alice', 30)",
        )
        .expect("insert");

        let res = candy_query(&conn, "SELECT * FROM candy_test_users").expect("query");
        let rows = candy_fetch_all(res).expect("fetch");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Alice");

        candy_close(conn).expect("close");
    }

    #[test]
    fn test_pg_transaction() {
        let url = match pg_url() {
            Some(u) => u,
            None => return,
        };
        let conn = setup(&url);

        candy_transaction(
            &conn,
            vec![
                "INSERT INTO candy_test_users (name, age) VALUES ('Bob', 25)",
                "INSERT INTO candy_test_users (name, age) VALUES ('Carol', 35)",
            ],
        )
        .expect("transaction");

        let res =
            candy_query(&conn, "SELECT COUNT(*) AS cnt FROM candy_test_users").expect("query");
        let row = candy_fetch_one(res).expect("fetch");
        assert_eq!(row["cnt"], "2");

        candy_close(conn).expect("close");
    }
}
