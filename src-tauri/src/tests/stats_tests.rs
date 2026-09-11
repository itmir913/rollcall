//! 한도 집계 테스트.
//!
//! 이 모듈은 읽기만 한다 — 규정을 넘겼다고 저장을 막거나 기록을 고치지 않는다.
//! 그래서 롤백 경로가 없고, 대신 **세는 규칙**과 **태그 누락 목록**을 촘촘히 본다.

use crate::commands::stats::get_quota_reports_impl;
use crate::tests::*;
use crate::types::QuotaReport;
use rusqlite::Connection;

// ── 도우미 ────────────────────────────────────────────────────

fn add_span(
    conn: &Connection,
    class_id: i64,
    student_id: i64,
    date: &str,
    reason: Option<i64>,
    type_id: Option<i64>,
    tag: Option<i64>,
) -> i64 {
    add_span_slots(
        conn, class_id, student_id, date, reason, type_id, tag, None, None,
    )
}

#[allow(clippy::too_many_arguments)]
fn add_span_slots(
    conn: &Connection,
    class_id: i64,
    student_id: i64,
    date: &str,
    reason: Option<i64>,
    type_id: Option<i64>,
    tag: Option<i64>,
    start: Option<&str>,
    end: Option<&str>,
) -> i64 {
    conn.execute(
        "INSERT INTO absence_span
             (class_id, student_id, date, reason_id, type_id, tag_id, start_slot, end_slot)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![class_id, student_id, date, reason, type_id, tag, start, end],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[allow(clippy::too_many_arguments)]
fn add_rule(
    conn: &Connection,
    name: &str,
    tag: Option<i64>,
    reason: Option<i64>,
    type_id: Option<i64>,
    period: &str,
    limit_n: i64,
    unit: &str,
) -> i64 {
    let school = school_id(conn);
    conn.execute(
        "INSERT INTO quota_rule
             (school_id, name, tag_id, reason_id, type_id, period, limit_n, unit, sort_order, valid_from)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 100, '1900-01-01')",
        rusqlite::params![school, name, tag, reason, type_id, period, limit_n, unit],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 규정 하나만 골라 집계한다.
fn report(conn: &Connection, class_id: i64, rule: i64) -> QuotaReport {
    let mut all = get_quota_reports_impl(conn, class_id, Some(rule), None, None).unwrap();
    assert_eq!(all.len(), 1, "규정을 지정했으면 보고서도 하나다");
    all.remove(0)
}

fn used_of(report: &QuotaReport, student_id: i64) -> i64 {
    report
        .rows
        .iter()
        .find(|r| r.student_id == student_id)
        .unwrap_or_else(|| panic!("학생 행이 없습니다: {student_id}"))
        .used
}

fn state_of(report: &QuotaReport, student_id: i64) -> String {
    report
        .rows
        .iter()
        .find(|r| r.student_id == student_id)
        .unwrap()
        .state
        .clone()
}

// ── 날짜 단위 vs 건수 단위 ────────────────────────────────────

#[test]
fn 하루에_두_건이어도_날짜_단위에서는_하루다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (r, t_absent) = axes(&conn, "출석인정", "결석");
    let (_, t_early) = axes(&conn, "출석인정", "조퇴");

    add_span(&conn, class, s, "2026-05-11", r, t_absent, Some(tag));
    add_span(&conn, class, s, "2026-05-11", r, t_early, Some(tag));

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(used_of(&rep, s), 1, "같은 날 두 건이어도 1일이다");
    let row = rep.rows.iter().find(|x| x.student_id == s).unwrap();
    assert_eq!(row.dates, vec!["2026-05-11".to_string()]);
    assert_eq!(rep.used_total, 1);
}

#[test]
fn 하루에_두_건이면_건수_단위에서는_두_건이다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (r, t_absent) = axes(&conn, "출석인정", "결석");
    let (_, t_early) = axes(&conn, "출석인정", "조퇴");

    add_span(&conn, class, s, "2026-05-11", r, t_absent, Some(tag));
    add_span(&conn, class, s, "2026-05-11", r, t_early, Some(tag));

    let rule = add_rule(&conn, "체험학습 연 20건", Some(tag), None, None, "year", 20, "count");
    let rep = report(&conn, class, rule);

    assert_eq!(used_of(&rep, s), 2, "건수 단위는 구간마다 센다");
    let row = rep.rows.iter().find(|x| x.student_id == s).unwrap();
    assert!(row.dates.is_empty(), "건수 단위에서는 날짜 칸을 쓰지 않는다");
}

// ── 태그로 센다 ───────────────────────────────────────────────

#[test]
fn 태그가_다르면_세지_않는다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let trip = tag_id(&conn, "체험학습");
    let cramps = tag_id(&conn, "생리통");
    let (r, t) = axes(&conn, "출석인정", "조퇴");

    add_span(&conn, class, s, "2026-05-11", r, t, Some(cramps));

    let rule = add_rule(&conn, "체험학습 연 20일", Some(trip), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(used_of(&rep, s), 0, "다른 태그가 붙은 구간은 이 규정이 세지 않는다");
}

#[test]
fn 결석과_조퇴가_같은_태그면_함께_센다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (r, t_absent) = axes(&conn, "출석인정", "결석");
    let (_, t_early) = axes(&conn, "출석인정", "조퇴");

    // 체험학습은 출석인정 결석으로도, 출석인정 조퇴로도 나간다.
    add_span(&conn, class, s, "2026-05-11", r, t_absent, Some(tag));
    add_span(&conn, class, s, "2026-05-12", r, t_early, Some(tag));

    let by_tag = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    assert_eq!(
        used_of(&report(&conn, school, by_tag), s),
        2,
        "태그로 세면 결석과 조퇴가 한 규정에 함께 잡힌다"
    );

    // 구분 × 종류 조합으로 세면 한쪽을 빠뜨린다 — 태그로 세는 이유다.
    let by_axes = add_rule(&conn, "출석인정 조퇴", None, r, t_early, "year", 20, "day");
    assert_eq!(used_of(&report(&conn, school, by_axes), s), 1);
}

