use crate::commands::attendance::{add_spans_impl, get_spans_on_impl};
use crate::commands::axis::*;
use crate::tests::*;

// ── 시드 ──────────────────────────────────────────────────────

#[test]
fn seed_has_both_axes_and_every_pair() {
    let conn = setup_test_db();
    assert_eq!(get_reasons_impl(&conn, None).unwrap().len(), 4);
    assert_eq!(get_types_impl(&conn, None).unwrap().len(), 4);
    assert_eq!(get_codes_impl(&conn, None).unwrap().len(), 16);
}

#[test]
fn slot_prompt_is_data_not_a_string_match_in_code() {
    let conn = setup_test_db();
    let types = get_types_impl(&conn, None).unwrap();
    let by = |label: &str| {
        types
            .iter()
            .find(|t| t.label == label)
            .unwrap()
            .slot_prompt
            .clone()
    };
    assert_eq!(by("결석"), "none");
    assert_eq!(by("조퇴"), "start");
    assert_eq!(by("지각"), "end");
    assert_eq!(by("결과"), "both");
}

#[test]
fn seeded_pairs_carry_a_phrase_pattern_matching_their_type() {
    let conn = setup_test_db();
    let codes = get_codes_impl(&conn, None).unwrap();
    let pattern = |label: &str| {
        codes
            .iter()
            .find(|c| c.label == label)
            .unwrap()
            .phrase_pattern
            .clone()
            .unwrap()
    };
    assert_eq!(pattern("질병결석"), "{증상}(으)로 질병결석");
    assert_eq!(pattern("질병조퇴"), "{증상}(으)로 {시작교시}부터 질병조퇴");
    assert_eq!(pattern("질병지각"), "{증상}(으)로 {끝교시}까지 질병지각");
}

#[test]
fn every_seeded_code_has_a_neis_alias() {
    let conn = setup_test_db();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM code_alias", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 16);
}

// ── 쌍 찾기 ───────────────────────────────────────────────────

#[test]
fn a_pair_is_found_only_when_both_axes_are_set() {
    let conn = setup_test_db();
    let r = reason_id(&conn, "질병");
    let t = type_id(&conn, "조퇴");

    let found = find_code_impl(&conn, Some(r), Some(t), None).unwrap();
    assert_eq!(found.unwrap().label, "질병조퇴");

    // 한쪽만 정해진 상태는 정상이고, 그때는 코드가 없다.
    assert!(find_code_impl(&conn, Some(r), None, None).unwrap().is_none());
    assert!(find_code_impl(&conn, None, Some(t), None).unwrap().is_none());
    assert!(find_code_impl(&conn, None, None, None).unwrap().is_none());
}

#[test]
fn only_one_pair_can_be_active_at_a_time() {
    // 같은 쌍이 둘 다 유효하면 문구를 어느 쪽으로 그릴지 알 수 없다.
    let conn = setup_test_db();
    let r = reason_id(&conn, "질병");
    let t = type_id(&conn, "조퇴");
    let err = create_code_impl(&conn, r, t, "질병 조퇴", None, 0, "2026-09-01").unwrap_err();
    assert!(err.contains("이미"), "영문 원문이 새어나왔다: {err}");
}

// ── 마감 후 추가 ──────────────────────────────────────────────

#[test]
fn retired_code_leaves_the_current_list_but_stays_for_past_dates() {
    let conn = setup_test_db();
    let id = get_codes_impl(&conn, None)
        .unwrap()
        .iter()
        .find(|c| c.label == "기타결과")
        .unwrap()
        .id;
    retire_code_impl(&conn, id, "2026-09-01").unwrap();

    assert_eq!(get_codes_impl(&conn, None).unwrap().len(), 15);
    assert!(get_codes_impl(&conn, Some("2026-08-26"))
        .unwrap()
        .iter()
        .any(|c| c.id == id));
    assert!(!get_codes_impl(&conn, Some("2026-09-01"))
        .unwrap()
        .iter()
        .any(|c| c.id == id));
}

