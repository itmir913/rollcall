//! 내보내기 테스트.
//!
//! 표를 만드는 함수는 DTO만 받는 순수 함수라 DB 없이 검사한다. 엑셀이 실제로
//! 열어야 하는 성질(BOM · 인용 · 열 순서)이 여기서 갈리기 때문에, 그 셋을 먼저 본다.

use crate::commands::export::*;
use crate::tests::*;
use crate::types::{QuotaBucket, QuotaReport, QuotaRow, QuotaRuleItem, SpanItem};
use rusqlite::Connection;

// ── 재료 ──────────────────────────────────────────────────────

fn span(id: i64, number: i64, name: &str) -> SpanItem {
    SpanItem {
        id,
        student_id: number,
        number,
        name: name.to_string(),
        date: "2026-09-10".to_string(),
        date_label: "2026.09.10.(목)".to_string(),
        reason_id: None,
        type_id: None,
        reason_label: None,
        type_label: None,
        code_label: None,
        start_slot: None,
        end_slot: None,
        slot_prompt: None,
        span_text: "? ~ ?".to_string(),
        tag_id: None,
        tag_name: None,
        memo: String::new(),
        doc_done: false,
        doc_due: None,
        doc_done_on: None,
        days_overdue: None,
        neis_done: false,
        neis_done_on: None,
        group_id: None,
        complete: false,
        overlapping: false,
    }
}

fn rule(name: &str, period: &str, unit: &str, limit_n: i64) -> QuotaRuleItem {
    QuotaRuleItem {
        id: 1,
        name: name.to_string(),
        tag_id: Some(1),
        tag_name: Some("체험학습".to_string()),
        reason_id: None,
        type_id: None,
        period: period.to_string(),
        limit_n,
        unit: unit.to_string(),
        sort_order: 10,
        valid_from: "1900-01-01".to_string(),
        valid_to: None,
    }
}

fn body(csv: &str) -> Vec<String> {
    csv.trim_start_matches('\u{feff}')
        .lines()
        .map(|l| l.to_string())
        .collect()
}

/// 인용된 값이 섞여 있으므로 열을 세는 데만 쓴다.
fn first_cells(line: &str, n: usize) -> Vec<String> {
    line.split(',').take(n).map(|c| c.to_string()).collect()
}

fn insert_span(
    conn: &Connection,
    student_id: i64,
    date: &str,
    axes: (Option<i64>, Option<i64>),
    memo: &str,
) -> i64 {
    conn.execute(
        "INSERT INTO absence_span (student_id, date, reason_id, type_id, memo)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![student_id, date, axes.0, axes.1, memo],
    )
    .unwrap();
    conn.last_insert_rowid()
}

// ── 인용 ──────────────────────────────────────────────────────

#[test]
fn csv_cell_leaves_plain_text_alone() {
    assert_eq!(csv_cell("김민준"), "김민준");
    assert_eq!(csv_cell(""), "");
}

#[test]
fn csv_cell_quotes_comma_quote_and_newline() {
    assert_eq!(csv_cell("김, 민준"), "\"김, 민준\"");
    assert_eq!(csv_cell("이르길 \"괜찮다\""), "\"이르길 \"\"괜찮다\"\"\"");
    assert_eq!(csv_cell("첫 줄\n둘째 줄"), "\"첫 줄\n둘째 줄\"");
}

#[test]
fn name_with_comma_stays_one_cell() {
    let mut s = span(1, 7, "김, 민준");
    s.memo = "확인서, 다음 주".to_string();
    let csv = build_spans_csv(3, 6, &[s], &Default::default());
    let rows = body(&csv);
    assert!(rows[1].contains("\"김, 민준\""), "{}", rows[1]);
    assert!(rows[1].contains("\"확인서, 다음 주\""), "{}", rows[1]);
}

// ── BOM과 머리글 ──────────────────────────────────────────────

#[test]
fn every_table_starts_with_bom() {
    let report = QuotaReport {
        rule: rule("체험학습 연 20일", "year", "day", 20),
        rows: vec![],
        used_total: 0,
        near_count: 0,
        over_count: 0,
        untagged: vec![],
    };
    for csv in [
        build_spans_csv(3, 6, &[], &Default::default()),
        build_doc_pending_csv(3, 6, "2026-09-10", &[]),
        build_neis_pending_csv(3, 6, &[]),
        build_quota_csv(3, 6, &report),
    ] {
        assert!(csv.starts_with('\u{feff}'), "BOM이 없다: {csv:?}");
    }
}

