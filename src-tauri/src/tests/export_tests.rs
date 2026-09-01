use crate::commands::attendance::add_spans_impl;
use crate::commands::check::get_pending_impl;
use crate::commands::export::*;
use crate::commands::student::set_contacts_impl;
use crate::tests::*;
use crate::types::ContactItem;

fn contact(label: &str, value: &str) -> ContactItem {
    ContactItem {
        id: 0,
        label: label.into(),
        value: value.into(),
        note: None,
        sort_order: 0,
    }
}

#[test]
fn cell_is_quoted_only_when_needed() {
    assert_eq!(csv_cell("김철수"), "김철수");
    assert_eq!(csv_cell("몸살, 두통"), "\"몸살, 두통\"");
    assert_eq!(csv_cell("그가 \"아프다\""), "\"그가 \"\"아프다\"\"\"");
    assert_eq!(csv_cell("두\n줄"), "\"두\n줄\"");
}

#[test]
fn csv_starts_with_bom_so_excel_reads_korean() {
    assert!(build_pending_csv(&[]).starts_with('\u{feff}'));
}

#[test]
fn pending_csv_leads_with_the_columns_the_sms_system_needs() {
    // 외부 문자 시스템이 학년/반/번호로 수신자를 식별한다.
    let csv = build_pending_csv(&[]);
    let header = csv.trim_start_matches('\u{feff}').lines().next().unwrap();
    assert!(header.starts_with("학년,반,번호,성명"));
}

#[test]
fn pending_csv_renders_a_row_with_the_primary_contact() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");

    set_contacts_impl(
        &conn,
        sid,
        &[contact("어머니", "010-0000-0000"), contact("학생", "010-1111-1111")],
    )
    .unwrap();
    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, None, None, Some("몸살")).unwrap();

    let rows = get_pending_impl(&conn, year, 3, 6, "2026-09-05").unwrap();
    let csv = build_pending_csv(&rows);

    assert!(csv.contains("3,6,2,이영희"));
    assert!(csv.contains("2026.08.26.(수)"));
    assert!(csv.contains("3일 경과"));
    // 여러 개여도 첫 번째만 나간다.
    assert!(csv.contains("어머니,010-0000-0000"));
    assert!(!csv.contains("010-1111-1111"));
}

#[test]
fn a_student_without_contacts_still_appears() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");
    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, None, None, None).unwrap();

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
    let (r, t) = axes(&conn, "질병", "결석");
    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, None, None, None).unwrap();

    let rows = get_pending_impl(&conn, year, 3, 6, "2026-08-31").unwrap();
    assert!(build_pending_csv(&rows).contains("2일 남음"));
}

#[test]
fn backup_csv_keeps_open_ends_as_asterisks() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "조퇴");
    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, Some("5"), None, Some("복통")).unwrap();

    let csv = build_backup_csv_impl(&conn, year, 3, 6).unwrap();
    assert!(csv.contains("2,이영희,2026.08.26.(수),질병,조퇴,5,*,복통,복통으로 5교시부터 질병조퇴"));
}

#[test]
fn backup_csv_marks_undecided_axes_rather_than_dropping_the_row() {
    // 아직 못 정한 기록도 백업에 들어가야 한다. 그것이 지금 남은 일이기 때문이다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_spans_impl(&conn, &[sid], "2026-08-26", None, None, None, None, None).unwrap();

    let csv = build_backup_csv_impl(&conn, year, 3, 6).unwrap();
    assert!(csv.contains("2,이영희,2026.08.26.(수),미정,미정,*,*"));
}
