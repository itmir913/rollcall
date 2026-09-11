//! 출결 입력과 수정의 시험.
//!
//! 이 모듈이 지키는 것은 두 가지다 — **교사가 찍은 것을 그대로 저장한다**는 것과,
//! **같은 조합을 다시 찍으면 취소된다**는 것. 나머지(한도 · 겹침 · 미완성)는
//! 저장을 막지 않고 표시할 값만 계산한다.

use crate::commands::attendance::*;
use crate::tests::*;
use crate::types::{SpanEdit, SpanItem, StampInput};
use rusqlite::{params, Connection};

// ── 준비 ──────────────────────────────────────────────────────

struct Fixture {
    conn: Connection,
    school: i64,
    year: i64,
    /// 3학년 6반 1 · 2 · 3번
    students: Vec<i64>,
}

fn fixture() -> Fixture {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let students = vec![
        insert_student(&conn, year, 1, "김하나"),
        insert_student(&conn, year, 2, "이두리"),
        insert_student(&conn, year, 3, "박세찬"),
    ];
    Fixture {
        conn,
        school,
        year,
        students,
    }
}

fn stamp(student_id: i64, date: &str, reason_id: Option<i64>, type_id: Option<i64>, slots: &[&str]) -> StampInput {
    StampInput {
        student_id,
        date: date.to_string(),
        reason_id,
        type_id,
        slots: slots.iter().map(|s| s.to_string()).collect(),
    }
}

/// 기준일은 마감 경과일에만 쓰인다. 그 값을 보지 않는 시험은 아무 날이나 넘긴다.
fn spans_on(f: &Fixture, student_id: i64, today: &str) -> Vec<SpanItem> {
    load_spans(
        &f.conn,
        "WHERE s.student_id = ?1 ORDER BY s.id",
        &[&student_id],
        today,
    )
    .unwrap()
}

fn spans_of(f: &Fixture, student_id: i64) -> Vec<SpanItem> {
    spans_on(f, student_id, "2026-09-10")
}

fn span_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap()
}

fn add_off_day(conn: &Connection, school: i64, date: &str) {
    conn.execute(
        "INSERT INTO off_day (school_id, date, label) VALUES (?1, ?2, '재량휴업일')",
        params![school, date],
    )
    .unwrap();
}

fn doc_due_of(conn: &Connection, span_id: i64) -> Option<String> {
    conn.query_row(
        "SELECT doc_due FROM absence_span WHERE id = ?1",
        params![span_id],
        |r| r.get(0),
    )
    .unwrap()
}

// ── 같은 조합 = 무르기 ────────────────────────────────────────

#[test]
fn 같은_조합을_두_번_찍으면_취소된다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    let input = stamp(f.students[0], "2026-09-10", reason, kind, &[]);

    let first = stamp_span_impl(&f.conn, &input).unwrap();
    assert_eq!(first.action, "added");
    assert_eq!(first.span_ids.len(), 1);
    assert_eq!(span_count(&f.conn), 1);

    let second = stamp_span_impl(&f.conn, &input).unwrap();
    assert_eq!(second.action, "cancelled");
    assert_eq!(second.span_ids, first.span_ids);
    assert_eq!(span_count(&f.conn), 0);

    // 한 번 더 누르면 다시 생긴다. 무르기는 삭제가 아니라 되돌리기다.
    let third = stamp_span_impl(&f.conn, &input).unwrap();
    assert_eq!(third.action, "added");
    assert_eq!(span_count(&f.conn), 1);
}

#[test]
fn 조합이_하나라도_다르면_쌓인다() {
    let f = fixture();
    let (reason, late) = axes(&f.conn, "질병", "지각");
    let (_, early) = axes(&f.conn, "질병", "조퇴");

    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, late, &["1"])).unwrap();
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, early, &["5"])).unwrap();

    // 하루 2구간은 정상이다.
    let rows = spans_of(&f, f.students[0]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].start_slot.as_deref(), Some("조회"));
    assert_eq!(rows[0].end_slot.as_deref(), Some("1"));
    assert_eq!(rows[1].start_slot.as_deref(), Some("5"));
    assert_eq!(rows[1].end_slot.as_deref(), Some("종례"));
}

