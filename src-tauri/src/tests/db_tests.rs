//! DB 파일과 트랜잭션의 기본 동작.
//!
//! 여기 있는 것은 커맨드가 아니라 **바닥 규칙**이다. 트랜잭션이 열린 채 남지 않는지,
//! 외래키가 실제로 걸려 있는지, 전출을 삭제로 처리하면 무엇이 함께 사라지는지.

use crate::db::{with_transaction, SCHEMA_VERSION};
use crate::tests::*;
use rusqlite::Connection;

/// 학생 한 명에게 구간 하나를 직접 넣는다.
///
/// 커맨드를 부르지 않고 SQL로 넣는 이유는, 이 파일이 검사하는 것이 커맨드의
/// 업무 규칙이 아니라 **DB 자체의 동작**이기 때문이다.
fn insert_span(conn: &Connection, class_id: i64, student_id: i64, date: &str) -> i64 {
    conn.execute(
        "INSERT INTO absence_span (class_id, student_id, date, memo) VALUES (?1, ?2, ?3, '')",
        rusqlite::params![class_id, student_id, date],
    )
    .unwrap();
    conn.last_insert_rowid()
}

// ── 트랜잭션 ──────────────────────────────────────────────────

#[test]
fn transaction_rolls_back_on_error() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);

    let result: Result<(), String> = with_transaction(&conn, || {
        conn.execute(
            "INSERT INTO student (school_id, year_id, grade, class_no, number, name, enrolled_from)
             VALUES (?1, ?2, 3, 6, 1, '김철수', '2026-03-02')",
            rusqlite::params![school, year],
        )
        .map_err(|e| e.to_string())?;
        Err("실패".to_string())
    });

    assert!(result.is_err());
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM student", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

#[test]
fn transaction_does_not_stay_open_after_failure() {
    // 열린 채 남으면 이후 모든 BEGIN이 실패하고, 앱을 닫을 때 작업이 통째로 롤백된다.
    let conn = setup_test_db();
    let _: Result<(), String> = with_transaction(&conn, || Err("실패".to_string()));

    let second: Result<i64, String> = with_transaction(&conn, || Ok(1));
    assert_eq!(second.unwrap(), 1);
}

// ── 버전 ──────────────────────────────────────────────────────

#[test]
fn migration_list_matches_schema_version() {
    assert_eq!(
        crate::db::MIGRATIONS.len() as u32,
        SCHEMA_VERSION,
        "SCHEMA_VERSION을 올렸으면 MIGRATIONS에도 항목을 추가해야 한다."
    );
}

// ── 외래키 ────────────────────────────────────────────────────

#[test]
fn foreign_keys_are_enforced() {
    let conn = setup_test_db();
    let bad = conn.execute(
        "INSERT INTO absence_span (student_id, date) VALUES (9999, '2026-08-26')",
        [],
    );
    assert!(bad.is_err(), "없는 학생에게 구간이 붙었다");
}

#[test]
fn a_student_belongs_to_a_school() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let bad = conn.execute(
        "INSERT INTO student (school_id, year_id, grade, class_no, number, name, enrolled_from)
         VALUES (9999, ?1, 3, 6, 1, '김철수', '2026-03-02')",
        rusqlite::params![year],
    );
    assert!(bad.is_err(), "없는 학교에 학생이 들어갔다");
}

#[test]
fn deleting_a_student_takes_its_records_with_it() {
    // 그래서 전출을 삭제로 처리하지 않는다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let sid = enroll(&conn, class, year, 2, "이영희");
    insert_span(&conn, class, sid, "2026-08-26");

    conn.execute("DELETE FROM student WHERE id = ?1", rusqlite::params![sid])
        .unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

// ── 미완성 기록이 정상 상태다 ─────────────────────────────────

#[test]
fn a_span_can_be_saved_with_neither_axis_decided() {
    // 학생이 안 왔는데 연락이 닿지 않으면 두 축이 다 빈 채로 남는다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let sid = enroll(&conn, class, year, 2, "이영희");
    let id = insert_span(&conn, class, sid, "2026-08-26");

    let (reason, r#type): (Option<i64>, Option<i64>) = conn
        .query_row(
            "SELECT reason_id, type_id FROM absence_span WHERE id = ?1",
            rusqlite::params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(reason, None);
    assert_eq!(r#type, None);
}

// ── 번호 ──────────────────────────────────────────────────────

#[test]
fn active_number_is_unique_but_withdrawn_numbers_are_reusable() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    insert_student(&conn, year, 1, "김철수");

    let dup = conn.execute(
        "INSERT INTO student (school_id, year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, ?2, 3, 6, 1, '다른사람', '2026-03-02')",
        rusqlite::params![school, year],
    );
    assert!(dup.is_err());

    conn.execute(
        "UPDATE student SET enrolled_to = '2026-06-30' WHERE number = 1",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO student (school_id, year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, ?2, 3, 6, 1, '전입생', '2026-07-01')",
        rusqlite::params![school, year],
    )
    .unwrap();
}

#[test]
fn the_same_number_can_exist_in_two_schools() {
    // 순회 교사의 3학년 6반 1번은 학교마다 다른 학생이다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");

    conn.execute(
        "INSERT INTO school (name, max_slot, due_days, due_skip_offdays, sort_order, active)
         VALUES ('옆 학교', 6, 5, 1, 20, 1)",
        [],
    )
    .unwrap();
    let other = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO student (school_id, year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, ?2, 3, 6, 1, '최지훈', '2026-03-02')",
        rusqlite::params![other, year],
    )
    .unwrap();

    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM student WHERE number = 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 2);
}
