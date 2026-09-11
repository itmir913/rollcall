//! 앱 전체에 걸린 키-값. **담는 것은 "마지막에 연 학교·학년도·학급"뿐이다.**
//!
//! `app_config`는 학교가 하나뿐이라는 가정이 들어간 전역 표다. 학교 단위 값
//! (최대 교시, 서류 제출 기한, 태그, 한도 규정)은 여기가 아니라 `school` 행에 있다 —
//! 지금은 학교가 하나뿐이어도 자리를 학교에 만들어야, 나중에 순회 교사가 학교를
//! 둘 등록할 때 그 값을 읽는 곳을 다시 찾지 않는다.
//!
//! 화면 취향(테마)도 여기 없다. 첫 페인트 전에 읽어야 하므로 `localStorage`에 둔다.

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