#[test]
fn 구분만_달라도_다른_건이다() {
    let f = fixture();
    let (sick, kind) = axes(&f.conn, "질병", "결석");
    let (unapproved, _) = axes(&f.conn, "미인정", "결석");

    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", sick, kind, &[])).unwrap();
    let other =
        stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", unapproved, kind, &[])).unwrap();

    assert_eq!(other.action, "added");
    assert_eq!(span_count(&f.conn), 2);
}

// ── 기간 규칙 ─────────────────────────────────────────────────

#[test]
fn 이어지지_않은_교시는_구간이_나뉜다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결과");

    let out = stamp_span_impl(
        &f.conn,
        &stamp(f.students[0], "2026-09-10", reason, kind, &["1", "3", "5"]),
    )
    .unwrap();
    assert_eq!(out.span_ids.len(), 3);

    let rows = spans_of(&f, f.students[0]);
    let pairs: Vec<(String, String)> = rows
        .iter()
        .map(|r| {
            (
                r.start_slot.clone().unwrap(),
                r.end_slot.clone().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("1".to_string(), "1".to_string()),
            ("3".to_string(), "3".to_string()),
            ("5".to_string(), "5".to_string()),
        ]
    );
}

#[test]
fn 이어진_교시는_한_구간이다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결과");

    let out = stamp_span_impl(
        &f.conn,
        &stamp(f.students[0], "2026-09-10", reason, kind, &["3", "1", "2"]),
    )
    .unwrap();
    assert_eq!(out.span_ids.len(), 1);

    let rows = spans_of(&f, f.students[0]);
    assert_eq!(rows[0].start_slot.as_deref(), Some("1"));
    assert_eq!(rows[0].end_slot.as_deref(), Some("3"));
    assert_eq!(rows[0].span_text, "1교시부터 3교시까지");
}

#[test]
fn 결석은_교시를_묻지_않고_하루_전체다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[])).unwrap();

    let rows = spans_of(&f, f.students[0]);
    assert_eq!(rows[0].start_slot.as_deref(), Some("조회"));
    assert_eq!(rows[0].end_slot.as_deref(), Some("종례"));
    assert_eq!(rows[0].span_text, "하루 종일");
}

#[test]
fn 지각은_조회부터_조퇴는_종례까지다() {
    let f = fixture();
    let (reason, late) = axes(&f.conn, "질병", "지각");
    let (_, early) = axes(&f.conn, "질병", "조퇴");

    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, late, &["2"])).unwrap();
    stamp_span_impl(&f.conn, &stamp(f.students[1], "2026-09-10", reason, early, &["5"])).unwrap();

    let late_row = &spans_of(&f, f.students[0])[0];
    assert_eq!(late_row.start_slot.as_deref(), Some("조회"));
    assert_eq!(late_row.end_slot.as_deref(), Some("2"));
    assert_eq!(late_row.span_text, "조회부터 2교시까지");

    let early_row = &spans_of(&f, f.students[1])[0];
    assert_eq!(early_row.start_slot.as_deref(), Some("5"));
    assert_eq!(early_row.end_slot.as_deref(), Some("종례"));
    assert_eq!(early_row.span_text, "5교시부터 종례까지");
}

#[test]
fn 축과_기간이_비어도_저장된다() {
    let f = fixture();
    // 안 왔는데 연락이 닿지 않는 상태. 프로그램은 이것을 거절하지 않는다.
    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", None, None, &[])).unwrap();
    assert_eq!(out.action, "added");

    let rows = spans_of(&f, f.students[0]);
    assert!(rows[0].start_slot.is_none());
    assert!(rows[0].end_slot.is_none());
    assert!(!rows[0].complete);
    assert!(rows[0].code_label.is_none());
    assert_eq!(rows[0].span_text, "기간 미정");
}

#[test]
fn 최대_교시를_넘는_교시는_거절된다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결과");
    // 학교 설정의 최대 교시가 7이다. 표현할 수 없는 구간만 막는다.
    let err = stamp_span_impl(
        &f.conn,
        &stamp(f.students[0], "2026-09-10", reason, kind, &["9"]),
    )
    .unwrap_err();
    assert!(err.contains("알 수 없는 교시"), "{err}");
    assert_eq!(span_count(&f.conn), 0);
}

