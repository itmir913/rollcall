//! 교사가 실제로 밟는 순서를 하나로 꿰어 본다.
//!
//! 모듈별 테스트는 각자의 규칙을 지키지만, **모듈 사이의 이음매**는 잡지 못한다.
//! 실제로 이 앱에서 처음 난 결함도 그 이음매였다 — 목록 커맨드가 다른 모듈의
//! 헬퍼를 부르면서 별칭 규약이 어긋나 SQL이 깨졌고, 각 모듈의 테스트는 전부 통과했다.
//!
//! 그래서 여기서는 명렬표 → 찍기 → 서류 → 나이스 → 통계를 한 흐름으로 확인한다.

use crate::commands::attendance::{
    delete_span_impl, edit_span_impl, get_day_grid_impl, get_month_log_impl, set_span_memo_impl,
    set_span_tag_impl, stamp_span_impl,
};
use crate::commands::home::get_home_summary_impl;
use crate::commands::mark::{
    get_doc_pending_impl, get_neis_pending_impl, mark_day_neis_impl, set_doc_done_impl,
};
use crate::commands::school::{add_off_day_impl, get_school_impl, update_school_impl};
use crate::commands::stats::get_quota_reports_impl;
use crate::tests::*;
use crate::types::{OffDayItem, SpanEdit, StampInput};

const TODAY: &str = "2026-09-10";
const SCHOOL_GRADE: i64 = 3;
const SCHOOL_CLASS: i64 = 6;

struct Fixture {
    conn: rusqlite::Connection,
    school: i64,
    year: i64,
    students: Vec<i64>,
}

fn fixture() -> Fixture {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let students = vec![
        insert_student(&conn, year, 5, "김하늘"),
        insert_student(&conn, year, 12, "박서연"),
        insert_student(&conn, year, 19, "임세훈"),
    ];
    Fixture {
        conn,
        school,
        year,
        students,
    }
}

fn stamp(f: &Fixture, student: i64, reason: &str, r#type: &str, slots: &[&str]) -> Vec<i64> {
    let (reason_id, type_id) = axes(&f.conn, reason, r#type);
    stamp_span_impl(
        &f.conn,
        &StampInput {
            student_id: student,
            date: TODAY.to_string(),
            reason_id,
            type_id,
            slots: slots.iter().map(|s| s.to_string()).collect(),
        },
    )
    .unwrap()
    .span_ids
}

/// 하루를 통째로 밟는다. 찍고 · 고치고 · 서류를 받고 · 나이스에 넣는다.
#[test]
fn a_whole_day_goes_through_every_screen() {
    let f = fixture();

    // 1. 아침에 찍는다. 결석 하나, 지각 하나.
    stamp(&f, f.students[0], "질병", "결석", &[]);
    stamp(&f, f.students[1], "미인정", "지각", &["2"]);

    let grid = get_day_grid_impl(&f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, TODAY)
        .unwrap();
    assert_eq!(grid.rows.len(), 3, "재학생은 구간이 없어도 행이 나온다");
    assert_eq!(grid.spans.len(), 2);

    // 2. 개요가 보는 숫자. 기록된 것은 **학생 수**이지 구간 수가 아니다.
    let summary = get_home_summary_impl(
        &f.conn,
        f.school,
        f.year,
        SCHOOL_GRADE,
        SCHOOL_CLASS,
        TODAY,
        5,
    )
    .unwrap();
    assert_eq!(summary.enrolled, 3);
    assert_eq!(summary.recorded, 2);
    assert_eq!(summary.doc_pending, 2, "안 받은 서류는 마감과 무관하게 전부 센다");
    assert_eq!(summary.neis_pending, 2);

    // 3. 서류 미제출자 화면에서 하나를 받는다. **그 줄은 목록에서 사라지지 않는다.**
    let pending = get_doc_pending_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, None, None, false, TODAY,
    )
    .unwrap();
    assert_eq!(pending.len(), 2);

    set_doc_done_impl(&f.conn, pending[0].id, true, TODAY).unwrap();
    let after = get_doc_pending_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, None, None, true, TODAY,
    )
    .unwrap();
    assert_eq!(after.len(), 2, "include_done이면 받은 것도 함께 보여준다");
    assert!(after.iter().any(|s| s.doc_done));

    // 4. 나이스에 하루치를 한 번에 넣는다.
    let marked = mark_day_neis_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, TODAY, TODAY,
    )
    .unwrap();
    assert_eq!(marked, 2);

    let neis = get_neis_pending_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, None, None, TODAY,
    )
    .unwrap();
    assert!(neis.is_empty(), "전부 등재했으므로 남는 날짜가 없다");

    // 5. 출결 기록은 날짜별로 묶인다.
    let log =
        get_month_log_impl(&f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, 2026, 9).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].spans.len(), 2);
    assert_eq!(log[0].enrolled, 3);
}

