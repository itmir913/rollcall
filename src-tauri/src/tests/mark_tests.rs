//! 서류 · 나이스 표시와 두 미제출 목록.
//!
//! 구간은 SQL로 직접 넣는다. 찍기 커맨드는 다른 모듈의 것이고, 여기서 검사하려는 것은
//! 표시와 목록이지 입력 경로가 아니다.

use crate::commands::mark::*;
use crate::tests::*;
use rusqlite::Connection;

const TODAY: &str = "2026-09-11";
const GRADE: i64 = 3;
const CLASS: i64 = 6;

/// 구간 하나. 두 축은 비워 둔다 — 미완성 기록이 정상 상태다.
fn add_span(conn: &Connection, student_id: i64, date: &str, doc_due: Option<&str>) -> i64 {
    conn.execute(
        "INSERT INTO absence_span (student_id, date, doc_due) VALUES (?1, ?2, ?3)",
        rusqlite::params![student_id, date, doc_due],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 두 축이 채워진 구간.
fn add_full_span(conn: &Connection, student_id: i64, date: &str, doc_due: Option<&str>) -> i64 {
    let (reason, r#type) = axes(conn, "질병", "결석");
    conn.execute(
        "INSERT INTO absence_span (student_id, date, reason_id, type_id, doc_due)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![student_id, date, reason, r#type, doc_due],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 다른 반 학생. `insert_student`는 3학년 6반에 고정돼 있다.
fn add_student_in(
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

fn doc_flags(conn: &Connection, span_id: i64) -> (i64, Option<String>) {
    conn.query_row(
        "SELECT doc_done, doc_done_on FROM absence_span WHERE id = ?1",
        rusqlite::params![span_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap()
}

fn neis_flags(conn: &Connection, span_id: i64) -> (i64, Option<String>) {
    conn.query_row(
        "SELECT neis_done, neis_done_on FROM absence_span WHERE id = ?1",
        rusqlite::params![span_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap()
}

fn ids(spans: &[crate::types::SpanItem]) -> Vec<i64> {
    spans.iter().map(|s| s.id).collect()
}

// ── 표시 ──────────────────────────────────────────────────────

#[test]
fn doc_done_toggles_both_ways() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 1, "김가온");
    let span = add_span(&conn, student, "2026-09-07", Some("2026-09-14"));

    set_doc_done_impl(&conn, span, true, TODAY).unwrap();
    assert_eq!(doc_flags(&conn, span), (1, Some(TODAY.to_string())));

    // 해제하면 받은 날짜도 함께 비운다. 남겨 두면 다음 화면이 "받은 적 있다"로 읽는다.
    set_doc_done_impl(&conn, span, false, TODAY).unwrap();
    assert_eq!(doc_flags(&conn, span), (0, None));
}

#[test]
fn neis_done_toggles_both_ways() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 1, "김가온");
    let span = add_span(&conn, student, "2026-09-07", None);

    set_neis_done_impl(&conn, span, true, TODAY).unwrap();
    assert_eq!(neis_flags(&conn, span), (1, Some(TODAY.to_string())));

    set_neis_done_impl(&conn, span, false, TODAY).unwrap();
    assert_eq!(neis_flags(&conn, span), (0, None));
}

#[test]
fn marking_unknown_span_reports_the_id() {
    let conn = setup_test_db();

    let err = set_doc_done_impl(&conn, 999, true, TODAY).unwrap_err();
    assert!(err.contains("999"), "{err}");

    let err = set_neis_done_impl(&conn, 999, true, TODAY).unwrap_err();
    assert!(err.contains("999"), "{err}");
}

#[test]
fn marking_rejects_a_malformed_today() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 1, "김가온");
    let span = add_span(&conn, student, "2026-09-07", None);

    assert!(set_doc_done_impl(&conn, span, true, "2026.09.11").is_err());
    assert!(set_neis_done_impl(&conn, span, true, "9월 11일").is_err());
    // 거절했으니 표시도 남지 않는다.
    assert_eq!(doc_flags(&conn, span), (0, None));
    assert_eq!(neis_flags(&conn, span), (0, None));
}

/// 자리를 채우지 않은 날짜도 ISO로 맞춰 저장한다. 날짜 비교가 문자열 비교인 자리에서
/// `2026-9-11`은 `2026-09-01`보다 앞선 것으로 읽힌다.
#[test]
fn marking_stores_the_date_in_iso() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 1, "김가온");
    let span = add_span(&conn, student, "2026-09-07", None);

    set_doc_done_impl(&conn, span, true, "2026-9-11").unwrap();
    assert_eq!(doc_flags(&conn, span), (1, Some("2026-09-11".to_string())));
}

// ── 하루치 나이스 등재 ────────────────────────────────────────

#[test]
fn mark_day_neis_counts_only_the_unmarked() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");
    let b = insert_student(&conn, year, 2, "이나래");

    let first = add_span(&conn, a, "2026-09-10", None);
    let second = add_span(&conn, b, "2026-09-10", None);
    let already = add_span(&conn, b, "2026-09-10", None);
    set_neis_done_impl(&conn, already, true, "2026-09-10").unwrap();

    let n = mark_day_neis_impl(&conn, school, year, GRADE, CLASS, "2026-09-10", TODAY).unwrap();
    assert_eq!(n, 2);

    assert_eq!(neis_flags(&conn, first), (1, Some(TODAY.to_string())));
    assert_eq!(neis_flags(&conn, second), (1, Some(TODAY.to_string())));
    // 이미 등재한 건의 날짜를 오늘로 덮지 않는다. 언제 넣었는지가 사라진다.
    assert_eq!(
        neis_flags(&conn, already),
        (1, Some("2026-09-10".to_string()))
    );
}

#[test]
fn mark_day_neis_leaves_other_days_and_classes_alone() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let mine = insert_student(&conn, year, 1, "김가온");
    let other_class = add_student_in(&conn, year, GRADE, CLASS + 1, 1, "박다솜");

    let today_span = add_span(&conn, mine, "2026-09-10", None);
    let other_day = add_span(&conn, mine, "2026-09-09", None);
    let other_class_span = add_span(&conn, other_class, "2026-09-10", None);

    let n = mark_day_neis_impl(&conn, school, year, GRADE, CLASS, "2026-09-10", TODAY).unwrap();
    assert_eq!(n, 1);

    assert_eq!(neis_flags(&conn, today_span).0, 1);
    assert_eq!(neis_flags(&conn, other_day).0, 0);
    assert_eq!(neis_flags(&conn, other_class_span).0, 0);
}