#[test]
fn empty_list_still_has_header() {
    for csv in [
        build_spans_csv(3, 6, &[], &Default::default()),
        build_doc_pending_csv(3, 6, "2026-09-10", &[]),
        build_neis_pending_csv(3, 6, &[]),
    ] {
        let rows = body(&csv);
        assert_eq!(rows.len(), 1, "머리글만 남아야 한다: {rows:?}");
        assert!(rows[0].starts_with("학년,반,번호,"), "{}", rows[0]);
    }
}

#[test]
fn first_three_columns_are_grade_class_number() {
    let s = span(1, 7, "김민준");
    for csv in [
        build_spans_csv(3, 6, std::slice::from_ref(&s), &Default::default()),
        build_doc_pending_csv(3, 6, "2026-09-10", std::slice::from_ref(&s)),
        build_neis_pending_csv(3, 6, std::slice::from_ref(&s)),
    ] {
        let rows = body(&csv);
        assert_eq!(first_cells(&rows[0], 3), ["학년", "반", "번호"]);
        assert_eq!(first_cells(&rows[1], 3), ["3", "6", "7"]);
    }
}

#[test]
fn quota_table_also_leads_with_class_keys() {
    let report = QuotaReport {
        rule: rule("체험학습 연 20일", "year", "day", 20),
        rows: vec![QuotaRow {
            student_id: 1,
            number: 7,
            name: "김민준".to_string(),
            used: 3,
            limit_n: 20,
            dates: vec!["2026-05-04".to_string(), "2026-05-06".to_string()],
            buckets: vec![],
            state: "ok".to_string(),
        }],
        used_total: 3,
        near_count: 0,
        over_count: 0,
        untagged: vec![],
    };
    let rows = body(&build_quota_csv(3, 6, &report));
    assert_eq!(first_cells(&rows[0], 3), ["학년", "반", "번호"]);
    assert_eq!(first_cells(&rows[1], 3), ["3", "6", "7"]);
}

// ── 값 표기 ───────────────────────────────────────────────────

#[test]
fn unset_axes_read_as_undecided() {
    let rows = body(&build_spans_csv(3, 6, &[span(1, 7, "김민준")], &Default::default()));
    assert!(rows[1].contains("미정"), "{}", rows[1]);
}

#[test]
fn dates_are_written_in_screen_format() {
    let mut s = span(1, 7, "김민준");
    s.doc_due = Some("2026-09-17".to_string());
    let rows = body(&build_doc_pending_csv(3, 6, "2026-09-10", &[s]));
    assert!(rows[1].contains("2026.09.10.(목)"), "{}", rows[1]);
    assert!(rows[1].contains("2026.09.17.(목)"), "{}", rows[1]);
}

#[test]
fn overdue_column_reads_as_a_sentence() {
    let mut past = span(1, 7, "김민준");
    past.doc_due = Some("2026-09-07".to_string());
    let mut today = span(2, 8, "이서연");
    today.doc_due = Some("2026-09-10".to_string());
    let mut ahead = span(3, 9, "박지호");
    ahead.doc_due = Some("2026-09-14".to_string());
    let none = span(4, 10, "최유진");

    let rows = body(&build_doc_pending_csv(
        3,
        6,
        "2026-09-10",
        &[past, today, ahead, none],
    ));
    assert!(rows[1].ends_with("3일 경과"), "{}", rows[1]);
    assert!(rows[2].ends_with("오늘 마감"), "{}", rows[2]);
    assert!(rows[3].ends_with("4일 남음"), "{}", rows[3]);
    // 마감이 없는 건은 재촉할 근거가 없으므로 경과일 칸이 빈다.
    assert!(rows[4].ends_with(','), "{}", rows[4]);
}

#[test]
fn neis_table_flags_incomplete_records() {
    let mut done = span(1, 7, "김민준");
    done.complete = true;
    let waiting = span(2, 8, "이서연");
    let rows = body(&build_neis_pending_csv(3, 6, &[done, waiting]));
    assert!(rows[1].ends_with("완성"), "{}", rows[1]);
    assert!(rows[2].ends_with("미완성"), "{}", rows[2]);
}