#[test]
fn 최대_교시는_학교에서_읽는다() {
    let f = fixture();
    f.conn
        .execute("UPDATE school SET max_slot = 5 WHERE id = ?1", params![f.school])
        .unwrap();
    let (reason, kind) = axes(&f.conn, "질병", "결과");

    assert!(stamp_span_impl(
        &f.conn,
        &stamp(f.students[0], "2026-09-10", reason, kind, &["6"])
    )
    .is_err());
    assert!(stamp_span_impl(
        &f.conn,
        &stamp(f.students[0], "2026-09-10", reason, kind, &["5"])
    )
    .is_ok());
}

// ── 겹침은 막지 않고 표시한다 ─────────────────────────────────

#[test]
fn 겹치는_구간도_저장되고_표시만_된다() {
    let f = fixture();
    let (reason, missed) = axes(&f.conn, "질병", "결과");
    let (_, early) = axes(&f.conn, "질병", "조퇴");

    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, missed, &["3"])).unwrap();
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, early, &["3"])).unwrap();
    assert_eq!(span_count(&f.conn), 2);

    let rows = spans_of(&f, f.students[0]);
    assert!(rows.iter().all(|r| r.overlapping), "겹침은 표시되어야 한다");
}

#[test]
fn 겹치지_않는_구간은_표시되지_않는다() {
    let f = fixture();
    let (reason, missed) = axes(&f.conn, "질병", "결과");

    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, missed, &["1", "5"]))
        .unwrap();
    let rows = spans_of(&f, f.students[0]);
    assert!(rows.iter().all(|r| !r.overlapping));
}

// ── 마감일 ────────────────────────────────────────────────────

#[test]
fn 마감일은_주말을_건너뛴다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    // 2026-09-10은 목요일이고 기본 제출 기한은 7일이다.
    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[]))
        .unwrap();
    assert_eq!(
        doc_due_of(&f.conn, out.span_ids[0]).as_deref(),
        Some("2026-09-21")
    );
}

#[test]
fn 마감일은_등록된_휴업일도_건너뛴다() {
    let f = fixture();
    add_off_day(&f.conn, f.school, "2026-09-16");
    let (reason, kind) = axes(&f.conn, "질병", "결석");

    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[]))
        .unwrap();
    assert_eq!(
        doc_due_of(&f.conn, out.span_ids[0]).as_deref(),
        Some("2026-09-22")
    );
}

#[test]
fn 마감일을_세지_않기로_하면_날짜를_그대로_더한다() {
    let f = fixture();
    f.conn
        .execute(
            "UPDATE school SET due_skip_offdays = 0 WHERE id = ?1",
            params![f.school],
        )
        .unwrap();
    let (reason, kind) = axes(&f.conn, "질병", "결석");

    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[]))
        .unwrap();
    assert_eq!(
        doc_due_of(&f.conn, out.span_ids[0]).as_deref(),
        Some("2026-09-17")
    );
}

#[test]
fn 마감_경과일은_서류를_받으면_사라진다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[]))
        .unwrap();
    let id = out.span_ids[0];

    // 마감은 2026-09-21이다. 기준일을 넘겨 세므로 오늘 날짜에 흔들리지 않는다.
    let rows = spans_on(&f, f.students[0], "2026-09-25");
    assert_eq!(rows[0].days_overdue, Some(4));

    f.conn
        .execute(
            "UPDATE absence_span SET doc_done = 1, doc_done_on = '2026-09-20' WHERE id = ?1",
            params![id],
        )
        .unwrap();
    let rows = spans_on(&f, f.students[0], "2026-09-25");
    assert_eq!(rows[0].days_overdue, None);
    assert!(rows[0].doc_done);
}

// ── 수정 · 삭제 ───────────────────────────────────────────────