#[test]
fn mark_day_neis_returns_zero_when_nothing_is_left() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김가온");

    let n = mark_day_neis_impl(&conn, school, year, GRADE, CLASS, "2026-09-10", TODAY).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn mark_day_neis_rejects_a_malformed_date() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 1, "김가온");
    let span = add_span(&conn, student, "2026-09-10", None);

    assert!(mark_day_neis_impl(&conn, school, year, GRADE, CLASS, "9월 10일", TODAY).is_err());
    assert_eq!(neis_flags(&conn, span).0, 0);
}

#[test]
fn mark_day_neis_matches_the_date_in_iso() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 1, "김가온");
    let span = add_span(&conn, student, "2026-09-10", None);

    let n = mark_day_neis_impl(&conn, school, year, GRADE, CLASS, "2026-9-10", TODAY).unwrap();
    assert_eq!(n, 1);
    assert_eq!(neis_flags(&conn, span).0, 1);
}

/// 트랜잭션이 열린 채 남으면 이후 모든 BEGIN이 실패한다. 커넥션이 하나뿐이라
/// 그 상태가 세션 내내 이어진다.
#[test]
fn mark_day_neis_leaves_no_open_transaction() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 1, "김가온");
    add_span(&conn, student, "2026-09-10", None);

    mark_day_neis_impl(&conn, school, year, GRADE, CLASS, "2026-09-10", TODAY).unwrap();

    conn.execute_batch("BEGIN")
        .expect("트랜잭션이 열린 채 남았다");
    conn.execute_batch("ROLLBACK").unwrap();
}

// ── 서류 미제출 목록 ──────────────────────────────────────────

#[test]
fn doc_pending_sorts_the_most_overdue_first() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");
    let b = insert_student(&conn, year, 2, "이나래");

    let soon = add_span(&conn, a, "2026-09-10", Some("2026-09-17")); // 아직 남았다
    let overdue = add_span(&conn, b, "2026-09-01", Some("2026-09-04")); // 일주일 지났다
    let today_due = add_span(&conn, a, "2026-09-04", Some(TODAY)); // 오늘이 마감

    let rows =
        get_doc_pending_impl(&conn, school, year, GRADE, CLASS, None, None, false, TODAY).unwrap();
    assert_eq!(ids(&rows), vec![overdue, today_due, soon]);
}

#[test]
fn doc_pending_orders_the_same_due_by_student_number() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");
    let b = insert_student(&conn, year, 2, "이나래");

    let later_number = add_span(&conn, b, "2026-09-01", Some("2026-09-04"));
    let first_number = add_span(&conn, a, "2026-09-01", Some("2026-09-04"));

    let rows =
        get_doc_pending_impl(&conn, school, year, GRADE, CLASS, None, None, false, TODAY).unwrap();
    assert_eq!(ids(&rows), vec![first_number, later_number]);
}

