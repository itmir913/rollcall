//! 개요 요약 테스트.
//!
//! 두 가지를 집중해서 본다.
//!   · `recorded`가 **학생 수**인가. 구간 수로 세면 하루 2구간인 학생이 두 명이 된다.
//!   · `doc_pending`이 마감과 무관하게 **전부**인가. 마감이 지난 것만 세면 여유가
//!     있을 때 서류를 받으라는 배지의 뜻이 사라진다.

use crate::commands::class::member_count_on;
use crate::commands::home::get_home_summary_impl;
use crate::tests::*;
use rusqlite::Connection;

const TODAY: &str = "2026-09-10";

/// 구간은 **학급에 연결한다.** 담임 기록이 가리키는 것이 학급이다.
fn insert_span(
    conn: &Connection,
    class_id: i64,
    student_id: i64,
    date: &str,
    axes: (Option<i64>, Option<i64>),
) -> i64 {
    conn.execute(
        "INSERT INTO absence_span (class_id, student_id, date, reason_id, type_id, memo)
         VALUES (?1, ?2, ?3, ?4, ?5, '')",
        rusqlite::params![class_id, student_id, date, axes.0, axes.1],
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

/// 학교 · 학년도 · 담임 학급 하나.
fn fixture() -> (Connection, i64, i64) {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    (conn, class, school)
}

// ── 재학 인원 ─────────────────────────────────────────────────

#[test]
fn 인원은_그날_내_명단으로_센다() {
    let (conn, class, school) = fixture();
    enroll(&conn, class, school, 1, "김민준");
    enroll(&conn, class, school, 2, "이서연");
    // 학적만 있고 내 명단에는 없는 학생. 학년 · 반으로 셌다면 섞여 들어온다.
    insert_student_at(&conn, school, 3, 7, 1, "다른 반 학생");

    assert_eq!(member_count_on(&conn, class, TODAY).unwrap(), 2);
}

#[test]
fn a_withdrawn_student_still_counts_before_the_transfer() {
    let (conn, class, school) = fixture();
    let leaving = enroll(&conn, class, school, 1, "김민준");
    conn.execute(
        "UPDATE student SET enrolled_to = '2026-09-15' WHERE id = ?1",
        rusqlite::params![leaving],
    )
    .unwrap();

    assert_eq!(member_count_on(&conn, class, TODAY).unwrap(), 1);
    assert_eq!(member_count_on(&conn, class, "2026-09-20").unwrap(), 0);
}

#[test]
fn 명단에서_빠진_날_전에는_그대로_센다() {
    // 명단에서 빼는 것은 학적을 마감하는 것과 다른 일이라 기간도 따로 닫힌다.
    let (conn, class, school) = fixture();
    let leaving = enroll(&conn, class, school, 1, "김민준");
    conn.execute(
        "UPDATE class_member SET left_on = '2026-09-15' WHERE student_id = ?1",
        rusqlite::params![leaving],
    )
    .unwrap();

    assert_eq!(member_count_on(&conn, class, TODAY).unwrap(), 1);
    assert_eq!(member_count_on(&conn, class, "2026-09-20").unwrap(), 0);
}

// ── 오늘치 ────────────────────────────────────────────────────

#[test]
fn recorded_counts_students_not_spans() {
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    let b = enroll(&conn, class, school, 2, "이서연");
    enroll(&conn, class, school, 3, "박지호");

    let axes = axes(&conn, "질병", "조퇴");
    // 하루 2구간은 정상 입력이다. 두 명으로 세면 인원보다 많아진다.
    insert_span(&conn, class, a, TODAY, axes);
    insert_span(&conn, class, a, TODAY, axes);
    insert_span(&conn, class, b, TODAY, axes);

    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
    assert_eq!(summary.enrolled, 3);
    assert_eq!(summary.recorded, 2);
}

#[test]
fn only_the_given_day_is_counted_as_recorded() {
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");
    insert_span(&conn, class, a, "2026-09-09", axes);

    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
    assert_eq!(summary.recorded, 0);
}

#[test]
fn incomplete_counts_spans_with_either_axis_missing() {
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    let reason = Some(reason_id(&conn, "질병"));
    let kind = Some(type_id(&conn, "결석"));

    insert_span(&conn, class, a, TODAY, (reason, kind)); // 완성
    insert_span(&conn, class, a, TODAY, (reason, None)); // 종류 미정
    insert_span(&conn, class, a, TODAY, (None, kind)); // 구분 미정
    insert_span(&conn, class, a, TODAY, (None, None)); // 둘 다 미정

    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
    assert_eq!(summary.recorded, 1);
    assert_eq!(summary.incomplete, 3);
}

// ── 밀린 일 ───────────────────────────────────────────────────

#[test]
fn doc_pending_counts_everything_not_yet_received() {
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");

    let overdue = insert_span(&conn, class, a, "2026-08-20", axes);
    set_doc(&conn, overdue, false, Some("2026-08-27"));
    let ahead = insert_span(&conn, class, a, "2026-09-09", axes);
    set_doc(&conn, ahead, false, Some("2026-12-31"));
    let no_due = insert_span(&conn, class, a, "2026-09-09", axes);
    set_doc(&conn, no_due, false, None);
    let received = insert_span(&conn, class, a, "2026-09-09", axes);
    set_doc(&conn, received, true, Some("2026-09-16"));

    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
    // 마감이 지난 것만 세지 않는다 — 아직 안 받은 셋 전부다.
    assert_eq!(summary.doc_pending, 3);
    assert_eq!(summary.doc_overdue, 1);
}

#[test]
fn a_due_date_that_falls_on_the_given_day_is_not_overdue_yet() {
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");
    let today_due = insert_span(&conn, class, a, "2026-09-03", axes);
    set_doc(&conn, today_due, false, Some(TODAY));

    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
    assert_eq!(summary.doc_pending, 1);
    assert_eq!(summary.doc_overdue, 0);
}

#[test]
fn neis_pending_counts_records_not_yet_entered() {
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");

    insert_span(&conn, class, a, "2026-09-03", axes);
    insert_span(&conn, class, a, "2026-09-09", axes);
    let done = insert_span(&conn, class, a, "2026-09-09", axes);
    set_neis(&conn, done, true);

    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
    assert_eq!(summary.neis_pending, 2);
}

#[test]
fn neis_rows_start_from_the_oldest_day() {
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");
    insert_span(&conn, class, a, "2026-09-08", axes);
    insert_span(&conn, class, a, "2026-09-01", axes);
    insert_span(&conn, class, a, "2026-09-04", axes);

    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
    let dates: Vec<String> = summary.neis_rows.iter().map(|s| s.date.clone()).collect();
    assert_eq!(dates, ["2026-09-01", "2026-09-04", "2026-09-08"]);
}

#[test]
fn doc_rows_start_from_the_most_overdue() {
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");

    let mild = insert_span(&conn, class, a, "2026-09-01", axes);
    set_doc(&conn, mild, false, Some("2026-09-08"));
    let worst = insert_span(&conn, class, a, "2026-08-01", axes);
    set_doc(&conn, worst, false, Some("2026-08-08"));

    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
    assert_eq!(summary.doc_rows[0].id, worst);
    assert_eq!(summary.doc_rows[1].id, mild);
}

// ── limit ─────────────────────────────────────────────────────

#[test]
fn limit_caps_rows_but_never_the_counts() {
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");
    for day in ["2026-09-01", "2026-09-02", "2026-09-03"] {
        insert_span(&conn, class, a, day, axes);
    }

    let summary = get_home_summary_impl(&conn, class, TODAY, 1).unwrap();
    assert_eq!(summary.doc_pending, 3);
    assert_eq!(summary.neis_pending, 3);
    assert_eq!(summary.doc_rows.len(), 1);
    assert_eq!(summary.neis_rows.len(), 1);
}

#[test]
fn a_limit_of_zero_or_less_returns_no_rows() {
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    let axes = axes(&conn, "질병", "결석");
    insert_span(&conn, class, a, "2026-09-01", axes);

    for limit in [0, -3] {
        let summary = get_home_summary_impl(&conn, class, TODAY, limit).unwrap();
        assert_eq!(summary.doc_pending, 1);
        assert!(summary.doc_rows.is_empty());
        assert!(summary.neis_rows.is_empty());
    }
}

// ── 경계 ──────────────────────────────────────────────────────

#[test]
fn another_class_never_leaks_in() {
    let (conn, class, school) = fixture();
    // **같은 학생이 두 학급에 있다.** 학급마다 다른 학생을 쓰면 학적(학년 · 반)으로
    // 거르던 옛 질의로도 통과해 버려서, 이 테스트가 주장하는 것을 확인하지 못한다.
    // 로컬 전용 프로그램이라 두 학급 모두 내 것이고, 내 선택과목에 내 반 학생이 있다.
    let mine = enroll(&conn, class, school, 1, "김민준");
    let next_door = insert_class(&conn, school, "homeroom", "3학년 7반", Some(3), Some(7));
    join_class(&conn, next_door, mine);

    let axes = axes(&conn, "질병", "결석");
    insert_span(&conn, class, mine, TODAY, axes);
    // 옆 반에서 같은 학생에게 같은 날 한 건 더. 학생으로 거르면 둘 다 세어진다.
    insert_span(&conn, next_door, mine, TODAY, axes);

    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
    assert_eq!(summary.enrolled, 1);
    assert_eq!(summary.recorded, 1);
    assert_eq!(summary.doc_pending, 1, "옆 반 건이 섞이면 둘이 된다");
    assert_eq!(summary.neis_pending, 1);
}

#[test]
fn an_empty_class_summarizes_to_zero() {
    let (conn, class, _year) = fixture();
    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
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
    let (conn, class, _year) = fixture();
    let summary = get_home_summary_impl(&conn, class, TODAY, 10).unwrap();
    assert_eq!(summary.date, TODAY);
    assert!(
        summary.date_label.starts_with("2026.09.10."),
        "{}",
        summary.date_label
    );
}

#[test]
fn a_broken_date_is_rejected() {
    let (conn, class, _year) = fixture();
    let err = get_home_summary_impl(&conn, class, "어제", 10).unwrap_err();
    assert!(err.contains("어제"), "{err}");
}

#[test]
fn a_loosely_written_date_comes_back_in_iso() {
    // chrono는 `2026-9-10`도 받아들인다. 저장된 값은 언제나 자리를 채운 형식이므로
    // 그대로 질의하면 아무것도 걸리지 않는다.
    let (conn, class, school) = fixture();
    let a = enroll(&conn, class, school, 1, "김민준");
    insert_span(&conn, class, a, TODAY, axes(&conn, "질병", "결석"));

    let summary = get_home_summary_impl(&conn, class, "2026-9-10", 10).unwrap();
    assert_eq!(summary.date, TODAY);
    assert_eq!(summary.recorded, 1);
}

#[test]
fn 없는_학급은_이유를_말하고_거절한다() {
    // 범위가 학급 하나이므로 그것이 없으면 셀 것도 없다. 빈 요약으로 넘기면
    // 교사는 오늘 아무 일도 없는 것과 화면을 잘못 연 것을 구별할 수 없다.
    let (conn, _, _) = fixture();
    let err = get_home_summary_impl(&conn, 9999, TODAY, 10).unwrap_err();
    assert!(err.contains("학급을 찾을 수 없습니다"), "{err}");
}
