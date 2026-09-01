use rusqlite::Connection;

pub mod attendance_tests;
pub mod axis_tests;
pub mod check_tests;
pub mod db_tests;
pub mod due_tests;
pub mod export_tests;
pub mod phrase_tests;
pub mod schema_lock_tests;
pub mod slots_tests;
pub mod student_tests;

/// 스키마 + 시드가 적용된 메모리 DB. 실제 파일과 같은 경로를 타야 하므로
/// 시드도 함께 넣는다 — 코드 목록이 비어 있으면 대부분의 테스트가 무의미해진다.
pub fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(include_str!("../schema.sql")).unwrap();
    conn.execute_batch(include_str!("../seed.sql")).unwrap();
    conn
}

pub fn insert_year(conn: &Connection, year: i64) -> i64 {
    conn.execute(
        "INSERT INTO academic_year (year, starts_on, ends_on) VALUES (?1, ?2, ?3)",
        rusqlite::params![year, format!("{year}-03-01"), format!("{}-02-28", year + 1)],
    )
    .unwrap();
    conn.last_insert_rowid()
}

pub fn insert_student(conn: &Connection, year_id: i64, number: i64, name: &str) -> i64 {
    conn.execute(
        "INSERT INTO student (year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, 3, 6, ?2, ?3, '2026-03-02')",
        rusqlite::params![year_id, number, name],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 시드에서 라벨로 구분 id를 찾는다. (질병 / 출석인정 / 미인정 / 기타)
pub fn reason_id(conn: &Connection, label: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM attendance_reason WHERE label = ?1 AND valid_to IS NULL",
        rusqlite::params![label],
        |r| r.get(0),
    )
    .unwrap()
}

/// 시드에서 라벨로 종류 id를 찾는다. (결석 / 지각 / 조퇴 / 결과)
pub fn type_id(conn: &Connection, label: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM attendance_type WHERE label = ?1 AND valid_to IS NULL",
        rusqlite::params![label],
        |r| r.get(0),
    )
    .unwrap()
}

/// 두 축을 한 번에. `axes(&conn, "질병", "결석")`
pub fn axes(conn: &Connection, reason: &str, r#type: &str) -> (Option<i64>, Option<i64>) {
    (Some(reason_id(conn, reason)), Some(type_id(conn, r#type)))
}

pub fn item_id(conn: &Connection, name: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM check_item WHERE name = ?1",
        rusqlite::params![name],
        |r| r.get(0),
    )
    .unwrap()
}
