//! DB 파일의 생명주기. 이 앱은 단일 사용자·단일 파일이므로 "프로젝트 열기"가 없다.
//! 앱 데이터 폴더의 `rollcall.db` 하나를 앱이 직접 연다.

use crate::db::{self, OpenError};
use crate::state::{DbPathState, DbState};
use chrono::Local;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

const DB_FILE: &str = "rollcall.db";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStatus {
    pub path: String,
    /// 이번 실행에서 새로 만들어진 파일인지. true면 온보딩으로 보낸다.
    pub created: bool,
    pub db_version: u32,
    pub app_version: u32,
    /// 마이그레이션이 필요한지. true면 `migrate_schema`를 부른다.
    pub needs_migration: bool,
}

fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("앱 데이터 폴더를 찾지 못했습니다: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("폴더를 만들지 못했습니다: {e}"))?;
    Ok(dir.join(DB_FILE))
}

/// 파일을 열 때마다 백업본을 만든다.
///
/// 마이그레이션 유무와 무관하게 매 실행마다 백업하는 것이 의도된 정책이다.
/// 앱은 백업 파일을 스캔하지도 지우지도 않는다 — 파일명만으로는 그 파일이 앱이
/// 만든 것인지 사용자가 보관 중인 것인지 구분할 수 없기 때문이다.
fn backup(path: &PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let dest = path.with_extension(format!("db.{stamp}.backup"));
    fs::copy(path, &dest).map_err(|e| format!("백업에 실패했습니다: {e}"))?;
    Ok(())
}

/// 앱 시작 시 프론트엔드가 가장 먼저 부르는 커맨드.
#[tauri::command]
pub fn init_db(
    app: AppHandle,
    db: State<DbState>,
    db_path_state: State<DbPathState>,
) -> Result<DbStatus, String> {
    let path = db_path(&app)?;
    let exists = path.exists();

    let (conn, db_version) = if exists {
        backup(&path)?;
        let version = db::file_version(&path).map_err(|e| e.to_string())?;
        let conn = db::open_existing(&path).map_err(|e| match e {
            OpenError::Db(err) => format!("데이터베이스 오류: {err}"),
            other => other.to_string(),
        })?;
        (conn, version)
    } else {
        let conn = db::create_new(&path).map_err(|e| format!("파일을 만들지 못했습니다: {e}"))?;
        (conn, db::SCHEMA_VERSION)
    };

    *db.0.lock().map_err(|e| e.to_string())? = Some(conn);
    *db_path_state.0.lock().map_err(|e| e.to_string())? = Some(path.clone());

    Ok(DbStatus {
        path: path.to_string_lossy().to_string(),
        created: !exists,
        db_version,
        app_version: db::SCHEMA_VERSION,
        needs_migration: db_version < db::SCHEMA_VERSION,
    })
}

#[tauri::command]
pub fn migrate_schema(db: State<DbState>) -> Result<u32, String> {
    let mut guard = db.0.lock().map_err(|e| e.to_string())?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| crate::state::DB_NOT_OPEN.to_string())?;
    let from: u32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    db::migrate(conn, from).map_err(|e| format!("마이그레이션에 실패했습니다: {e}"))?;
    Ok(db::SCHEMA_VERSION)
}

/// 백업 = 파일 복사 하나. 그 이상을 하지 않는 것이 요점이다.
#[tauri::command]
pub fn export_backup(
    db_path_state: State<DbPathState>,
    dest: String,
) -> Result<String, String> {
    let guard = db_path_state.0.lock().map_err(|e| e.to_string())?;
    let path = guard
        .as_ref()
        .ok_or_else(|| crate::state::DB_NOT_OPEN.to_string())?;
    fs::copy(path, &dest).map_err(|e| format!("백업에 실패했습니다: {e}"))?;
    Ok(dest)
}

#[tauri::command]
pub fn get_db_path(db_path_state: State<DbPathState>) -> Result<Option<String>, String> {
    let guard = db_path_state.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.as_ref().map(|p| p.to_string_lossy().to_string()))
}
