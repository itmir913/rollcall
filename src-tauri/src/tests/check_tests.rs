use crate::commands::attendance::{add_span_impl, bulk_apply_impl, delete_span_impl};
use crate::commands::check::*;
use crate::tests::*;

fn absent(conn: &rusqlite::Connection, sid: i64, date: &str) -> i64 {
    add_span_impl(conn, sid, date, code_id(conn, "질병결석"), None, None, Some("몸살")).unwrap()
}

#[test]
fn seeded_items_are_nationwide_only() {
    // 학교마다 다른 항목(리로스쿨 등)은 시드에 넣지 않는다.
    let conn = setup_test_db();
    let items = get_check_items_impl(&conn, true).unwrap();
    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, vec!["나이스 입력 완료", "증빙 서류 제출 완료"]);
}

#[test]
fn adding_an_item_adds_a_column_to_the_grid() {
    let conn = setup_test_db();
    create_check_item_impl(&conn, "리로스쿨 입력", Some(3), false, false, 30).unwrap();
    assert_eq!(get_check_items_impl(&conn, true).unwrap().len(), 3);
}

#[test]
fn item_name_cannot_be_blank() {
    let conn = setup_test_db();
    assert!(create_check_item_impl(&conn, "  ", None, false, false, 0).is_err());
}

#[test]
fn negative_due_days_is_rejected() {
    let conn = setup_test_db();
    assert!(create_check_item_impl(&conn, "항목", Some(-1), false, false, 0).is_err());
}

#[test]
fn deactivating_keeps_past_checks() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26");

    let neis = item_id(&conn, "나이스 입력 완료");
    deactivate_check_item_impl(&conn, neis).unwrap();

    assert_eq!(get_check_items_impl(&conn, true).unwrap().len(), 1);
    assert_eq!(get_check_items_impl(&conn, false).unwrap().len(), 2);

    let kept: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daily_check WHERE item_id = ?1",
            rusqlite::params![neis],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(kept, 1);
}

#[test]
fn item_without_due_days_has_no_due_date() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26");

    let due: Option<String> = conn
        .query_row(
            "SELECT due_date FROM daily_check WHERE student_id = ?1 AND item_id = ?2",
            rusqlite::params![sid, item_id(&conn, "나이스 입력 완료")],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(due, None);
}

#[test]
fn due_date_uses_business_days_by_default() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26"); // 수

    let due: Option<String> = conn
        .query_row(
            "SELECT due_date FROM daily_check WHERE student_id = ?1 AND item_id = ?2",
            rusqlite::params![sid, item_id(&conn, "증빙 서류 제출 완료")],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(due.as_deref(), Some("2026-09-02"));
}

#[test]
fn teacher_can_override_the_due_date() {
    // 계산이 틀린 경우의 탈출구다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26");
    let doc = item_id(&conn, "증빙 서류 제출 완료");

    set_check_due_impl(&conn, sid, "2026-08-26", doc, Some("2026-09-10")).unwrap();
    let due: Option<String> = conn
        .query_row(
            "SELECT due_date FROM daily_check WHERE student_id = ?1 AND item_id = ?2",
            rusqlite::params![sid, doc],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(due.as_deref(), Some("2026-09-10"));
}

#[test]
fn bad_override_date_is_rejected() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26");
    let doc = item_id(&conn, "증빙 서류 제출 완료");
    assert!(set_check_due_impl(&conn, sid, "2026-08-26", doc, Some("9월 10일")).is_err());
}

#[test]
fn changing_item_settings_does_not_touch_saved_due_dates() {
    // 교사가 고쳐 둔 마감일을 설정 변경이 소급해 덮으면 안 된다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26");
    let doc = item_id(&conn, "증빙 서류 제출 완료");

    update_check_item_impl(&conn, doc, "증빙 서류 제출 완료", Some(30), false, false, 20).unwrap();

    let due: Option<String> = conn
        .query_row(
            "SELECT due_date FROM daily_check WHERE student_id = ?1 AND item_id = ?2",
            rusqlite::params![sid, doc],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(due.as_deref(), Some("2026-09-02"));
}

#[test]
fn checking_without_a_span_is_an_error_not_a_silent_noop() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let err = set_check_impl(&conn, sid, "2026-08-26", item_id(&conn, "나이스 입력 완료"), true)
        .unwrap_err();
    assert!(err.contains("출결"));
}

