//! Identity parsing from live SQL probe rows (CONN-06).
//!
//! The native client discards the auth handshake's `AuthResponse`, so the studio
//! derives the connected user/role/current-database via a best-effort SQL probe
//! (`SELECT current_user, current_role, current_database`) plus `SHOW DATABASES`.
//! Parsing is total and panic-free: missing columns, non-string cells, and empty
//! results all fall back to documented defaults so a connect never fails on a
//! surprising probe shape (research Target 2).
//!
//! Fallback chain (research Target 2 table):
//! - user -> `form_user` -> `conn_name`
//! - role -> `""` (the chip simply shows no role)
//! - current_database -> `default_db`
//! - databases -> `vec![current_database]`

use nodedb_types::{QueryResult, Value};

/// The parsed live-session identity for the connection chip.
///
/// Consumed by the live connect-and-probe path in Plan 02; built and unit-tested
/// here in Plan 01 (Wave 0 scaffolding), hence the scoped `dead_code` allow.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub user: String,
    pub role: String,
    pub current_database: String,
}

/// Read a named column from the first row of a probe result, trimmed.
///
/// Returns `None` if the result/row/column is absent or the cell is not a
/// non-empty string. Never panics (safe accessor only).
#[allow(dead_code)]
fn first_row_cell<'a>(who: &'a QueryResult, column: &str) -> Option<&'a str> {
    let idx = who.columns.iter().position(|c| c == column)?;
    let row = who.rows.first()?;
    let cell = row.get(idx)?;
    let text = cell.as_str()?.trim();
    if text.is_empty() { None } else { Some(text) }
}

/// Parse identity from a best-effort probe result, applying fallbacks.
///
/// `who` is the optional `SELECT current_user, current_role, current_database`
/// result. `form_user` is the username the user typed (trust/password modes),
/// `conn_name` the saved-connection name, `default_db` the saved default
/// database. Any missing piece falls back per the module-level chain.
///
/// Consumed by the live connect path in Plan 02; scaffolded + tested in Plan 01.
#[allow(dead_code)]
pub fn parse_identity(
    who: Option<&QueryResult>,
    form_user: Option<&str>,
    conn_name: &str,
    default_db: &str,
) -> Identity {
    let probed_user = who.and_then(|r| first_row_cell(r, "current_user"));
    let probed_role = who.and_then(|r| first_row_cell(r, "current_role"));
    let probed_db = who.and_then(|r| first_row_cell(r, "current_database"));

    let user = probed_user
        .or(form_user.map(str::trim).filter(|s| !s.is_empty()))
        .unwrap_or(conn_name)
        .to_string();
    let role = probed_role.unwrap_or_default().to_string();
    let current_database = probed_db.unwrap_or(default_db).to_string();

    Identity {
        user,
        role,
        current_database,
    }
}

/// Parse the database list from a `SHOW DATABASES` result.
///
/// Collects the first column of every row that is a non-empty string. If the
/// result is `None` or yields nothing, returns `vec![current_database]` so the
/// chip always has at least the connected database (research Target 2).
///
/// Consumed by the live connect path in Plan 02; scaffolded + tested in Plan 01.
#[allow(dead_code)]
pub fn parse_databases(dbs: Option<&QueryResult>, current_database: &str) -> Vec<String> {
    let parsed: Vec<String> = dbs
        .map(|r| {
            r.rows
                .iter()
                .filter_map(|row| row.first())
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    if parsed.is_empty() {
        vec![current_database.to_string()]
    } else {
        parsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::String(text.to_string())
    }

    fn result(columns: &[&str], rows: Vec<Vec<Value>>) -> QueryResult {
        QueryResult {
            columns: columns.iter().map(|c| c.to_string()).collect(),
            rows,
            rows_affected: 0,
        }
    }

    #[test]
    fn identity_parse_full_row() {
        let who = result(
            &["current_user", "current_role", "current_database"],
            vec![vec![s("root"), s("admin"), s("analytics")]],
        );
        let id = parse_identity(Some(&who), Some("alice"), "conn", "mydb");
        assert_eq!(id.user, "root");
        assert_eq!(id.role, "admin");
        assert_eq!(id.current_database, "analytics");
    }

    #[test]
    fn identity_parse_missing_columns_uses_fallbacks() {
        // Empty result (no rows): falls back to form_user + default_db.
        let empty = result(&[], vec![]);
        let id = parse_identity(Some(&empty), Some("alice"), "conn", "mydb");
        assert_eq!(id.user, "alice");
        assert_eq!(id.role, "");
        assert_eq!(id.current_database, "mydb");
    }

    #[test]
    fn identity_parse_no_form_user_falls_back_to_name() {
        let empty = result(&[], vec![]);
        let id = parse_identity(Some(&empty), None, "prod", "mydb");
        assert_eq!(id.user, "prod");
    }

    #[test]
    fn databases_parse_from_show_databases() {
        let dbs = result(&["name"], vec![vec![s("a")], vec![s("b")]]);
        assert_eq!(parse_databases(Some(&dbs), "fallback"), vec!["a", "b"]);

        // Empty / None -> just the current database.
        let empty = result(&["name"], vec![]);
        assert_eq!(parse_databases(Some(&empty), "cur"), vec!["cur"]);
        assert_eq!(parse_databases(None, "cur"), vec!["cur"]);
    }
}