#[test]
fn 구분_조건이_있으면_그_조건까지_맞아야_센다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "생리통");
    let sick = reason_id(&conn, "질병");
    let recognized = reason_id(&conn, "출석인정");
    let early = type_id(&conn, "조퇴");

    add_span(&conn, class, s, "2026-05-11", Some(sick), Some(early), Some(tag));
    add_span(&conn, class, s, "2026-05-12", Some(recognized), Some(early), Some(tag));

    let rule = add_rule(
        &conn,
        "질병 생리통 조퇴",
        Some(tag),
        Some(sick),
        Some(early),
        "year",
        10,
        "day",
    );
    assert_eq!(used_of(&report(&conn, class, rule), s), 1);
}

#[test]
fn 태그도_구분도_없는_규정은_모든_구간을_센다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (r, t) = axes(&conn, "질병", "결석");

    add_span(&conn, class, s, "2026-05-11", r, t, None);
    add_span(&conn, class, s, "2026-05-12", r, t, Some(tag));

    let rule = add_rule(&conn, "전체", None, None, None, "year", 30, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(used_of(&rep, s), 2);
    assert!(
        rep.untagged.is_empty(),
        "태그를 지정하지 않은 규정에는 빠진 태그라는 개념이 없다"
    );
}

// ── 기간 단위 ─────────────────────────────────────────────────

#[test]
fn 달_단위는_한_달에_두_번이면_넘긴_것이다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "생리통");
    let (r, t) = axes(&conn, "질병", "조퇴");

    add_span(&conn, class, s, "2026-06-03", r, t, Some(tag));
    add_span(&conn, class, s, "2026-06-17", r, t, Some(tag));

    let rule = add_rule(&conn, "생리통 월 1회", Some(tag), None, None, "month", 1, "count");
    let rep = report(&conn, class, rule);
    let row = rep.rows.iter().find(|x| x.student_id == s).unwrap();

    assert_eq!(row.used, 2, "used는 가장 많이 쓴 달의 수다");
    assert_eq!(row.state, "over");
    assert_eq!(rep.over_count, 1);

    // 달 칸은 3월부터 2월 순서다.
    assert_eq!(row.buckets.first().unwrap().key, "2026-03");
    assert_eq!(row.buckets.first().unwrap().label, "3월");
    assert_eq!(row.buckets.last().unwrap().key, "2027-02");
    assert_eq!(row.buckets.last().unwrap().label, "2월");
    let june = row.buckets.iter().find(|b| b.key == "2026-06").unwrap();
    assert_eq!(june.count, 2);
    let may = row.buckets.iter().find(|b| b.key == "2026-05").unwrap();
    assert_eq!(may.count, 0);
}