/// 같은 조합을 다시 찍으면 취소된다. 조합이 다르면 하루 2구간으로 쌓인다.
#[test]
fn stamping_twice_cancels_but_a_different_combination_stacks() {
    let f = fixture();

    stamp(&f, f.students[0], "질병", "지각", &["1"]);
    stamp(&f, f.students[0], "질병", "지각", &["1"]); // 같은 조합 — 취소
    let grid = get_day_grid_impl(&f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, TODAY)
        .unwrap();
    assert!(grid.spans.is_empty());

    stamp(&f, f.students[0], "질병", "지각", &["1"]);
    stamp(&f, f.students[0], "질병", "조퇴", &["5"]);
    let grid = get_day_grid_impl(&f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, TODAY)
        .unwrap();
    assert_eq!(grid.spans.len(), 2, "하루 2구간은 정상 입력이다");
}

/// 결과는 이어진 교시끼리 묶어 저장한다. 떨어진 교시를 한 구간으로 만들면
/// 사이의 교시가 조용히 포함된다.
#[test]
fn separate_periods_never_become_one_span() {
    let f = fixture();

    let ids = stamp(&f, f.students[0], "질병", "결과", &["1", "3", "5"]);
    assert_eq!(ids.len(), 3);

    let ids = stamp(&f, f.students[1], "질병", "결과", &["1", "2", "3"]);
    assert_eq!(ids.len(), 1);
}

/// 미완성 기록은 정상 상태다. 두 축이 비어도 저장되고, 개요가 그것을 센다.
#[test]
fn an_unfinished_record_is_saved_and_counted() {
    let f = fixture();

    stamp_span_impl(
        &f.conn,
        &StampInput {
            student_id: f.students[0],
            date: TODAY.to_string(),
            reason_id: None,
            type_id: None,
            slots: vec![],
        },
    )
    .unwrap();

    let summary = get_home_summary_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, TODAY, 5,
    )
    .unwrap();
    assert_eq!(summary.incomplete, 1);
    assert_eq!(summary.recorded, 1);
}

/// 마감은 만들 때 박는다. 설정을 나중에 바꿔도 과거 기록의 마감은 움직이지 않는다.
#[test]
fn changing_the_school_setting_never_moves_an_existing_due_date() {
    let f = fixture();
    stamp(&f, f.students[0], "질병", "결석", &[]);

    let before = get_doc_pending_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, None, None, false, TODAY,
    )
    .unwrap()[0]
        .doc_due
        .clone();

    let mut school = get_school_impl(&f.conn, f.school).unwrap();
    school.due_days = 1;
    update_school_impl(&f.conn, &school).unwrap();

    let after = get_doc_pending_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, None, None, false, TODAY,
    )
    .unwrap()[0]
        .doc_due
        .clone();

    assert_eq!(before, after, "이미 박힌 마감이 소급 변경되면 안 된다");
}

/// 휴업일을 등록하면 그 뒤에 만든 기록의 마감이 그만큼 밀린다.
#[test]
fn an_off_day_pushes_the_due_date_of_records_made_after_it() {
    let f = fixture();

    stamp(&f, f.students[0], "질병", "결석", &[]);
    let without = get_doc_pending_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, None, None, false, TODAY,
    )
    .unwrap()[0]
        .doc_due
        .clone();

    // 마감 구간 안에 휴업일을 넣고 다른 학생에게 같은 날짜로 찍는다.
    add_off_day_impl(
        &f.conn,
        f.school,
        &OffDayItem {
            id: 0,
            date: "2026-09-15".to_string(),
            label: Some("개교기념일".to_string()),
        },
    )
    .unwrap();
    stamp(&f, f.students[1], "질병", "결석", &[]);

    let rows = get_doc_pending_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, None, None, false, TODAY,
    )
    .unwrap();
    let with = rows
        .iter()
        .find(|s| s.student_id == f.students[1])
        .unwrap()
        .doc_due
        .clone();

    assert_ne!(without, with, "휴업일을 세지 않으므로 마감이 하루 밀린다");
}