#[test]
fn 수정은_마감일을_다시_계산하지_않는다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[]))
        .unwrap();
    let id = out.span_ids[0];
    let before = doc_due_of(&f.conn, id);

    // 설정을 바꾸고 축과 기간을 고쳐도 이미 박힌 마감은 움직이지 않는다.
    f.conn
        .execute("UPDATE school SET due_days = 1 WHERE id = ?1", params![f.school])
        .unwrap();
    add_off_day(&f.conn, f.school, "2026-09-16");

    let late = Some(type_id(&f.conn, "지각"));
    edit_span_impl(
        &f.conn,
        &SpanEdit {
            span_id: id,
            reason_id: reason,
            type_id: late,
            slots: vec!["2".to_string()],
        },
    )
    .unwrap();

    assert_eq!(doc_due_of(&f.conn, id), before);
    let rows = spans_of(&f, f.students[0]);
    assert_eq!(rows[0].start_slot.as_deref(), Some("조회"));
    assert_eq!(rows[0].end_slot.as_deref(), Some("2"));
    assert_eq!(rows[0].type_label.as_deref(), Some("지각"));
}

#[test]
fn 수정으로_축을_비울_수_있다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[]))
        .unwrap();

    edit_span_impl(
        &f.conn,
        &SpanEdit {
            span_id: out.span_ids[0],
            reason_id: None,
            type_id: None,
            slots: vec![],
        },
    )
    .unwrap();

    let rows = spans_of(&f, f.students[0]);
    assert!(!rows[0].complete);
    assert!(rows[0].start_slot.is_none());
    assert_eq!(rows[0].span_text, "기간 미정");
}

#[test]
fn 이어지지_않은_교시로는_한_구간을_고칠_수_없다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결과");
    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &["1"]))
        .unwrap();

    let err = edit_span_impl(
        &f.conn,
        &SpanEdit {
            span_id: out.span_ids[0],
            reason_id: reason,
            type_id: kind,
            slots: vec!["1".to_string(), "3".to_string()],
        },
    )
    .unwrap_err();
    assert!(err.contains("이어지지 않은 교시"), "{err}");
}

#[test]
fn 없는_기록을_고치거나_지우면_오류다() {
    let f = fixture();
    let missing = 9999;

    let err = delete_span_impl(&f.conn, missing).unwrap_err();
    assert!(err.contains("출결 기록을 찾을 수 없습니다"), "{err}");
    assert!(set_span_memo_impl(&f.conn, missing, "메모").is_err());
    assert!(set_span_tag_impl(&f.conn, missing, None).is_err());
    assert!(edit_span_impl(
        &f.conn,
        &SpanEdit {
            span_id: missing,
            reason_id: None,
            type_id: None,
            slots: vec![],
        }
    )
    .is_err());
}

#[test]
fn 없는_학생에게는_찍을_수_없다() {
    let f = fixture();
    let err = stamp_span_impl(&f.conn, &stamp(9999, "2026-09-10", None, None, &[])).unwrap_err();
    assert!(err.contains("학생을 찾을 수 없습니다"), "{err}");
}

#[test]
fn 날짜_형식이_틀리면_거절한다() {
    let f = fixture();
    let err =
        stamp_span_impl(&f.conn, &stamp(f.students[0], "2026/09/10", None, None, &[])).unwrap_err();
    assert!(err.contains("날짜 형식"), "{err}");
}

#[test]
fn 메모와_태그는_따로_고친다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "출석인정", "결석");
    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[]))
        .unwrap();
    let id = out.span_ids[0];
    let tag = tag_id(&f.conn, "체험학습");

    set_span_memo_impl(&f.conn, id, "가족 여행 확인서 받기로 함").unwrap();
    set_span_tag_impl(&f.conn, id, Some(tag)).unwrap();

    let rows = spans_of(&f, f.students[0]);
    assert_eq!(rows[0].memo, "가족 여행 확인서 받기로 함");
    assert_eq!(rows[0].tag_id, Some(tag));
    assert_eq!(rows[0].tag_name.as_deref(), Some("체험학습"));

    // 태그는 뗄 수 있다. 한 건에 태그는 하나뿐이다.
    set_span_tag_impl(&f.conn, id, None).unwrap();
    let rows = spans_of(&f, f.students[0]);
    assert!(rows[0].tag_id.is_none());
}

#[test]
fn 없는_태그를_붙이면_오류다() {
    let f = fixture();
    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", None, None, &[]))
        .unwrap();
    assert!(set_span_tag_impl(&f.conn, out.span_ids[0], Some(9999)).is_err());
}

// ── 트랜잭션 ──────────────────────────────────────────────────

