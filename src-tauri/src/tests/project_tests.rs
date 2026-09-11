//! DB 파일의 생명주기와 파일 쓰기.
//!
//! 여기 있는 것은 업무 규칙이 아니라 **파일을 여닫는 순간**이다. 첫 실행인지,
//! 백업을 떴는지, 상위 버전 파일을 거부하는지, 없는 폴더에 쓰려 할 때 무엇을
//! 돌려주는지 — 교사가 앱을 켜자마자 지나가는 길이라 여기서 틀리면 화면이 아예 뜨지 않는다.
//!
//! 커맨드가 아니라 `*_impl`을 부른다. `State<DbState>`나 `AppHandle`을 받는 함수는
//! 테스트에서 호출할 수 없기 때문이다.

use crate::commands::file::write_bytes_file_impl;
use crate::commands::project::{
    export_backup_impl, get_db_path_impl, init_db_impl, migrate_schema_impl,
};
use crate::db::SCHEMA_VERSION;
use crate::tests::setup_test_db;
use base64::Engine;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

// ── 재료 ──────────────────────────────────────────────────────

/// 테스트마다 겹치지 않는 빈 폴더. 테스트는 병렬로 돌기 때문에 이름을 고정할 수 없다.
fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rollcall-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 폴더를 지운다. Windows는 열려 있는 파일을 지우지 못하므로, 부르기 전에 커넥션을
/// 먼저 해제해야 한다. 실패해도 테스트를 깨지 않는다 — 임시 폴더가 남는 것뿐이다.
fn clean(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

fn user_version(conn: &Connection) -> u32 {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

/// DB에 실제로 만들어진 객체 목록. 마이그레이션이 구조를 건드렸는지만 본다.
fn objects(conn: &Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare(
            "SELECT type || '|' || name FROM sqlite_master
             WHERE name NOT LIKE 'sqlite_%' ORDER BY type, name",
        )
        .unwrap();
    let rows = stmt.query_map([], |r| r.get(0)).unwrap();
    rows.collect::<Result<Vec<String>, _>>().unwrap()
}

/// 시드가 들어갔는지 확인하는 값. 비어 있으면 첫 화면에서 구분을 선택할 수 없다.
fn reason_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM attendance_reason", [], |r| r.get(0))
        .unwrap()
}

// ── 마이그레이션 ──────────────────────────────────────────────

#[test]
fn migrating_a_current_file_changes_nothing() {
    // SCHEMA_VERSION이 1에 고정된 동안 마이그레이션은 할 일이 없다.
    // 이 테스트가 깨진다는 것은 배포된 구조가 움직였다는 뜻이다.
    let mut conn = setup_test_db();
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)
        .unwrap();
    let before = objects(&conn);
    let reasons = reason_count(&conn);

    let to = migrate_schema_impl(&mut conn).unwrap();

    assert_eq!(to, SCHEMA_VERSION);
    assert_eq!(user_version(&conn), SCHEMA_VERSION);
    assert_eq!(objects(&conn), before, "마이그레이션이 구조를 건드렸다");
    assert_eq!(reason_count(&conn), reasons, "시드가 함께 지워졌다");
}

#[test]
fn foreign_keys_come_back_on_after_a_migration() {
    // db::migrate는 외래키를 껐다가 다시 켠다. 꺼진 채 남으면 그 세션의 모든 쓰기가
    // 무결성 검사를 건너뛰고, 그 사실이 앱을 다시 켤 때까지 드러나지 않는다.
    let mut conn = setup_test_db();
    migrate_schema_impl(&mut conn).unwrap();

    let on: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(on, 1, "마이그레이션 뒤 외래키가 꺼진 채 남았다");
}

// ── 파일 열기 ─────────────────────────────────────────────────

#[test]
fn a_new_file_is_a_first_run_and_needs_no_migration() {
    let dir = temp_dir();
    let path = dir.join("rollcall.db");

    let (conn, status) = init_db_impl(&path).unwrap();

    assert!(status.created, "새로 만든 파일이 첫 실행으로 잡히지 않았다");
    assert!(!status.needs_migration);
    assert_eq!(status.db_version, SCHEMA_VERSION);
    assert_eq!(status.app_version, SCHEMA_VERSION);
    assert!(reason_count(&conn) > 0, "시드 없이 파일만 만들어졌다");

    drop(conn);
    clean(&dir);
}

#[test]
fn reopening_the_same_file_is_not_a_first_run() {
    // created가 계속 true면 매번 첫 실행 화면으로 보내진다.
    let dir = temp_dir();
    let path = dir.join("rollcall.db");
    let (first, _) = init_db_impl(&path).unwrap();
    drop(first);

    let (second, status) = init_db_impl(&path).unwrap();
    drop(second);

    assert!(!status.created, "있던 파일을 새로 만든 것으로 봤다");
    assert_eq!(status.db_version, SCHEMA_VERSION);
    clean(&dir);
}

