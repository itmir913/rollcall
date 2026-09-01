use crate::commands::attendance::{add_span_impl, get_spans_on_impl};
use crate::commands::code::*;
use crate::tests::*;

#[test]
fn seed_covers_every_reason_and_type() {
    let conn = setup_test_db();
    let codes = get_codes_impl(&conn, None).unwrap();
    assert_eq!(codes.len(), 16);
}

#[test]
fn the_four_common_codes_have_shortcuts() {
    let conn = setup_test_db();
    let codes = get_codes_impl(&conn, None).unwrap();
    let with_shortcut: Vec<&str> = codes
        .iter()
        .filter(|c| c.shortcut.is_some())
        .map(|c| c.label.as_str())
        .collect();
    assert_eq!(
        with_shortcut,
        vec!["질병결석", "질병조퇴", "출석인정결석", "출석인정조퇴"]
    );
}

#[test]
fn slot_prompt_rides_along_with_the_code() {
    // 프론트엔드가 유형을 보고 다시 계산하지 않게 한다.
    let conn = setup_test_db();
    let codes = get_codes_impl(&conn, None).unwrap();
    let by = |label: &str| {
        codes
            .iter()
            .find(|c| c.label == label)
            .unwrap()
            .slot_prompt
            .clone()
    };
    assert_eq!(by("질병결석"), "none");
    assert_eq!(by("질병조퇴"), "start");
    assert_eq!(by("질병지각"), "end");
    assert_eq!(by("질병결과"), "both");
}

#[test]
fn every_seeded_code_has_a_neis_alias() {
    let conn = setup_test_db();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM code_alias", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 16);
}

#[test]
fn retired_code_leaves_the_current_list() {
    let conn = setup_test_db();
    let id = code_id(&conn, "기타결과");
    retire_code_impl(&conn, id, "2026-09-01").unwrap();
    assert_eq!(get_codes_impl(&conn, None).unwrap().len(), 15);
}

#[test]
fn retired_code_still_appears_for_past_dates() {
    let conn = setup_test_db();
    let id = code_id(&conn, "기타결과");
    retire_code_impl(&conn, id, "2026-09-01").unwrap();

    let before = get_codes_impl(&conn, Some("2026-08-26")).unwrap();
    assert!(before.iter().any(|c| c.id == id));

    let after = get_codes_impl(&conn, Some("2026-09-01")).unwrap();
    assert!(!after.iter().any(|c| c.id == id));
}

#[test]
fn revising_a_code_closes_the_old_row_and_adds_a_new_one() {
    // UPDATE로 고치면 과거 기록의 뜻이 소급 변경된다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let old = code_id(&conn, "질병조퇴");
    add_span_impl(&conn, sid, "2026-08-26", old, Some("5"), None, Some("복통")).unwrap();

    let new = revise_code_impl(
        &conn,
        old,
        "질병",
        "조퇴",
        "질병 조퇴",
        Some("{증상}(으)로 {시작교시}부터 질병 조퇴"),
        None,
        None,
        Some("W"),
        11,
        "2026-09-01",
    )
    .unwrap();

    assert_ne!(new, old);

    // 과거 기록은 옛 코드를 그대로 가리킨다.
    let spans = get_spans_on_impl(&conn, "2026-08-26").unwrap();
    assert_eq!(spans[0].code_id, old);
    assert_eq!(spans[0].code_label, "질병조퇴");

    // 새 목록에는 새 코드만 있다.
    let current = get_codes_impl(&conn, None).unwrap();
    assert!(current.iter().any(|c| c.id == new));
    assert!(!current.iter().any(|c| c.id == old));
}

#[test]
fn revision_boundary_has_no_overlap() {
    let conn = setup_test_db();
    let old = code_id(&conn, "질병조퇴");
    revise_code_impl(
        &conn, old, "질병", "조퇴", "질병 조퇴", None, None, None, None, 11, "2026-09-01",
    )
    .unwrap();

    // 경계일에는 새 코드 하나만 유효하다.
    let on_boundary = get_codes_impl(&conn, Some("2026-09-01")).unwrap();
    let labels: Vec<&str> = on_boundary
        .iter()
        .filter(|c| c.code_type == "조퇴" && c.reason == "질병")
        .map(|c| c.label.as_str())
        .collect();
    assert_eq!(labels, vec!["질병 조퇴"]);
}

#[test]
fn code_label_cannot_be_blank() {
    let conn = setup_test_db();
    assert!(create_code_impl(&conn, "질병", "결석", " ", None, None, None, None, 0, "2026-09-01")
        .is_err());
}

#[test]
fn code_default_span_is_validated() {
    let conn = setup_test_db();
    let err = create_code_impl(
        &conn,
        "질병",
        "결과",
        "역순",
        None,
        Some("6"),
        Some("3"),
        None,
        0,
        "2026-09-01",
    )
    .unwrap_err();
    assert!(err.contains("시작 교시"));
}

// ── 증상 자동완성 ─────────────────────────────────────────────

#[test]
fn suggestions_come_from_past_entries() {
    // 별도 테이블 없이 쓸수록 후보가 쌓인다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let code = code_id(&conn, "질병결석");

    add_span_impl(&conn, sid, "2026-08-24", code, None, None, Some("몸살")).unwrap();
    add_span_impl(&conn, sid, "2026-08-25", code, None, None, Some("몸살")).unwrap();
    add_span_impl(&conn, sid, "2026-08-26", code, None, None, Some("복통")).unwrap();

    let all = get_symptom_suggestions_impl(&conn, "", 8).unwrap();
    assert_eq!(all, vec!["몸살", "복통"]); // 자주 쓴 것이 앞

    let filtered = get_symptom_suggestions_impl(&conn, "복", 8).unwrap();
    assert_eq!(filtered, vec!["복통"]);
}

#[test]
fn suggestion_prefix_treats_wildcards_literally() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_span_impl(
        &conn,
        sid,
        "2026-08-24",
        code_id(&conn, "질병결석"),
        None,
        None,
        Some("몸살"),
    )
    .unwrap();
    assert!(get_symptom_suggestions_impl(&conn, "%", 8).unwrap().is_empty());
}