#[test]
fn 달이_다르면_각각_센다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "생리통");
    let (r, t) = axes(&conn, "질병", "조퇴");

    add_span(&conn, class, s, "2026-06-03", r, t, Some(tag));
    add_span(&conn, class, s, "2026-07-03", r, t, Some(tag));

    let rule = add_rule(&conn, "생리통 월 1회", Some(tag), None, None, "month", 2, "count");
    let rep = report(&conn, class, rule);

    assert_eq!(used_of(&rep, s), 1, "달마다 따로 세므로 최대 월 사용 수는 1이다");
    assert_eq!(state_of(&rep, s), "ok");
}

#[test]
fn 학년도_밖의_날짜는_세지_않는다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (r, t) = axes(&conn, "출석인정", "결석");

    add_span(&conn, class, s, "2027-01-15", r, t, Some(tag)); // 2026학년도
    add_span(&conn, class, s, "2027-02-28", r, t, Some(tag)); // 2026학년도 마지막 날
    add_span(&conn, class, s, "2027-03-02", r, t, Some(tag)); // 2027학년도 — 세지 않는다
    add_span(&conn, class, s, "2026-02-20", r, t, Some(tag)); // 2025학년도 — 세지 않는다

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(used_of(&rep, s), 2);
}

/// **세지 않은 것을 조용히 넘기지 않는다.** 학년도는 기준 연도일 뿐 날짜 울타리가
/// 아니라, 2026학년도 학급에 2027-03-05를 입력하는 일이 실제로 있다. 그 건이 한도에서
/// 빠진 채 아무 말도 없으면 교사에게 "덜 썼다"고 거짓으로 알리게 된다.
#[test]
fn 학년도_밖이라_세지_않은_건수를_함께_알린다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (r, t) = axes(&conn, "출석인정", "결석");

    add_span(&conn, class, s, "2027-01-15", r, t, Some(tag)); // 2026학년도
    add_span(&conn, class, s, "2027-03-05", r, t, Some(tag)); // 2027학년도 — 창 밖
    add_span(&conn, class, s, "2026-02-20", r, t, Some(tag)); // 2025학년도 — 창 밖

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(used_of(&rep, s), 1, "한도는 학년도 창으로 센다");
    assert_eq!(rep.outside, 2, "창 밖 두 건을 세어서 알린다");
    assert_eq!(rep.window_from, "2026-03-01");
    assert_eq!(rep.window_to, "2027-02-28");
}

#[test]
fn 창_안에만_기록이_있으면_알릴_것이_없다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (r, t) = axes(&conn, "출석인정", "결석");

    add_span(&conn, class, s, "2026-05-11", r, t, Some(tag));

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(used_of(&rep, s), 1);
    assert_eq!(rep.outside, 0);
}

/// 태그 누락 목록도 창으로 잘린다. 그쪽에서 빠진 건도 같이 센다 —
/// 세는 쪽에서 빠지는 것과 목록에서 빠지는 것이 교사에게는 같은 일이다.
#[test]
fn 창_밖의_태그_누락_건도_세어서_알린다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (r, t) = axes(&conn, "출석인정", "결석");

    add_span(&conn, class, s, "2027-03-05", r, t, None); // 태그 없음 + 창 밖

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(used_of(&rep, s), 0);
    assert!(rep.untagged.is_empty(), "창 밖이라 목록에도 없다");
    assert_eq!(rep.outside, 1, "그래서 세어서 알린다");
}