#[test]
fn opening_an_existing_file_leaves_a_backup() {
    // 마이그레이션 유무와 무관하게 매 실행마다 백업하는 것이 의도된 정책이다.
    let dir = temp_dir();
    let path = dir.join("rollcall.db");
    let (first, _) = init_db_impl(&path).unwrap();
    drop(first);

    let (second, _) = init_db_impl(&path).unwrap();
    drop(second);

    let backups = fs::read_dir(&dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".backup")
        })
        .count();
    assert_eq!(backups, 1, "기존 파일을 열면서 백업을 남기지 않았다");
    clean(&dir);
}

#[test]
fn a_file_from_a_newer_app_is_refused_in_korean() {
    // 상위 버전 파일을 그대로 열면 앱이 모르는 구조에 쓰게 된다.
    let dir = temp_dir();
    let path = dir.join("rollcall.db");
    let (conn, _) = init_db_impl(&path).unwrap();
    conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
        .unwrap();
    drop(conn);

    let err = init_db_impl(&path).unwrap_err();

    assert!(err.contains("앱을 업데이트"), "영문 원문이 그대로 나갔다: {err}");
    clean(&dir);
}

// ── 백업 파일 저장 ─────────────────────────────────────────────

#[test]
fn a_backup_is_a_copy_that_opens_on_its_own() {
    let dir = temp_dir();
    let path = dir.join("rollcall.db");
    let (conn, _) = init_db_impl(&path).unwrap();
    drop(conn);
    let dest = dir.join("보관본.db");

    let out = export_backup_impl(&path, &dest.to_string_lossy()).unwrap();

    assert_eq!(out, dest.to_string_lossy());
    let copied = Connection::open(&dest).unwrap();
    assert!(reason_count(&copied) > 0, "복사본이 비어 있다");
    assert_eq!(user_version(&copied), SCHEMA_VERSION);

    drop(copied);
    clean(&dir);
}

#[test]
fn a_backup_to_a_folder_that_does_not_exist_is_refused() {
    let dir = temp_dir();
    let path = dir.join("rollcall.db");
    let (conn, _) = init_db_impl(&path).unwrap();
    drop(conn);
    let dest = dir.join("없는 폴더").join("보관본.db");

    let err = export_backup_impl(&path, &dest.to_string_lossy()).unwrap_err();

    assert!(err.contains("백업에 실패했습니다"), "{err}");
    clean(&dir);
}

// ── 열려 있는 파일의 경로 ─────────────────────────────────────

#[test]
fn the_path_is_none_before_a_file_is_opened() {
    // 오류가 아니라 None이다. 설정 화면은 첫 실행에서도 열린다.
    assert_eq!(get_db_path_impl(None), None);
    assert_eq!(
        get_db_path_impl(Some(Path::new("rollcall.db"))),
        Some("rollcall.db".to_string())
    );
}

// ── 바이트를 파일로 ───────────────────────────────────────────

#[test]
fn base64_goes_in_and_the_same_bytes_come_out() {
    // 샘플 명렬표는 브라우저 쪽 exceljs가 만든 xlsx다. 한 바이트라도 어긋나면
    // 엑셀이 열지 못한다.
    let dir = temp_dir();
    let path = dir.join("샘플 명렬표.xlsx");
    let bytes: Vec<u8> = vec![0x50, 0x4b, 0x03, 0x04, 0x00, 0xff];
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

    write_bytes_file_impl(&path.to_string_lossy(), &encoded).unwrap();

    assert_eq!(fs::read(&path).unwrap(), bytes);
    clean(&dir);
}

#[test]
fn a_folder_that_does_not_exist_is_refused_instead_of_being_created() {
    // 다이얼로그를 거쳤어도 그 사이에 USB가 빠질 수 있다. 폴더를 대신 만들면
    // 교사가 선택한 적 없는 자리에 파일이 생긴다.
    let dir = temp_dir();
    let missing = dir.join("없는 폴더");
    let path = missing.join("샘플 명렬표.xlsx");

    let err = write_bytes_file_impl(&path.to_string_lossy(), "").unwrap_err();

    assert!(err.contains("폴더가 존재하지 않습니다"), "{err}");
    assert!(!missing.exists(), "거부해 놓고 폴더를 만들었다");
    clean(&dir);
}

#[test]
fn broken_base64_is_reported_and_no_file_is_left() {
    let dir = temp_dir();
    let path = dir.join("샘플 명렬표.xlsx");

    let err = write_bytes_file_impl(&path.to_string_lossy(), "이것은 base64가 아니다").unwrap_err();

    assert!(err.contains("해독하지 못했습니다"), "{err}");
    assert!(!path.exists(), "해독에 실패했는데 빈 파일이 남았다");
    clean(&dir);
}