#[test]
fn revising_a_code_closes_the_old_row_and_keeps_past_records_pointing_at_the_axes() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "조퇴");
    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, Some("5"), None, Some("복통")).unwrap();

    let old = find_code_impl(&conn, r, t, None).unwrap().unwrap();
    let new = revise_code_impl(
        &conn,
        old.id,
        "질병 조퇴",
        Some("{증상}(으)로 {시작교시}부터 질병 조퇴"),
        11,
        "2026-09-01",
    )
    .unwrap();
    assert_ne!(new, old.id);

    // 구간은 코드가 아니라 두 축을 가리킨다. 코드를 갈아도 기록은 그대로다.
    let spans = get_spans_on_impl(&conn, "2026-08-26").unwrap();
    assert_eq!(spans[0].reason_id, r);
    assert_eq!(spans[0].type_id, t);
    // 그날 유효했던 코드로 라벨이 붙는다.
    assert_eq!(spans[0].code_label.as_deref(), Some("질병조퇴"));
}

#[test]
fn retiring_an_axis_also_retires_the_pairs_that_use_it() {
    // 남겨두면 사라진 구분을 가진 코드가 계속 유효한 것으로 조회된다.
    let conn = setup_test_db();
    let r = reason_id(&conn, "기타");
    retire_reason_impl(&conn, r, "2026-09-01").unwrap();

    assert_eq!(get_reasons_impl(&conn, None).unwrap().len(), 3);
    assert_eq!(get_codes_impl(&conn, None).unwrap().len(), 12);
}

#[test]
fn retiring_a_type_also_retires_its_pairs() {
    let conn = setup_test_db();
    let t = type_id(&conn, "결과");
    retire_type_impl(&conn, t, "2026-09-01").unwrap();

    assert_eq!(get_types_impl(&conn, None).unwrap().len(), 3);
    assert_eq!(get_codes_impl(&conn, None).unwrap().len(), 12);
}

// ── 축 추가 ───────────────────────────────────────────────────

#[test]
fn a_new_type_must_declare_how_it_asks_for_slots() {
    let conn = setup_test_db();
    let err = create_type_impl(&conn, "공결", "가운데", None, 50, "2026-09-01").unwrap_err();
    assert!(err.contains("none / start / end / both"));

    assert!(create_type_impl(&conn, "공결", "none", None, 50, "2026-09-01").is_ok());
    assert_eq!(get_types_impl(&conn, None).unwrap().len(), 5);
}

#[test]
fn axis_labels_cannot_be_blank() {
    let conn = setup_test_db();
    assert!(create_reason_impl(&conn, "  ", None, 0, "2026-09-01").is_err());
    assert!(create_type_impl(&conn, " ", "none", None, 0, "2026-09-01").is_err());
}

// ── 증상 자동완성 ─────────────────────────────────────────────

#[test]
fn suggestions_come_from_past_entries() {
    // 별도 테이블 없이 쓸수록 후보가 쌓인다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");

    add_spans_impl(&conn, &[sid], "2026-08-24", r, t, None, None, Some("몸살")).unwrap();
    add_spans_impl(&conn, &[sid], "2026-08-25", r, t, None, None, Some("몸살")).unwrap();
    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, None, None, Some("복통")).unwrap();

    assert_eq!(
        get_symptom_suggestions_impl(&conn, "", 8).unwrap(),
        vec!["몸살", "복통"] // 자주 쓴 것이 앞
    );
    assert_eq!(
        get_symptom_suggestions_impl(&conn, "복", 8).unwrap(),
        vec!["복통"]
    );
}

#[test]
fn suggestion_prefix_treats_wildcards_literally() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");
    add_spans_impl(&conn, &[sid], "2026-08-24", r, t, None, None, Some("몸살")).unwrap();
    assert!(get_symptom_suggestions_impl(&conn, "%", 8).unwrap().is_empty());
}