/// 통계는 태그로 센다. 결석과 조퇴가 같은 태그면 함께 세어진다 —
/// 구분 × 종류 조합으로는 셀 수 없기 때문에 태그를 둔 것이다.
#[test]
fn the_quota_counts_by_tag_across_different_types() {
    let f = fixture();
    let trip = tag_id(&f.conn, "체험학습");

    let absence = stamp(&f, f.students[0], "출석인정", "결석", &[])[0];
    set_span_tag_impl(&f.conn, absence, Some(trip)).unwrap();

    let leave = stamp(&f, f.students[0], "출석인정", "조퇴", &["5"])[0];
    set_span_tag_impl(&f.conn, leave, Some(trip)).unwrap();

    let rule = quota_rule_id(&f.conn, "체험학습 연 20일");
    let reports = get_quota_reports_impl(
        &f.conn,
        f.school,
        f.year,
        SCHOOL_GRADE,
        SCHOOL_CLASS,
        Some(rule),
        None,
        None,
    )
    .unwrap();

    let row = reports[0]
        .rows
        .iter()
        .find(|r| r.student_id == f.students[0])
        .unwrap();
    assert_eq!(row.used, 1, "하루에 두 건이어도 1일이다");
}

/// 태그가 빠진 건은 조용히 넘기지 않는다. 학년말에 발견되면 이미 늦다.
#[test]
fn a_record_without_a_tag_is_shown_instead_of_being_skipped() {
    let f = fixture();
    stamp(&f, f.students[0], "출석인정", "결석", &[]);

    let rule = quota_rule_id(&f.conn, "체험학습 연 20일");
    let reports = get_quota_reports_impl(
        &f.conn,
        f.school,
        f.year,
        SCHOOL_GRADE,
        SCHOOL_CLASS,
        Some(rule),
        None,
        None,
    )
    .unwrap();

    assert!(!reports[0].untagged.is_empty());
}

/// 고치기는 마감을 다시 계산하지 않고, 지우기는 그 줄만 없앤다.
#[test]
fn editing_keeps_the_due_date_and_deleting_removes_only_that_span() {
    let f = fixture();
    let first = stamp(&f, f.students[0], "질병", "지각", &["1"])[0];
    let second = stamp(&f, f.students[0], "질병", "조퇴", &["5"])[0];

    let before = get_doc_pending_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, None, None, false, TODAY,
    )
    .unwrap()
    .into_iter()
    .find(|s| s.id == first)
    .unwrap()
    .doc_due;

    let (reason_id, type_id) = axes(&f.conn, "미인정", "지각");
    edit_span_impl(
        &f.conn,
        &SpanEdit {
            span_id: first,
            reason_id,
            type_id,
            slots: vec!["3".to_string()],
        },
    )
    .unwrap();
    set_span_memo_impl(&f.conn, first, "늦잠").unwrap();

    let rows = get_doc_pending_impl(
        &f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, None, None, false, TODAY,
    )
    .unwrap();
    let edited = rows.iter().find(|s| s.id == first).unwrap();
    assert_eq!(edited.doc_due, before, "고쳐도 마감은 그대로다");
    assert_eq!(edited.reason_label.as_deref(), Some("미인정"));
    assert_eq!(edited.memo, "늦잠");

    delete_span_impl(&f.conn, first).unwrap();
    let grid = get_day_grid_impl(&f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, TODAY)
        .unwrap();
    assert_eq!(grid.spans.len(), 1);
    assert_eq!(grid.spans[0].id, second);
}

/// 다른 학교의 같은 학년·반이 섞이지 않는다.
#[test]
fn another_school_never_leaks_into_this_class() {
    let f = fixture();
    let other = f
        .conn
        .query_row(
            "INSERT INTO school (name, max_slot, due_days, due_skip_offdays)
             VALUES ('다른 학교', 7, 7, 1) RETURNING id",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();
    f.conn
        .execute(
            "INSERT INTO student (school_id, year_id, grade, class_no, number, name, enrolled_from)
             VALUES (?1, ?2, 3, 6, 5, '동명이인', '2026-03-02')",
            rusqlite::params![other, f.year],
        )
        .unwrap();

    let grid = get_day_grid_impl(&f.conn, f.school, f.year, SCHOOL_GRADE, SCHOOL_CLASS, TODAY)
        .unwrap();
    assert_eq!(grid.rows.len(), 3, "우리 학교 학생만 나온다");
}
