use crate::commands::with_conn;
use crate::state::DbState;

use rusqlite::Connection;
use tauri::State;

pub fn get_config_impl(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT config_value FROM app_config WHERE config_key = ?1",
        rusqlite::params![key],
        |r| r.get(0),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other.to_string()),
    })
}

pub fn set_config_impl(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_config (config_key, config_value) VALUES (?1, ?2)
         ON CONFLICT(config_key) DO UPDATE SET config_value = excluded.config_value",
        rusqlite::params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_config(db: State<DbState>, key: String) -> Result<Option<String>, String> {
    with_conn(&db, |c| get_config_impl(c, &key))
}

#[tauri::command]
pub fn set_config(db: State<DbState>, key: String, value: String) -> Result<(), String> {
    with_conn(&db, |c| set_config_impl(c, &key, &value))
}