#[test]
fn span_table_shows_document_and_neis_state() {
    let mut s = span(1, 7, "김민준");
    s.doc_done = true;
    s.doc_done_on = Some("2026-09-12".to_string());
    s.neis_done = false;
    s.overlapping = true;
    let rows = body(&build_spans_csv(3, 6, &[s], &Default::default()));
    assert!(rows[1].contains("받음"), "{}", rows[1]);
    assert!(rows[1].contains("미등재"), "{}", rows[1]);
    assert!(rows[1].contains("겹침"), "{}", rows[1]);
    assert!(rows[1].contains("2026.09.12."), "{}", rows[1]);
}

// ── 한도 표 ───────────────────────────────────────────────────

#[test]
fn quota_table_lists_used_dates() {
    let report = QuotaReport {
        rule: rule("체험학습 연 20일", "year", "day", 20),
        rows: vec![QuotaRow {
            student_id: 1,
            number: 7,
            name: "김민준".to_string(),
            used: 2,
            limit_n: 20,
            dates: vec!["2026-05-04".to_string(), "2026-05-06".to_string()],
            buckets: vec![],
            state: "ok".to_string(),
        }],
        used_total: 2,
        near_count: 0,
        over_count: 0,
        untagged: vec![],
    };
    let rows = body(&build_quota_csv(3, 6, &report));
    assert!(rows[1].contains("학년도"), "{}", rows[1]);
    assert!(rows[1].contains("일수"), "{}", rows[1]);
    assert!(rows[1].contains("여유"), "{}", rows[1]);
    // 날짜 목록에 쉼표가 들어가므로 한 칸으로 인용돼야 한다.
    assert!(
        rows[1].contains("\"2026.05.04.(월), 2026.05.06.(수)\""),
        "{}",
        rows[1]
    );
}

#[test]
fn quota_table_uses_month_buckets_when_present() {
    let report = QuotaReport {
        rule: rule("생리통 월 1회", "month", "count", 1),
        rows: vec![QuotaRow {
            student_id: 1,
            number: 7,
            name: "김민준".to_string(),
            used: 3,
            limit_n: 1,
            dates: vec!["2026-06-02".to_string()],
            buckets: vec![
                QuotaBucket {
                    key: "2026-06".to_string(),
                    label: "6월".to_string(),
                    count: 2,
                },
                QuotaBucket {
                    key: "2026-07".to_string(),
                    label: "7월".to_string(),
                    count: 1,
                },
            ],
            state: "over".to_string(),
        }],
        used_total: 3,
        near_count: 0,
        over_count: 1,
        untagged: vec![],
    };
    let rows = body(&build_quota_csv(3, 6, &report));
    assert!(rows[1].contains("달"), "{}", rows[1]);
    assert!(rows[1].contains("건수"), "{}", rows[1]);
    assert!(rows[1].contains("초과"), "{}", rows[1]);
    assert!(rows[1].contains("\"6월 2, 7월 1\""), "{}", rows[1]);
}

#[test]
fn untagged_records_are_not_dropped_silently() {
    let mut orphan = span(9, 11, "정하윤");
    orphan.reason_label = Some("출석인정".to_string());
    orphan.type_label = Some("조퇴".to_string());
    let report = QuotaReport {
        rule: rule("체험학습 연 20일", "year", "day", 20),
        rows: vec![],
        used_total: 0,
        near_count: 0,
        over_count: 0,
        untagged: vec![orphan],
    };
    let csv = build_quota_csv(3, 6, &report);
    assert!(csv.contains("태그 없는 기록"), "{csv}");
    assert!(csv.contains("정하윤"), "{csv}");

    let rows = body(&csv);
    // 빈 줄 뒤에 제목 · 머리글 · 값이 이어진다.
    let marker = rows
        .iter()
        .position(|r| r.as_str() == "태그 없는 기록")
        .unwrap();
    assert_eq!(rows[marker - 1], "");
    assert_eq!(first_cells(&rows[marker + 1], 3), ["학년", "반", "번호"]);
    assert_eq!(first_cells(&rows[marker + 2], 3), ["3", "6", "11"]);
}