#[test]
fn from_to로_창을_좁힌다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (r, t) = axes(&conn, "출석인정", "결석");

    add_span(&conn, class, s, "2026-05-11", r, t, Some(tag));
    add_span(&conn, class, s, "2026-06-11", r, t, Some(tag));
    add_span(&conn, class, s, "2026-07-11", r, t, Some(tag));

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = get_quota_reports_impl(
        &conn,
        class,
        Some(rule),
        Some("2026-06-01"),
        Some("2026-06-30"),
    )
    .unwrap()
    .remove(0);

    assert_eq!(used_of(&rep, s), 1);
    // 교사가 좁힌 창도 마찬가지다 — 밖으로 나간 두 건을 세어서 알린다.
    assert_eq!(rep.outside, 2);
    assert_eq!(rep.window_from, "2026-06-01");
    assert_eq!(rep.window_to, "2026-06-30");
}

// ── 상태 ──────────────────────────────────────────────────────

#[test]
fn 한도의_80퍼센트부터_near다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (r, t) = axes(&conn, "출석인정", "결석");
    let rule = add_rule(&conn, "체험학습 연 10일", Some(tag), None, None, "year", 10, "day");

    let days = [
        "2026-05-01",
        "2026-05-02",
        "2026-05-03",
        "2026-05-04",
        "2026-05-05",
        "2026-05-06",
        "2026-05-07",
    ];
    for d in days {
        add_span(&conn, class, s, d, r, t, Some(tag));
    }
    let rep = report(&conn, class, rule);
    assert_eq!(used_of(&rep, s), 7);
    assert_eq!(state_of(&rep, s), "ok", "70%는 아직 ok다");
    assert_eq!(rep.near_count, 0);

    add_span(&conn, class, s, "2026-05-08", r, t, Some(tag));
    let rep = report(&conn, class, rule);
    assert_eq!(state_of(&rep, s), "near", "80%부터 near다");
    assert_eq!(rep.near_count, 1);

    add_span(&conn, class, s, "2026-05-11", r, t, Some(tag));
    add_span(&conn, class, s, "2026-05-12", r, t, Some(tag));
    let rep = report(&conn, class, rule);
    assert_eq!(used_of(&rep, s), 10);
    assert_eq!(state_of(&rep, s), "over", "한도에 도달하면 over다");
    assert_eq!(rep.over_count, 1);
    assert_eq!(rep.near_count, 0);
}

#[test]
fn 기록이_없는_학생도_행으로_나온다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let a = enroll(&conn, class, school, 1, "김하나");
    let b = enroll(&conn, class, school, 2, "이두리");
    let tag = tag_id(&conn, "체험학습");
    let (r, t) = axes(&conn, "출석인정", "결석");
    add_span(&conn, class, a, "2026-05-11", r, t, Some(tag));

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(rep.rows.len(), 2);
    assert_eq!(rep.rows[0].number, 1);
    assert_eq!(rep.rows[1].number, 2);
    assert_eq!(used_of(&rep, b), 0);
    assert_eq!(state_of(&rep, b), "ok");
}

// ── 태그 누락 ─────────────────────────────────────────────────

#[test]
fn 태그가_빠진_구간을_따로_모은다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (recognized, absent) = axes(&conn, "출석인정", "결석");
    let (sick, _) = axes(&conn, "질병", "결석");

    add_span(&conn, class, s, "2026-05-11", recognized, absent, Some(tag)); // 태그 있음
    let missing = add_span(&conn, class, s, "2026-05-12", recognized, absent, None); // 태그 빠짐
    add_span(&conn, class, s, "2026-05-13", sick, absent, None); // 질병은 목록에 넣지 않는다

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(used_of(&rep, s), 1);
    assert_eq!(rep.untagged.len(), 1, "태그가 빠진 건을 조용히 넘기지 않는다");
    let u = &rep.untagged[0];
    assert_eq!(u.id, missing);
    assert_eq!(u.date, "2026-05-12");
    assert_eq!(u.date_label, "2026.05.12.(화)");
    assert_eq!(u.number, 1);
    assert!(u.tag_id.is_none());
    assert!(u.complete, "두 축이 다 채워졌다");
    assert_eq!(u.code_label.as_deref(), Some("출석인정결석"));
}

