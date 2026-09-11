//! 개요 요약 테스트.
//!
//! 두 가지를 집중해서 본다.
//!   · `recorded`가 **학생 수**인가. 구간 수로 세면 하루 2구간인 학생이 두 명이 된다.
//!   · `doc_pending`이 마감과 무관하게 **전부**인가. 마감이 지난 것만 세면 여유가
//!     있을 때 서류를 받으라는 배지의 뜻이 사라진다.

use crate::commands::home::{enrolled_on_impl, get_home_summary_impl};
use crate::tests::*;
use rusqlite::Connection;

const TODAY: &str = "2026-09-10";

/// 학급을 지정해 학생을 넣는다. 다른 반이 섞여 들어오지 않는지 볼 때 쓴다.
fn insert_student_in(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
    number: i64,
    name: &str,
) -> i64 {
    let school = school_id(conn);
    conn.execute(
        "INSERT INTO student (school_id, year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, '2026-03-02')",
        rusqlite::params![school, year_id, grade, class_no, number, name],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn insert_span(
    conn: &Connection,
    student_id: i64,
    date: &str,
    axes: (Option<i64>, Option<i64>),
) -> i64 {
    conn.execute(
        "INSERT INTO absence_span (student_id, date, reason_id, type_id, memo)
         VALUES (?1, ?2, ?3, ?4, '')",
        rusqlite::params![student_id, date, axes.0, axes.1],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn set_doc(conn: &Connection, span_id: i64, done: bool, due: Option<&str>) {
    conn.execute(
        "UPDATE absence_span SET doc_done = ?1, doc_due = ?2 WHERE id = ?3",
        rusqlite::params![done as i64, due, span_id],
    )
    .unwrap();
}

fn set_neis(conn: &Connection, span_id: i64, done: bool) {
    conn.execute(
        "UPDATE absence_span SET neis_done = ?1 WHERE id = ?2",
        rusqlite::params![done as i64, span_id],
    )
    .unwrap();
}

/// 학교 · 학년도 · 학생 셋을 갖춘 3학년 6반.
fn fixture() -> (Connection, i64, i64) {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    (conn, school, year)
}

// ── 재학 인원 ─────────────────────────────────────────────────

#[test]
fn enrolled_counts_the_class_on_that_day() {
    let (conn, school, year) = fixture();
    insert_student(&conn, year, 1, "김민준");
    insert_student(&conn, year, 2, "이서연");
    insert_student_in(&conn, year, 3, 7, 1, "다른 반 학생");

    assert_eq!(
        enrolled_on_impl(&conn, school, year, 3, 6, TODAY).unwrap(),
        2
    );
}

#[test]
fn a_withdrawn_student_still_counts_before_the_transfer() {
    let (conn, school, year) = fixture();
    let leaving = insert_student(&conn, year, 1, "김민준");
    conn.execute(
        "UPDATE student SET enrolled_to = '2026-09-15' WHERE id = ?1",
        rusqlite::params![leaving],
    )
    .unwrap();

    assert_eq!(
        enrolled_on_impl(&conn, school, year, 3, 6, TODAY).unwrap(),
        1
    );
    assert_eq!(
        enrolled_on_impl(&conn, school, year, 3, 6, "2026-09-20").unwrap(),
        0
    );
}

// ── 오늘치 ────────────────────────────────────────────────────

#[test]
fn recorded_counts_students_not_spans() {
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    let b = insert_student(&conn, year, 2, "이서연");
    insert_student(&conn, year, 3, "박지호");

    let axes = axes(&conn, "질병", "조퇴");
    // 하루 2구간은 정상 입력이다. 두 명으로 세면 인원보다 많아진다.
    insert_span(&conn, a, TODAY, axes);
    insert_span(&conn, a, TODAY, axes);
    insert_span(&conn, b, TODAY, axes);

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    assert_eq!(summary.enrolled, 3);
    assert_eq!(summary.recorded, 2);
}

#[test]
fn only_the_given_day_is_counted_as_recorded() {
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");
    insert_span(&conn, a, "2026-09-09", axes);

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    assert_eq!(summary.recorded, 0);
}

#[test]
fn incomplete_counts_spans_with_either_axis_missing() {
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    let reason = Some(reason_id(&conn, "질병"));
    let kind = Some(type_id(&conn, "결석"));

    insert_span(&conn, a, TODAY, (reason, kind)); // 완성
    insert_span(&conn, a, TODAY, (reason, None)); // 종류 미정
    insert_span(&conn, a, TODAY, (None, kind)); // 구분 미정
    insert_span(&conn, a, TODAY, (None, None)); // 둘 다 미정

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    assert_eq!(summary.recorded, 1);
    assert_eq!(summary.incomplete, 3);
}

// ── 밀린 일 ───────────────────────────────────────────────────

#[test]
fn doc_pending_counts_everything_not_yet_received() {
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");

    let overdue = insert_span(&conn, a, "2026-08-20", axes);
    set_doc(&conn, overdue, false, Some("2026-08-27"));
    let ahead = insert_span(&conn, a, "2026-09-09", axes);
    set_doc(&conn, ahead, false, Some("2026-12-31"));
    let no_due = insert_span(&conn, a, "2026-09-09", axes);
    set_doc(&conn, no_due, false, None);
    let received = insert_span(&conn, a, "2026-09-09", axes);
    set_doc(&conn, received, true, Some("2026-09-16"));

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    // 마감이 지난 것만 세지 않는다 — 아직 안 받은 셋 전부다.
    assert_eq!(summary.doc_pending, 3);
    assert_eq!(summary.doc_overdue, 1);
}

#[test]
fn a_due_date_that_falls_on_the_given_day_is_not_overdue_yet() {
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");
    let today_due = insert_span(&conn, a, "2026-09-03", axes);
    set_doc(&conn, today_due, false, Some(TODAY));

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    assert_eq!(summary.doc_pending, 1);
    assert_eq!(summary.doc_overdue, 0);
}

#[test]
fn neis_pending_counts_records_not_yet_entered() {
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");

    insert_span(&conn, a, "2026-09-03", axes);
    insert_span(&conn, a, "2026-09-09", axes);
    let done = insert_span(&conn, a, "2026-09-09", axes);
    set_neis(&conn, done, true);

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    assert_eq!(summary.neis_pending, 2);
}

#[test]
fn neis_rows_start_from_the_oldest_day() {
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");
    insert_span(&conn, a, "2026-09-08", axes);
    insert_span(&conn, a, "2026-09-01", axes);
    insert_span(&conn, a, "2026-09-04", axes);

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    let dates: Vec<String> = summary.neis_rows.iter().map(|s| s.date.clone()).collect();
    assert_eq!(dates, ["2026-09-01", "2026-09-04", "2026-09-08"]);
}

#[test]
fn doc_rows_start_from_the_most_overdue() {
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");

    let mild = insert_span(&conn, a, "2026-09-01", axes);
    set_doc(&conn, mild, false, Some("2026-09-08"));
    let worst = insert_span(&conn, a, "2026-08-01", axes);
    set_doc(&conn, worst, false, Some("2026-08-08"));

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    assert_eq!(summary.doc_rows[0].id, worst);
    assert_eq!(summary.doc_rows[1].id, mild);
}

// ── limit ─────────────────────────────────────────────────────

#[test]
fn limit_caps_rows_but_never_the_counts() {
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");
    for day in ["2026-09-01", "2026-09-02", "2026-09-03"] {
        insert_span(&conn, a, day, axes);
    }

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 1).unwrap();
    assert_eq!(summary.doc_pending, 3);
    assert_eq!(summary.neis_pending, 3);
    assert_eq!(summary.doc_rows.len(), 1);
    assert_eq!(summary.neis_rows.len(), 1);
}

#[test]
fn a_limit_of_zero_or_less_returns_no_rows() {
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");
    insert_span(&conn, a, "2026-09-01", axes);

    for limit in [0, -3] {
        let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, limit).unwrap();
        assert_eq!(summary.doc_pending, 1);
        assert!(summary.doc_rows.is_empty());
        assert!(summary.neis_rows.is_empty());
    }
}

// ── 경계 ──────────────────────────────────────────────────────

#[test]
fn another_class_never_leaks_in() {
    let (conn, school, year) = fixture();
    let mine = insert_student(&conn, year, 1, "김민준");
    let theirs = insert_student_in(&conn, year, 3, 7, 1, "옆 반 학생");
    let axes = axes(&conn, "질병", "결석");
    insert_span(&conn, mine, TODAY, axes);
    insert_span(&conn, theirs, TODAY, axes);

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    assert_eq!(summary.enrolled, 1);
    assert_eq!(summary.recorded, 1);
    assert_eq!(summary.doc_pending, 1);
    assert_eq!(summary.neis_pending, 1);
}

#[test]
fn an_empty_class_summarizes_to_zero() {
    let (conn, school, year) = fixture();
    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    assert_eq!(summary.enrolled, 0);
    assert_eq!(summary.recorded, 0);
    assert_eq!(summary.incomplete, 0);
    assert_eq!(summary.doc_pending, 0);
    assert_eq!(summary.doc_overdue, 0);
    assert_eq!(summary.neis_pending, 0);
    assert!(summary.doc_rows.is_empty());
    assert!(summary.neis_rows.is_empty());
}

#[test]
fn the_date_comes_back_in_both_forms() {
    let (conn, school, year) = fixture();
    let summary = get_home_summary_impl(&conn, school, year, 3, 6, TODAY, 10).unwrap();
    assert_eq!(summary.date, TODAY);
    assert!(
        summary.date_label.starts_with("2026.09.10."),
        "{}",
        summary.date_label
    );
}

#[test]
fn a_broken_date_is_rejected() {
    let (conn, school, year) = fixture();
    let err = get_home_summary_impl(&conn, school, year, 3, 6, "어제", 10).unwrap_err();
    assert!(err.contains("어제"), "{err}");
}

#[test]
fn a_loosely_written_date_comes_back_in_iso() {
    // chrono는 `2026-9-10`도 받아들인다. 저장된 값은 언제나 자리를 채운 형식이므로
    // 그대로 질의하면 아무것도 걸리지 않는다.
    let (conn, school, year) = fixture();
    let a = insert_student(&conn, year, 1, "김민준");
    insert_span(&conn, a, TODAY, axes(&conn, "질병", "결석"));

    let summary = get_home_summary_impl(&conn, school, year, 3, 6, "2026-9-10", 10).unwrap();
    assert_eq!(summary.date, TODAY);
    assert_eq!(summary.recorded, 1);
}

#[test]
fn a_missing_year_summarizes_to_zero_rather_than_failing() {
    // 없는 학년도 id는 오류가 아니다. 화면이 아직 학년도를 고르지 않은 상태에서
    // 개요가 통째로 실패하면 교사가 볼 것이 없다.
    let (conn, school, _) = fixture();
    let summary = get_home_summary_impl(&conn, school, 9999, 3, 6, TODAY, 10).unwrap();
    assert_eq!(summary.enrolled, 0);
    assert_eq!(summary.doc_pending, 0);
}