#[test]
fn quota_table_without_untagged_has_no_second_block() {
    let report = QuotaReport {
        rule: rule("체험학습 연 20일", "year", "day", 20),
        rows: vec![],
        used_total: 0,
        near_count: 0,
        over_count: 0,
        untagged: vec![],
    };
    let csv = build_quota_csv(3, 6, &report);
    assert!(!csv.contains("태그 없는 기록"), "{csv}");
    assert_eq!(body(&csv).len(), 1);
}

// ── 실패 경로 ─────────────────────────────────────────────────

#[test]
fn unknown_pending_kind_is_rejected() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    let err =
        export_pending_csv_impl(&conn, school, year, 3, 6, "check", "2026-09-10").unwrap_err();
    assert!(err.contains("check"), "{err}");
}

#[test]
fn broken_date_is_rejected() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    assert!(export_spans_csv_impl(&conn, school, year, 3, 6, "어제", "2026-09-30").is_err());
    assert!(export_spans_csv_impl(&conn, school, year, 3, 6, "2026-09-01", "2026-13-01").is_err());
    assert!(export_pending_csv_impl(&conn, school, year, 3, 6, "doc", "어제").is_err());
}

#[test]
fn a_day_past_the_end_of_the_month_is_rejected() {
    // 달의 마지막 날을 모르는 채 `{달}-31`로 기간을 만들면 2 · 4 · 6 · 9 · 11월에서
    // 내보내기가 통째로 실패한다. 그 값을 앱이 마지막 날로 당겨 주지 않는다 —
    // 기간을 정하는 것은 부르는 쪽의 일이다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    let err =
        export_spans_csv_impl(&conn, school, year, 3, 6, "2026-02-01", "2026-02-31").unwrap_err();
    assert!(err.contains("2026-02-31"), "{err}");
}

#[test]
fn reversed_range_is_rejected() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    let err =
        export_spans_csv_impl(&conn, school, year, 3, 6, "2026-09-30", "2026-09-01").unwrap_err();
    assert!(err.contains("앞뒤"), "{err}");
}

#[test]
fn missing_quota_rule_is_rejected() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    let err = export_quota_csv_impl(&conn, school, year, 3, 6, 9999).unwrap_err();
    assert!(err.contains("9999"), "{err}");
}

// ── DB를 거쳐서 ───────────────────────────────────────────────

#[test]
fn span_export_reads_only_the_given_range() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    let student = insert_student(&conn, year, 7, "김, 민준");
    let axes = axes(&conn, "질병", "결석");
    insert_span(&conn, student, "2026-09-10", axes, "감기");
    insert_span(&conn, student, "2026-10-02", axes, "장염");

    let csv = export_spans_csv_impl(&conn, school, year, 3, 6, "2026-09-01", "2026-09-30").unwrap();
    assert!(csv.starts_with('\u{feff}'));
    assert!(csv.contains("감기"), "{csv}");
    assert!(!csv.contains("장염"), "{csv}");
    // 쉼표가 든 이름은 DB를 거쳐도 한 칸으로 남는다.
    assert!(csv.contains("\"김, 민준\""), "{csv}");
}

#[test]
fn empty_class_exports_header_only() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    let csv = export_spans_csv_impl(&conn, school, year, 3, 6, "2026-09-01", "2026-09-30").unwrap();
    assert_eq!(body(&csv).len(), 1);
}

#[test]
fn quota_export_reads_the_named_rule() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    let student = insert_student(&conn, year, 7, "김민준");
    let axes = axes(&conn, "출석인정", "결석");
    let id = insert_span(&conn, student, "2026-05-04", axes, "가족 여행");
    conn.execute(
        "UPDATE absence_span SET tag_id = ?1 WHERE id = ?2",
        rusqlite::params![tag_id(&conn, "체험학습"), id],
    )
    .unwrap();

    let rule = quota_rule_id(&conn, "체험학습 연 20일");
    let csv = export_quota_csv_impl(&conn, school, year, 3, 6, rule).unwrap();
    assert!(csv.starts_with('\u{feff}'));
    let rows = body(&csv);
    assert_eq!(first_cells(&rows[0], 3), ["학년", "반", "번호"]);
    assert!(csv.contains("체험학습 연 20일"), "{csv}");
    assert!(csv.contains("김민준"), "{csv}");
}