#[test]
fn 추가_조건이_있으면_그_조건에_맞는_것만_태그_누락으로_본다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "생리통");
    let sick = reason_id(&conn, "질병");
    let recognized = reason_id(&conn, "출석인정");
    let early = type_id(&conn, "조퇴");

    let missing = add_span(&conn, class, s, "2026-05-11", Some(sick), Some(early), None);
    add_span(&conn, class, s, "2026-05-12", Some(recognized), Some(early), None);

    let rule = add_rule(
        &conn,
        "질병 생리통 조퇴 월 1회",
        Some(tag),
        Some(sick),
        Some(early),
        "month",
        1,
        "count",
    );
    let rep = report(&conn, class, rule);

    assert_eq!(rep.untagged.len(), 1);
    assert_eq!(rep.untagged[0].id, missing);
}

#[test]
fn 미완성_기록과_열린_구간도_태그_누락_목록에_들어간다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let recognized = reason_id(&conn, "출석인정");

    // 종류가 아직 비어 있고 시작 교시도 열려 있는 기록. 프로그램은 판정하지 않는다.
    add_span_slots(
        &conn,
        class,
        s,
        "2026-05-12",
        Some(recognized),
        None,
        None,
        None,
        Some("3"),
    );

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(rep.untagged.len(), 1);
    let u = &rep.untagged[0];
    assert!(!u.complete, "종류가 비었으므로 미완성이다");
    assert!(u.type_id.is_none());
    assert!(u.code_label.is_none(), "두 축이 다 결정되어야 코드가 붙는다");
    assert_eq!(u.span_text, "?부터 3교시까지", "열린 쪽은 물음표로 적는다");
}

#[test]
fn 구간_표기는_다른_화면과_같은_문장이다() {
    // 태그 누락 목록은 `attendance::load_spans_on`이 읽는다. 여기서 질의를 따로
    // 만들면 같은 기록이 출결 기록과 통계에서 서로 다른 문장으로 보인다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let (recognized, absent) = axes(&conn, "출석인정", "결석");

    // 결석은 기간을 묻지 않는다(slot_prompt = none) — 조회~종례로 저장되고
    // 화면에는 "하루 종일"로 나온다.
    add_span_slots(
        &conn,
        class,
        s,
        "2026-05-12",
        recognized,
        absent,
        None,
        Some("조회"),
        Some("종례"),
    );
    // 기간을 아직 안 정한 건.
    add_span(&conn, class, s, "2026-05-13", recognized, None, None);

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(rep.untagged.len(), 2);
    assert_eq!(rep.untagged[0].span_text, "하루 종일");
    assert_eq!(rep.untagged[1].span_text, "기간 미정");
}

#[test]
fn 겹치는_구간은_막지_않고_표시만_한다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let s = enroll(&conn, class, school, 1, "김하나");
    let tag = tag_id(&conn, "체험학습");
    let recognized = reason_id(&conn, "출석인정");
    let result_type = type_id(&conn, "결과");

    add_span_slots(
        &conn,
        class,
        s,
        "2026-05-12",
        Some(recognized),
        Some(result_type),
        None,
        Some("1"),
        Some("3"),
    );
    add_span_slots(
        &conn,
        class,
        s,
        "2026-05-12",
        Some(recognized),
        Some(result_type),
        None,
        Some("3"),
        Some("5"),
    );
    // 겹치지 않는 다른 날의 구간.
    add_span_slots(
        &conn,
        class,
        s,
        "2026-05-13",
        Some(recognized),
        Some(result_type),
        None,
        Some("1"),
        Some("2"),
    );

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(rep.untagged.len(), 3);
    assert!(rep.untagged[0].overlapping);
    assert!(rep.untagged[1].overlapping);
    assert!(!rep.untagged[2].overlapping);
}

// ── 규정 목록 ─────────────────────────────────────────────────

