use crate::commands::attendance::add_span_impl;
use crate::commands::check::get_pending_impl;
use crate::commands::export::*;
use crate::tests::*;

#[test]
fn cell_is_quoted_only_when_needed() {
    assert_eq!(csv_cell("김철수"), "김철수");
    assert_eq!(csv_cell("몸살, 두통"), "\"몸살, 두통\"");
    assert_eq!(csv_cell("그가 \"아프다\""), "\"그가 \"\"아프다\"\"\"");
    assert_eq!(csv_cell("두\n줄"), "\"두\n줄\"");
}

#[test]
fn csv_starts_with_bom_so_excel_reads_korean() {
    let csv = build_pending_csv(&[]);
    assert!(csv.starts_with('\u{feff}'));
}

#[test]
fn pending_csv_leads_with_the_columns_the_sms_system_needs() {
    // 외부 문자 시스템이 학년/반/번호로 수신자를 식별한다.
    let csv = build_pending_csv(&[]);
    let header = csv.trim_start_matches('\u{feff}').lines().next().unwrap();
    assert!(header.starts_with("학년,반,번호,성명"));
}

#[test]
fn pending_csv_renders_a_row() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    conn.execute(
        "UPDATE student SET guardian_phone = '010-0000-0000' WHERE id = ?1",
        rusqlite::params![sid],
    )
    .unwrap();
    add_span_impl(&conn, sid, "2026-08-26", code_id(&conn, "질병결석"), None, None, Some("몸살"))
        .unwrap();

    let rows = get_pending_impl(&conn, year, 3, 6, "2026-09-05").unwrap();
    let csv = build_pending_csv(&rows);

    assert!(csv.contains("3,6,2,이영희"));
    assert!(csv.contains("2026.08.26.(수)"));
    assert!(csv.contains("3일 경과"));
    assert!(csv.contains("010-0000-0000"));
}

#[test]
fn missing_phone_leaves_the_cell_empty_but_keeps_the_row() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_span_impl(&conn, sid, "2026-08-26", code_id(&conn, "질병결석"), None, None, None).unwrap();

    let rows = get_pending_impl(&conn, year, 3, 6, "2026-09-05").unwrap();
    let csv = build_pending_csv(&rows);
    assert!(csv.contains("3,6,2,이영희"));
    assert!(csv.trim_end().ends_with(','));
}

#[test]
fn future_due_is_reported_as_days_remaining() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_span_impl(&conn, sid, "2026-08-26", code_id(&conn, "질병결석"), None, None, None).unwrap();

    let rows = get_pending_impl(&conn, year, 3, 6, "2026-08-31").unwrap();
    let csv = build_pending_csv(&rows);
    assert!(csv.contains("2일 남음"));
}

#[test]
fn backup_csv_keeps_open_ends_as_asterisks() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_span_impl(
        &conn,
        sid,
        "2026-08-26",
        code_id(&conn, "질병조퇴"),
        Some("5"),
        None,
        Some("복통"),
    )
    .unwrap();

    let csv = build_backup_csv_impl(&conn, year, 3, 6).unwrap();
    assert!(csv.contains("2,이영희,2026.08.26.(수),질병조퇴,5,*,복통,복통으로 5교시부터 질병조퇴"));
}