#[test]
fn 실패한_저장은_되돌려지고_다음_저장을_막지_않는다() {
    let f = fixture();
    let kind = Some(type_id(&f.conn, "결과"));

    // 없는 구분을 참조하면 저장이 실패한다.
    let err = stamp_span_impl(
        &f.conn,
        &stamp(f.students[0], "2026-09-10", Some(9999), kind, &["1", "3"]),
    )
    .unwrap_err();
    assert!(!err.is_empty());
    assert_eq!(span_count(&f.conn), 0, "실패한 저장은 한 건도 남지 않는다");

    // 트랜잭션이 열린 채 남았다면 다음 저장이 통째로 실패한다.
    let (reason, _) = axes(&f.conn, "질병", "결과");
    stamp_span_impl(
        &f.conn,
        &stamp(f.students[0], "2026-09-10", reason, kind, &["1", "3"]),
    )
    .unwrap();
    assert_eq!(span_count(&f.conn), 2);
}

// ── 하루치 격자 ───────────────────────────────────────────────

#[test]
fn 격자는_구간이_없어도_행을_낸다() {
    let f = fixture();
    let grid = get_day_grid_impl(&f.conn, f.school, f.year, 3, 6, "2026-09-10").unwrap();

    assert_eq!(grid.rows.len(), 3);
    assert!(grid.rows.iter().all(|r| r.spans.is_empty()));
    assert_eq!(grid.max_slot, 7);
    assert_eq!(grid.date_label, "2026.09.10.(목)");
    assert!(grid.spans.is_empty());
}

#[test]
fn 격자는_전출한_학생을_뺀다() {
    let f = fixture();
    f.conn
        .execute(
            "UPDATE student SET enrolled_to = '2026-09-01' WHERE id = ?1",
            params![f.students[1]],
        )
        .unwrap();

    let after = get_day_grid_impl(&f.conn, f.school, f.year, 3, 6, "2026-09-10").unwrap();
    assert_eq!(after.rows.len(), 2);
    assert!(after.rows.iter().all(|r| r.student_id != f.students[1]));

    // 전출 전날에는 아직 우리 반이다.
    let before = get_day_grid_impl(&f.conn, f.school, f.year, 3, 6, "2026-08-31").unwrap();
    assert_eq!(before.rows.len(), 3);
}

#[test]
fn 격자는_그날_구간을_평평하게_담는다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[])).unwrap();
    stamp_span_impl(&f.conn, &stamp(f.students[2], "2026-09-10", reason, kind, &[])).unwrap();
    // 다른 날은 섞이지 않는다.
    stamp_span_impl(&f.conn, &stamp(f.students[1], "2026-09-11", reason, kind, &[])).unwrap();

    let grid = get_day_grid_impl(&f.conn, f.school, f.year, 3, 6, "2026-09-10").unwrap();
    assert_eq!(grid.spans.len(), 2);
    assert_eq!(grid.rows[0].spans.len(), 1);
    assert!(grid.rows[1].spans.is_empty());
    assert_eq!(grid.rows[2].spans.len(), 1);
    assert_eq!(grid.spans[0].name, "김하나");
    assert_eq!(grid.spans[0].code_label.as_deref(), Some("질병결석"));
}

#[test]
fn 코드는_두_축이_다_있을_때만_붙는다() {
    let f = fixture();
    let (reason, _) = axes(&f.conn, "질병", "결석");
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, None, &[])).unwrap();

    let rows = spans_of(&f, f.students[0]);
    assert_eq!(rows[0].reason_label.as_deref(), Some("질병"));
    assert!(rows[0].type_label.is_none());
    assert!(rows[0].code_label.is_none());
    assert!(!rows[0].complete);
}

// ── 월별 기록 ─────────────────────────────────────────────────

#[test]
fn 월별_기록은_날짜별로_묶이고_최신이_먼저다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-06-01", reason, kind, &[])).unwrap();
    stamp_span_impl(&f.conn, &stamp(f.students[1], "2026-06-15", reason, kind, &[])).unwrap();
    stamp_span_impl(&f.conn, &stamp(f.students[2], "2026-06-15", reason, kind, &[])).unwrap();
    // 다른 달은 섞이지 않는다.
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-07-01", reason, kind, &[])).unwrap();

    let log = get_month_log_impl(&f.conn, f.school, f.year, 3, 6, 2026, 6).unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].date, "2026-06-15");
    assert_eq!(log[0].spans.len(), 2);
    assert_eq!(log[0].enrolled, 3);
    assert_eq!(log[0].date_label, "2026.06.15.(월)");
    assert_eq!(log[1].date, "2026-06-01");
    assert_eq!(log[1].spans.len(), 1);
}