#[test]
fn toggling_records_and_clears_the_timestamp() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26");
    let neis = item_id(&conn, "나이스 입력 완료");

    set_check_impl(&conn, sid, "2026-08-26", neis, true).unwrap();
    let done_at: Option<String> = conn
        .query_row(
            "SELECT done_at FROM daily_check WHERE student_id = ?1 AND item_id = ?2",
            rusqlite::params![sid, neis],
            |r| r.get(0),
        )
        .unwrap();
    assert!(done_at.is_some());

    set_check_impl(&conn, sid, "2026-08-26", neis, false).unwrap();
    let done_at: Option<String> = conn
        .query_row(
            "SELECT done_at FROM daily_check WHERE student_id = ?1 AND item_id = ?2",
            rusqlite::params![sid, neis],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(done_at, None);
}

#[test]
fn group_check_toggles_the_whole_period_at_once() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    let result = bulk_apply_impl(
        &conn,
        &[sid],
        &[
            "2026-08-28".to_string(),
            "2026-08-31".to_string(),
            "2026-09-01".to_string(),
        ],
        code_id(&conn, "출석인정결석"),
        None,
        None,
        Some("교외체험학습"),
    )
    .unwrap();

    let doc = item_id(&conn, "증빙 서류 제출 완료");
    let n = set_group_check_impl(&conn, &result.group_id, doc, true).unwrap();
    assert_eq!(n, 3);

    let pending = get_pending_impl(&conn, year, 3, 6, "2026-09-05").unwrap();
    assert!(pending.iter().all(|p| p.item_id != doc));
}

// ── 미제출자 ──────────────────────────────────────────────────

#[test]
fn pending_lists_every_unchecked_item() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26");

    let pending = get_pending_impl(&conn, year, 3, 6, "2026-09-05").unwrap();
    assert_eq!(pending.len(), 2);
}

#[test]
fn pending_reports_days_overdue() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26"); // 마감 2026-09-02

    let pending = get_pending_impl(&conn, year, 3, 6, "2026-09-05").unwrap();
    let doc = pending
        .iter()
        .find(|p| p.item_name == "증빙 서류 제출 완료")
        .unwrap();
    assert_eq!(doc.days_overdue, Some(3));

    let neis = pending
        .iter()
        .find(|p| p.item_name == "나이스 입력 완료")
        .unwrap();
    assert_eq!(neis.days_overdue, None); // 마감이 없다
}

#[test]
fn pending_puts_dated_items_before_undated_ones() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26");

    let pending = get_pending_impl(&conn, year, 3, 6, "2026-09-05").unwrap();
    assert!(pending[0].due_date.is_some());
    assert!(pending.last().unwrap().due_date.is_none());
}

#[test]
fn completed_items_leave_the_list() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26");
    set_check_impl(&conn, sid, "2026-08-26", item_id(&conn, "나이스 입력 완료"), true).unwrap();

    assert_eq!(get_pending_impl(&conn, year, 3, 6, "2026-09-05").unwrap().len(), 1);
}

#[test]
fn deleting_the_span_removes_it_from_pending() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let span = absent(&conn, sid, "2026-08-26");
    delete_span_impl(&conn, span).unwrap();

    assert!(get_pending_impl(&conn, year, 3, 6, "2026-09-05").unwrap().is_empty());
}

#[test]
fn summary_counts_by_item() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김철수");
    let b = insert_student(&conn, year, 2, "이영희");
    absent(&conn, a, "2026-08-26");
    absent(&conn, b, "2026-08-26");
    set_check_impl(&conn, a, "2026-08-26", item_id(&conn, "나이스 입력 완료"), true).unwrap();

    let summary = get_pending_summary_impl(&conn, year, 3, 6).unwrap();
    let neis = summary.iter().find(|s| s.item_name == "나이스 입력 완료").unwrap();
    let doc = summary.iter().find(|s| s.item_name == "증빙 서류 제출 완료").unwrap();
    assert_eq!(neis.count, 1);
    assert_eq!(doc.count, 2);
}

#[test]
fn deactivated_items_disappear_from_pending() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    absent(&conn, sid, "2026-08-26");
    deactivate_check_item_impl(&conn, item_id(&conn, "나이스 입력 완료")).unwrap();

    let pending = get_pending_impl(&conn, year, 3, 6, "2026-09-05").unwrap();
    assert_eq!(pending.len(), 1);
}
