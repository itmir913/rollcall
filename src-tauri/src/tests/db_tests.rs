use crate::db::{with_transaction, SCHEMA_VERSION};
use crate::tests::*;

#[test]
fn transaction_rolls_back_on_error() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);

    let result: Result<(), String> = with_transaction(&conn, || {
        conn.execute(
            "INSERT INTO student (year_id, grade, class_no, number, name, enrolled_from)
             VALUES (?1, 3, 6, 1, '김철수', '2026-03-02')",
            rusqlite::params![year],
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

#[test]
fn migration_list_matches_schema_version() {
    assert_eq!(
        crate::db::MIGRATIONS.len() as u32,
        SCHEMA_VERSION,
        "SCHEMA_VERSION을 올렸으면 MIGRATIONS에도 항목을 추가해야 한다."
    );
}

#[test]
fn foreign_keys_are_enforced() {
    let conn = setup_test_db();
    let err = conn.execute(
        "INSERT INTO absence_span (student_id, date, reason_id) VALUES (9999, '2026-08-26', 1)",
        [],
    );
    assert!(err.is_err());
}

#[test]
fn deleting_a_student_takes_its_records_with_it() {
    // 그래서 전출을 삭제로 처리하지 않는다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");
    crate::commands::attendance::add_spans_impl(
        &conn,
        &[sid],
        "2026-08-26",
        r,
        t,
        None,
        None,
        None,
    )
    .unwrap();

    conn.execute("DELETE FROM student WHERE id = ?1", rusqlite::params![sid])
        .unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

#[test]
fn active_number_is_unique_but_withdrawn_numbers_are_reusable() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");

    let dup = conn.execute(
        "INSERT INTO student (year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, 3, 6, 1, '다른사람', '2026-03-02')",
        rusqlite::params![year],
    );
    assert!(dup.is_err());

    conn.execute(
        "UPDATE student SET enrolled_to = '2026-06-30' WHERE number = 1",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO student (year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, 3, 6, 1, '전입생', '2026-07-01')",
        rusqlite::params![year],
    )
    .unwrap();
}