#[test]
fn 월별_기록의_재학_인원은_그날_기준이다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-06-15", reason, kind, &[])).unwrap();
    f.conn
        .execute(
            "UPDATE student SET enrolled_to = '2026-06-10' WHERE id = ?1",
            params![f.students[2]],
        )
        .unwrap();

    let log = get_month_log_impl(&f.conn, f.school, f.year, 3, 6, 2026, 6).unwrap();
    assert_eq!(log[0].enrolled, 2);
}

#[test]
fn 없는_달은_거절한다() {
    let f = fixture();
    let err = get_month_log_impl(&f.conn, f.school, f.year, 3, 6, 2026, 13).unwrap_err();
    assert!(err.contains("연월이 올바르지 않습니다"), "{err}");
}

// ── 여러 날 일괄 입력 ─────────────────────────────────────────

#[test]
fn 미리보기는_주말과_휴업일을_뺀다() {
    let f = fixture();
    // 2026-06-01은 월요일이다.
    let days = preview_bulk_impl(&f.conn, f.school, None, "2026-06-01", "2026-06-07").unwrap();
    assert_eq!(days.len(), 5);
    assert_eq!(days[0].date, "2026-06-01");
    assert_eq!(days[0].label, "2026.06.01.(월)");
    assert_eq!(days[4].date, "2026-06-05");

    add_off_day(&f.conn, f.school, "2026-06-03");
    let days = preview_bulk_impl(&f.conn, f.school, None, "2026-06-01", "2026-06-07").unwrap();
    assert_eq!(days.len(), 4);
    assert!(days.iter().all(|d| d.date != "2026-06-03"));
}

#[test]
fn 미리보기는_이미_기록이_있는_날을_알린다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-06-02", reason, kind, &[])).unwrap();

    let days = preview_bulk_impl(&f.conn, f.school, None, "2026-06-01", "2026-06-05").unwrap();
    assert!(!days[0].has_existing);
    assert!(days[1].has_existing);
}

#[test]
fn 미리보기는_그_학생의_기록만_알린다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-06-02", reason, kind, &[])).unwrap();

    // 찍은 학생에게는 표시가 붙는다.
    let mine =
        preview_bulk_impl(&f.conn, f.school, Some(f.students[0]), "2026-06-01", "2026-06-05")
            .unwrap();
    assert!(mine[1].has_existing);

    // 다른 학생에게는 붙지 않는다. 학급 누구든 하나 걸리면 되는 조건으로 세면
    // 학기 중 거의 모든 날에 표시가 붙어 정작 겹치는 날이 눈에 들어오지 않는다.
    let other =
        preview_bulk_impl(&f.conn, f.school, Some(f.students[1]), "2026-06-01", "2026-06-05")
            .unwrap();
    assert!(other.iter().all(|d| !d.has_existing));
}

#[test]
fn 없는_학교로는_미리볼_수_없다() {
    let f = fixture();
    let err = preview_bulk_impl(&f.conn, 9999, None, "2026-06-01", "2026-06-05").unwrap_err();
    assert!(err.contains("학교를 찾을 수 없습니다"), "{err}");
}

#[test]
fn 끝_날짜가_앞서면_거절한다() {
    let f = fixture();
    let err = preview_bulk_impl(&f.conn, f.school, None, "2026-06-05", "2026-06-01").unwrap_err();
    assert!(err.contains("끝 날짜"), "{err}");
}