#[test]
fn pending_export_lists_records_whose_due_has_not_arrived() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    let student = insert_student(&conn, year, 7, "김민준");
    let axes = axes(&conn, "질병", "결석");
    let id = insert_span(&conn, student, "2026-09-10", axes, "감기");
    conn.execute(
        "UPDATE absence_span SET doc_due = '2026-12-31' WHERE id = ?1",
        rusqlite::params![id],
    )
    .unwrap();

    let csv = export_pending_csv_impl(&conn, school, year, 3, 6, "doc", "2026-09-10").unwrap();
    assert!(csv.contains("김민준"), "{csv}");
    assert!(csv.contains("남음"), "{csv}");
}

#[test]
fn neis_export_flattens_the_day_groups_oldest_first() {
    // 나이스 목록은 날짜로 묶여 오는데 CSV는 한 표다. 묶음을 펼치면서 날짜 순서가
    // 흐트러지면 나이스에 위에서 아래로 옮겨 적을 수 없다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let school = school_id(&conn);
    let student = insert_student(&conn, year, 7, "김민준");
    let axes = axes(&conn, "질병", "결석");
    insert_span(&conn, student, "2026-09-08", axes, "늦게");
    insert_span(&conn, student, "2026-09-01", axes, "먼저");
    let done = insert_span(&conn, student, "2026-09-04", axes, "이미 넣음");
    conn.execute(
        "UPDATE absence_span SET neis_done = 1 WHERE id = ?1",
        rusqlite::params![done],
    )
    .unwrap();

    let csv = export_pending_csv_impl(&conn, school, year, 3, 6, "neis", "2026-09-10").unwrap();
    let rows = body(&csv);
    assert_eq!(rows.len(), 3, "머리글과 미등재 둘: {rows:?}");
    assert!(rows[1].contains("2026.09.01."), "{}", rows[1]);
    assert!(rows[2].contains("2026.09.08."), "{}", rows[2]);
    assert!(!csv.contains("이미 넣음"), "{csv}");
}

/// 문구 열은 `attendance_code.phrase_pattern`을 실제로 쓴다.
///
/// 패턴은 데이터다 — 코드에 문장을 박아 두면 학교마다 다른 표현을 고칠 수 없다.
/// 나이스 사유 칸에 붙여 넣을 초안이므로, 메모가 비면 자리표시자째 지워야 한다.
#[test]
fn the_phrase_column_uses_the_pattern_of_that_code() {
    use crate::commands::export::{build_spans_csv, phrase_patterns};

    let conn = setup_test_db();
    let patterns = phrase_patterns(&conn).unwrap();
    let reason = reason_id(&conn, "질병");
    let r#type = type_id(&conn, "결석");

    let mut s = span(1, 5, "김하늘");
    s.reason_id = Some(reason);
    s.type_id = Some(r#type);
    s.code_label = Some("질병결석".to_string());
    s.memo = "감기".to_string();

    let csv = build_spans_csv(3, 6, std::slice::from_ref(&s), &patterns);
    assert!(csv.contains("감기로 질병결석"), "문구가 만들어지지 않았다:
{csv}");

    // 메모가 비면 "(으)로" 부스러기가 남지 않아야 한다.
    s.memo = String::new();
    let csv = build_spans_csv(3, 6, std::slice::from_ref(&s), &patterns);
    assert!(csv.contains("질병결석"), "{csv}");
    assert!(!csv.contains("(으)로"), "빈 메모의 자리표시자가 남았다:
{csv}");
}

/// 두 축이 다 정해지지 않은 기록에는 문구를 만들지 않는다.
/// 억지로 만든 문장이 나이스에 그대로 올라가면 고치는 쪽이 더 번거롭다.
#[test]
fn an_unfinished_record_gets_no_phrase() {
    use crate::commands::export::{build_spans_csv, phrase_of};

    let conn = setup_test_db();
    let patterns = phrase_patterns_of(&conn);
    let mut s = span(1, 5, "김하늘");
    s.reason_id = None;
    s.code_label = None;

    assert_eq!(phrase_of(&s, &patterns), "");
    let _ = build_spans_csv(3, 6, std::slice::from_ref(&s), &patterns);
}

fn phrase_patterns_of(
    conn: &rusqlite::Connection,
) -> std::collections::HashMap<(i64, i64), Option<String>> {
    crate::commands::export::phrase_patterns(conn).unwrap()
}