#[test]
fn doc_pending_puts_spans_without_a_due_last() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");

    let no_due = add_span(&conn, a, "2026-09-01", None);
    let has_due = add_span(&conn, a, "2026-09-10", Some("2026-09-17"));

    let rows =
        get_doc_pending_impl(&conn, school, year, GRADE, CLASS, None, None, false, TODAY).unwrap();
    assert_eq!(ids(&rows), vec![has_due, no_due]);
}

#[test]
fn doc_pending_hides_received_documents_unless_asked() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");

    let received = add_span(&conn, a, "2026-09-01", Some("2026-09-04"));
    let pending = add_span(&conn, a, "2026-09-02", Some("2026-09-05"));
    set_doc_done_impl(&conn, received, true, TODAY).unwrap();

    let rows =
        get_doc_pending_impl(&conn, school, year, GRADE, CLASS, None, None, false, TODAY).unwrap();
    assert_eq!(ids(&rows), vec![pending]);

    let rows =
        get_doc_pending_impl(&conn, school, year, GRADE, CLASS, None, None, true, TODAY).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|s| s.id == received && s.doc_done));
}

/// 체크한 줄은 그 자리에 남는다. 화면에서 방금 누른 줄이 아래로 뛰면 다음 줄을
/// 다시 찾아야 한다.
#[test]
fn doc_pending_keeps_a_checked_row_in_place() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");
    let b = insert_student(&conn, year, 2, "이나래");

    let oldest = add_span(&conn, a, "2026-09-01", Some("2026-09-02"));
    let middle = add_span(&conn, b, "2026-09-03", Some("2026-09-05"));
    let newest = add_span(&conn, b, "2026-09-08", Some("2026-09-15"));

    let before =
        get_doc_pending_impl(&conn, school, year, GRADE, CLASS, None, None, true, TODAY).unwrap();
    assert_eq!(ids(&before), vec![oldest, middle, newest]);

    set_doc_done_impl(&conn, oldest, true, TODAY).unwrap();
    let after =
        get_doc_pending_impl(&conn, school, year, GRADE, CLASS, None, None, true, TODAY).unwrap();
    assert_eq!(ids(&after), vec![oldest, middle, newest]);
}

#[test]
fn doc_pending_filters_by_month_and_by_year() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");

    let june = add_span(&conn, a, "2026-06-15", Some("2026-06-22"));
    let september = add_span(&conn, a, "2026-09-01", Some("2026-09-08"));
    let january = add_span(&conn, a, "2027-01-12", Some("2027-01-19"));

    let rows = get_doc_pending_impl(
        &conn,
        school,
        year,
        GRADE,
        CLASS,
        Some(2026),
        Some(6),
        false,
        TODAY,
    )
    .unwrap();
    assert_eq!(ids(&rows), vec![june]);

    let rows = get_doc_pending_impl(
        &conn,
        school,
        year,
        GRADE,
        CLASS,
        Some(2026),
        None,
        false,
        TODAY,
    )
    .unwrap();
    let found = ids(&rows);
    assert!(found.contains(&june) && found.contains(&september));
    assert!(!found.contains(&january));
}

#[test]
fn doc_pending_rejects_a_month_without_a_year() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);

    let err = get_doc_pending_impl(
        &conn,
        school,
        year,
        GRADE,
        CLASS,
        None,
        Some(9),
        false,
        TODAY,
    )
    .unwrap_err();
    assert!(err.contains("연도"), "{err}");

    let err = get_doc_pending_impl(
        &conn,
        school,
        year,
        GRADE,
        CLASS,
        Some(2026),
        Some(13),
        false,
        TODAY,
    )
    .unwrap_err();
    assert!(err.contains("13"), "{err}");
}

#[test]
fn doc_pending_covers_only_the_asked_class() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let mine = insert_student(&conn, year, 1, "김가온");
    let other = add_student_in(&conn, year, GRADE, CLASS + 1, 1, "박다솜");

    let ours = add_span(&conn, mine, "2026-09-01", Some("2026-09-08"));
    add_span(&conn, other, "2026-09-01", Some("2026-09-08"));

    let rows =
        get_doc_pending_impl(&conn, school, year, GRADE, CLASS, None, None, false, TODAY).unwrap();
    assert_eq!(ids(&rows), vec![ours]);
}

/// 두 축이 비어 있어도 목록에서 빼지 않는다. 프로그램은 판정하지 않는다.
#[test]
fn doc_pending_lists_incomplete_records_too() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");

    let bare = add_span(&conn, a, "2026-09-01", Some("2026-09-08"));
    let full = add_full_span(&conn, a, "2026-09-02", Some("2026-09-09"));

    let rows =
        get_doc_pending_impl(&conn, school, year, GRADE, CLASS, None, None, false, TODAY).unwrap();
    let found = ids(&rows);
    assert!(found.contains(&bare) && found.contains(&full));
}

// ── 나이스 미등재 목록 ────────────────────────────────────────