#[test]
fn 마감된_규정은_세지_않는다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    enroll(&conn, class, school, 1, "김하나");
    let retired = quota_rule_id(&conn, "생리통 월 1회");
    conn.execute(
        "UPDATE quota_rule SET valid_to = '2026-08-31' WHERE id = ?1",
        rusqlite::params![retired],
    )
    .unwrap();

    let all = get_quota_reports_impl(&conn, class, None, None, None).unwrap();

    assert!(
        all.iter().all(|r| r.rule.id != retired),
        "마감된 규정은 목록에서 빠진다"
    );
    assert!(all.iter().any(|r| r.rule.name == "체험학습 연 20일"));

    let err = get_quota_reports_impl(&conn, class, Some(retired), None, None).unwrap_err();
    assert!(err.contains("한도 규정을 찾을 수 없습니다"), "{err}");
}

#[test]
fn 규정에_태그_이름이_함께_온다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    enroll(&conn, class, school, 1, "김하나");
    let rule = quota_rule_id(&conn, "체험학습 연 20일");
    let rep = report(&conn, class, rule);

    assert_eq!(rep.rule.tag_name.as_deref(), Some("체험학습"));
    assert_eq!(rep.rule.period, "year");
    assert_eq!(rep.rule.unit, "day");
    assert_eq!(rep.rule.limit_n, 20);
}

// ── 실패 경로 ─────────────────────────────────────────────────

#[test]
fn 없는_학급은_오류다() {
    let conn = setup_test_db();
    let err = get_quota_reports_impl(&conn, 9999, None, None, None).unwrap_err();
    assert!(err.contains("학급을 찾을 수 없습니다"), "{err}");
}

#[test]
fn 교과_강좌로는_담임_통계를_볼_수_없다() {
    // 담임 화면에 교과의 숫자가 한 줄도 나오면 안 된다. 반대도 마찬가지다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let subject = insert_class(&conn, school, "subject", "지구과학Ⅰ", None, None);
    let err = get_quota_reports_impl(&conn, subject, None, None, None).unwrap_err();
    assert!(err.contains("담임 학급이 아닙니다"), "{err}");
}

#[test]
fn 없는_규정은_오류다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let err = get_quota_reports_impl(&conn, class, Some(9999), None, None).unwrap_err();
    assert!(err.contains("한도 규정을 찾을 수 없습니다"), "{err}");
}

#[test]
fn 날짜_형식이_틀리면_오류다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let err = get_quota_reports_impl(&conn, class, None, Some("2026.06.01"), None).unwrap_err();
    assert!(err.contains("날짜 형식이 올바르지 않습니다"), "{err}");
}

// ── 다른 학급은 섞이지 않는다 ─────────────────────────────────

#[test]
fn 다른_학급_학생은_섞이지_않는다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    // **같은 학생을 두 학급에 넣는다.** 학급마다 다른 학생을 쓰면 학적으로 거르던
    // 옛 질의로도 통과해, 이 테스트가 주장하는 것을 확인하지 못한다.
    let mine = enroll(&conn, class, school, 1, "김하나");
    let next_door = insert_class(&conn, school, "homeroom", "3학년 7반", Some(3), Some(7));
    join_class(&conn, next_door, mine);

    let tag = tag_id(&conn, "체험학습");
    let (r, t) = axes(&conn, "출석인정", "결석");
    add_span(&conn, class, mine, "2026-05-11", r, t, Some(tag));
    // 옆 학급에서 같은 학생에게 이틀. 섞이면 한도가 하루가 아니라 사흘로 세어진다.
    add_span(&conn, next_door, mine, "2026-05-11", r, t, Some(tag));
    add_span(&conn, next_door, mine, "2026-05-12", r, t, None);

    let rule = add_rule(&conn, "체험학습 연 20일", Some(tag), None, None, "year", 20, "day");
    let rep = report(&conn, class, rule);

    assert_eq!(rep.rows.len(), 1);
    assert_eq!(rep.rows[0].student_id, mine);
    assert_eq!(rep.rows[0].used, 1, "옆 학급의 이틀이 섞이면 안 된다");
    assert!(rep.untagged.is_empty(), "옆 학급의 태그 없는 건도 섞이지 않는다");
}