#[test]
fn 일괄_입력은_묶음을_결정적으로_묶는다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "출석인정", "결석");
    let input = stamp(f.students[0], "2026-06-01", reason, kind, &[]);

    let first = apply_bulk_impl(&f.conn, &input, "2026-06-01", "2026-06-05").unwrap();
    assert_eq!(first.days, 5);
    assert_eq!(span_count(&f.conn), 5);

    let rows = spans_of(&f, f.students[0]);
    assert!(rows.iter().all(|r| r.group_id.as_deref() == Some(first.group_id.as_str())));
    assert!(rows.iter().all(|r| r.doc_due.is_some()));

    // 같은 입력이면 묶음 이름도 같고, 이미 넣은 날은 다시 넣지 않는다.
    let again = apply_bulk_impl(&f.conn, &input, "2026-06-01", "2026-06-05").unwrap();
    assert_eq!(again.group_id, first.group_id);
    assert_eq!(again.days, 0);
    assert_eq!(span_count(&f.conn), 5);
}

#[test]
fn 일괄_입력도_주말과_휴업일을_건너뛴다() {
    let f = fixture();
    add_off_day(&f.conn, f.school, "2026-06-03");
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    let input = stamp(f.students[0], "2026-06-01", reason, kind, &[]);

    let out = apply_bulk_impl(&f.conn, &input, "2026-06-01", "2026-06-07").unwrap();
    assert_eq!(out.days, 4);

    let dates: Vec<String> = spans_of(&f, f.students[0])
        .into_iter()
        .map(|r| r.date)
        .collect();
    assert!(!dates.contains(&"2026-06-03".to_string()));
    assert!(!dates.contains(&"2026-06-06".to_string()));
}

#[test]
fn 일괄_입력도_기간_규칙을_그대로_따른다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결과");
    let input = stamp(f.students[0], "2026-06-01", reason, kind, &["1", "3"]);

    let out = apply_bulk_impl(&f.conn, &input, "2026-06-01", "2026-06-02").unwrap();
    assert_eq!(out.days, 2);
    // 하루에 두 구간씩이다 — 1교시와 3교시는 이어지지 않았다.
    assert_eq!(span_count(&f.conn), 4);
}

// ── 문구 (순수 함수) ──────────────────────────────────────────

#[test]
fn 구간_문구는_종류가_묻는_쪽을_따른다() {
    assert_eq!(span_text(Some("none"), Some("조회"), Some("종례")), "하루 종일");
    assert_eq!(span_text(Some("end"), Some("조회"), Some("2")), "조회부터 2교시까지");
    assert_eq!(span_text(Some("start"), Some("5"), Some("종례")), "5교시부터 종례까지");
    assert_eq!(span_text(Some("multi"), Some("3"), Some("3")), "3교시");
    // 나이스 실파일에 결시교시가 `조회,` 하나뿐인 지각이 있었다. 두 끝이 같으면
    // 종류와 무관하게 슬롯 하나로 적는다 — "조회부터 조회까지"는 말이 되지 않는다.
    assert_eq!(span_text(Some("end"), Some("조회"), Some("조회")), "조회");
    assert_eq!(span_text(Some("start"), Some("종례"), Some("종례")), "종례");
    assert_eq!(span_text(Some("multi"), Some("1"), Some("3")), "1교시부터 3교시까지");
}

#[test]
fn 구간_문구는_열린_쪽을_물음표로_적는다() {
    assert_eq!(span_text(None, None, None), "기간 미정");
    assert_eq!(span_text(Some("multi"), Some("3"), None), "3교시부터 ?까지");
    assert_eq!(span_text(Some("multi"), None, Some("3")), "?부터 3교시까지");
    // 종류가 하루 전체면 기간을 묻지 않으므로 열린 쪽이 없다.
    assert_eq!(span_text(Some("none"), None, None), "하루 종일");
}