#[test]
fn neis_pending_groups_by_date_from_the_oldest() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");
    let b = insert_student(&conn, year, 2, "이나래");

    add_span(&conn, a, "2026-09-08", None);
    let later_number = add_span(&conn, b, "2026-09-02", None);
    let first_number = add_span(&conn, a, "2026-09-02", None);

    let groups =
        get_neis_pending_impl(&conn, school, year, GRADE, CLASS, None, None, TODAY).unwrap();
    let dates: Vec<&str> = groups.iter().map(|g| g.date.as_str()).collect();
    assert_eq!(dates, vec!["2026-09-02", "2026-09-08"]);
    assert_eq!(groups[0].date_label, "2026.09.02.(수)");
    // 하루 안에서는 번호 순이다. 나이스 화면이 번호 순으로 늘어서 있다.
    assert_eq!(ids(&groups[0].spans), vec![first_number, later_number]);
}

#[test]
fn neis_pending_drops_a_day_once_it_is_marked() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");

    add_span(&conn, a, "2026-09-02", None);
    add_span(&conn, a, "2026-09-08", None);

    mark_day_neis_impl(&conn, school, year, GRADE, CLASS, "2026-09-02", TODAY).unwrap();

    let groups =
        get_neis_pending_impl(&conn, school, year, GRADE, CLASS, None, None, TODAY).unwrap();
    let dates: Vec<&str> = groups.iter().map(|g| g.date.as_str()).collect();
    assert_eq!(dates, vec!["2026-09-08"]);
}

/// 전출한 학생의 기록은 목록에 남지만 그날 머릿수에서는 빠진다.
#[test]
fn neis_pending_counts_students_enrolled_on_that_date() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let stays = insert_student(&conn, year, 1, "김가온");
    let leaves = insert_student(&conn, year, 2, "이나래");
    conn.execute(
        "UPDATE student SET enrolled_to = '2026-09-05' WHERE id = ?1",
        rusqlite::params![leaves],
    )
    .unwrap();

    let before = add_span(&conn, leaves, "2026-09-02", None);
    add_span(&conn, stays, "2026-09-08", None);

    let groups =
        get_neis_pending_impl(&conn, school, year, GRADE, CLASS, None, None, TODAY).unwrap();
    assert_eq!(groups[0].date, "2026-09-02");
    assert_eq!(groups[0].enrolled, 2);
    assert_eq!(ids(&groups[0].spans), vec![before]);
    assert_eq!(groups[1].date, "2026-09-08");
    assert_eq!(groups[1].enrolled, 1);
}

#[test]
fn neis_pending_filters_by_month() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");

    add_span(&conn, a, "2026-06-15", None);
    add_span(&conn, a, "2026-09-01", None);

    let groups = get_neis_pending_impl(
        &conn,
        school,
        year,
        GRADE,
        CLASS,
        Some(2026),
        Some(9),
        TODAY,
    )
    .unwrap();
    let dates: Vec<&str> = groups.iter().map(|g| g.date.as_str()).collect();
    assert_eq!(dates, vec!["2026-09-01"]);
}

/// 기준일이 깨져 있으면 목록도 거절한다. 빈 목록으로 돌려주면 교사는
/// "밀린 일이 없다"로 읽는다.
#[test]
fn pending_lists_reject_a_malformed_today() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");
    add_span(&conn, a, "2026-09-01", Some("2026-09-08"));

    assert!(get_doc_pending_impl(
        &conn,
        school,
        year,
        GRADE,
        CLASS,
        None,
        None,
        false,
        "9월 11일"
    )
    .is_err());
    assert!(
        get_neis_pending_impl(&conn, school, year, GRADE, CLASS, None, None, "2026.09.11").is_err()
    );
}

/// 나이스 목록도 같은 월 필터를 쓴다. 서류 쪽만 거절하면 한쪽 화면에서
/// 엉뚱한 기간이 조용히 통과한다.
#[test]
fn neis_pending_rejects_a_month_it_cannot_place() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);

    let err =
        get_neis_pending_impl(&conn, school, year, GRADE, CLASS, None, Some(9), TODAY).unwrap_err();
    assert!(err.contains("연도"), "{err}");

    let err = get_neis_pending_impl(
        &conn,
        school,
        year,
        GRADE,
        CLASS,
        Some(2026),
        Some(0),
        TODAY,
    )
    .unwrap_err();
    assert!(err.contains("0"), "{err}");
}

#[test]
fn neis_pending_is_empty_for_an_unknown_class() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김가온");
    add_span(&conn, a, "2026-09-01", None);

    let groups = get_neis_pending_impl(&conn, school, year, GRADE, 99, None, None, TODAY).unwrap();
    assert!(groups.is_empty());

    let rows =
        get_doc_pending_impl(&conn, school, 999, GRADE, CLASS, None, None, false, TODAY).unwrap();
    assert!(rows.is_empty());
}