#[test]
fn 구간_묶기는_이어진_것끼리만_묶는다() {
    let picked = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    assert_eq!(
        ranges_for(Some("multi"), &picked(&["1", "2", "3"]), 7).unwrap(),
        vec![(Some("1".to_string()), Some("3".to_string()))]
    );
    assert_eq!(ranges_for(Some("multi"), &picked(&["1", "3", "5"]), 7).unwrap().len(), 3);
    // 조회와 1교시는 이어져 있다.
    assert_eq!(
        ranges_for(Some("multi"), &picked(&["조회", "1"]), 7).unwrap(),
        vec![(Some("조회".to_string()), Some("1".to_string()))]
    );
    // 결석은 고른 교시와 무관하게 하루 전체다.
    assert_eq!(
        ranges_for(Some("none"), &picked(&["3"]), 7).unwrap(),
        vec![(Some("조회".to_string()), Some("종례".to_string()))]
    );
    // 종류를 아직 안 정했고 교시도 안 골랐으면 기간 미정이다.
    assert_eq!(ranges_for(None, &[], 7).unwrap(), vec![(None, None)]);

    // `?`는 **묻고 있는 쪽이 열려 있다**는 뜻이고, 저장은 NULL이다(`slots.rs`).
    // 이 길이 없으면 화면의 `?` 버튼이 눌러도 언제나 실패한다.
    assert_eq!(
        ranges_for(Some("end"), &picked(&["?"]), 7).unwrap(),
        vec![(Some("조회".to_string()), None)],
        "지각은 조회부터 언제까지인지 모르는 것이다"
    );
    assert_eq!(
        ranges_for(Some("start"), &picked(&["?"]), 7).unwrap(),
        vec![(None, Some("종례".to_string()))],
        "조퇴는 언제부터인지 모르는 채 종례까지다"
    );
    // 종류가 미정이면 어느 쪽을 묻는지 정해지지 않아 저장할 수 없다.
    assert!(ranges_for(None, &picked(&["?"]), 7).is_err());
    assert!(ranges_for(Some("multi"), &picked(&["?"]), 7).is_err());
}

// ── 날짜 표기 ─────────────────────────────────────────────────
//
// chrono는 `2026-9-10`처럼 자리를 채우지 않은 표기도 받아 준다. 저장이 그것을 그대로
// 담으면 날짜로 거르는 모든 화면에서 그 행이 사라진다 — 비교가 문자열 비교라
// `2026-9-10`은 `2026-09-30`보다 뒤로 읽히고 `LIKE '2026-09%'`에도 걸리지 않는다.

#[test]
fn 저장되는_날짜는_자리를_채운_ISO다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    let out = stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-9-10", reason, kind, &[]))
        .unwrap();

    let stored: String = f
        .conn
        .query_row(
            "SELECT date FROM absence_span WHERE id = ?1",
            params![out.span_ids[0]],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stored, "2026-09-10");

    // 자리를 채운 날짜로 찾는 화면에 그대로 나온다.
    let grid = get_day_grid_impl(&f.conn, f.school, f.year, 3, 6, "2026-09-10").unwrap();
    assert_eq!(grid.spans.len(), 1);
    // 월별 기록의 기간 조건에도 걸린다.
    let log = get_month_log_impl(&f.conn, f.school, f.year, 3, 6, 2026, 9).unwrap();
    assert_eq!(log.len(), 1);
}

#[test]
fn 무르기는_날짜_표기에_흔들리지_않는다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");

    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[])).unwrap();
    // 같은 날을 다른 표기로 다시 찍어도 같은 건이므로 취소다.
    let again =
        stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-9-10", reason, kind, &[])).unwrap();
    assert_eq!(again.action, "cancelled");
    assert_eq!(span_count(&f.conn), 0);
}

#[test]
fn 격자는_자리를_채우지_않은_날짜도_같은_날로_본다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "질병", "결석");
    stamp_span_impl(&f.conn, &stamp(f.students[0], "2026-09-10", reason, kind, &[])).unwrap();

    let grid = get_day_grid_impl(&f.conn, f.school, f.year, 3, 6, "2026-9-10").unwrap();
    assert_eq!(grid.date, "2026-09-10");
    assert_eq!(grid.spans.len(), 1);
    assert_eq!(grid.rows.len(), 3);
}

#[test]
fn 묶음_이름은_날짜_표기에_흔들리지_않는다() {
    let f = fixture();
    let (reason, kind) = axes(&f.conn, "출석인정", "결석");
    let input = stamp(f.students[0], "2026-06-01", reason, kind, &[]);

    let first = apply_bulk_impl(&f.conn, &input, "2026-6-1", "2026-6-5").unwrap();
    assert_eq!(first.days, 5);

    // 같은 기간을 자리를 채워 적어도 같은 묶음이고, 다시 넣지 않는다.
    let again = apply_bulk_impl(&f.conn, &input, "2026-06-01", "2026-06-05").unwrap();
    assert_eq!(again.group_id, first.group_id);
    assert_eq!(again.days, 0);
    assert_eq!(span_count(&f.conn), 5);
}
