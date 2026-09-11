//! 두 축(구분·종류)과 그 쌍인 코드, 그리고 메모 자동완성.

use crate::commands::axis::*;
use crate::tests::*;
use rusqlite::Connection;

/// 구간 하나를 직접 넣는다. 축과 기간은 이 파일의 관심사가 아니므로 비워 둔다.
/// 구간은 **학급에 연결한다.** 메모 후보도 그 학급에서 쓰인 말이어야 도움이 된다.
fn insert_span(conn: &Connection, class_id: i64, student_id: i64, date: &str, memo: &str) {
    conn.execute(
        "INSERT INTO absence_span (class_id, student_id, date, memo) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![class_id, student_id, date, memo],
    )
    .unwrap();
}

// ── 시드 ──────────────────────────────────────────────────────

#[test]
fn seed_has_both_axes_and_every_pair() {
    // 쌍은 두 축의 곱이다. 한쪽만 늘려 놓고 코드를 안 만드는 일이 없어야 한다.
    let conn = setup_test_db();
    let reasons = get_reasons_impl(&conn, None).unwrap();
    let types = get_types_impl(&conn, None).unwrap();
    let codes = get_codes_impl(&conn, None).unwrap();

    assert!(!reasons.is_empty() && !types.is_empty());
    assert_eq!(codes.len(), reasons.len() * types.len());
}

#[test]
fn slot_prompt_is_data_not_a_string_match_in_code() {
    // 코드에서 한국어 라벨을 문자열로 비교하면 사용자가 종류를 추가하는 순간 틀린다.
    let conn = setup_test_db();
    let types = get_types_impl(&conn, None).unwrap();
    let by = |label: &str| {
        types
            .iter()
            .find(|t| t.label == label)
            .unwrap_or_else(|| panic!("시드에 없는 종류: {label}"))
            .slot_prompt
            .clone()
    };
    assert_eq!(by("결석"), "none");
    assert_eq!(by("조퇴"), "start");
    assert_eq!(by("지각"), "end");
    assert_eq!(by("결과"), "multi");
}

#[test]
fn every_seeded_slot_prompt_is_a_known_value() {
    let conn = setup_test_db();
    for t in get_types_impl(&conn, None).unwrap() {
        assert!(
            crate::slots::is_slot_prompt(&t.slot_prompt),
            "{}의 slot_prompt가 알 수 없는 값이다: {}",
            t.label,
            t.slot_prompt
        );
    }
}

#[test]
fn seeded_pairs_carry_a_phrase_pattern_matching_their_type() {
    // 자리표시자는 `{메모}`다. 교사가 치는 칸이 메모 하나이기 때문이다.
    let conn = setup_test_db();
    let codes = get_codes_impl(&conn, None).unwrap();
    let pattern = |label: &str| {
        codes
            .iter()
            .find(|c| c.label == label)
            .unwrap_or_else(|| panic!("시드에 없는 코드: {label}"))
            .phrase_pattern
            .clone()
            .unwrap()
    };
    assert_eq!(pattern("질병결석"), "{메모}(으)로 질병결석");
    assert_eq!(pattern("질병조퇴"), "{메모}(으)로 {시작교시}부터 질병조퇴");
    assert_eq!(pattern("질병지각"), "{메모}(으)로 {끝교시}까지 질병지각");
}

#[test]
fn every_seeded_code_has_a_neis_alias() {
    let conn = setup_test_db();
    let aliases: i64 = conn
        .query_row("SELECT COUNT(*) FROM code_alias", [], |r| r.get(0))
        .unwrap();
    assert_eq!(aliases, get_codes_impl(&conn, None).unwrap().len() as i64);
}

// ── 쌍 찾기 ───────────────────────────────────────────────────

#[test]
fn a_pair_is_found_only_when_both_axes_are_set() {
    let conn = setup_test_db();
    let r = reason_id(&conn, "질병");
    let t = type_id(&conn, "조퇴");

    let found = find_code_impl(&conn, Some(r), Some(t), None).unwrap();
    assert_eq!(found.unwrap().label, "질병조퇴");

    // 한쪽만 설정된 상태는 정상이고, 그때는 코드가 없다.
    assert!(find_code_impl(&conn, Some(r), None, None).unwrap().is_none());
    assert!(find_code_impl(&conn, None, Some(t), None).unwrap().is_none());
    assert!(find_code_impl(&conn, None, None, None).unwrap().is_none());
}

#[test]
fn a_pair_that_never_existed_is_not_an_error() {
    let conn = setup_test_db();
    let r = reason_id(&conn, "질병");
    let t = type_id(&conn, "조퇴");
    // 없는 축 id로 물어도 실패가 아니라 "없음"이다. 화면은 미완성 기록을 그린다.
    assert!(find_code_impl(&conn, Some(9999), Some(t), None)
        .unwrap()
        .is_none());
    assert!(find_code_impl(&conn, Some(r), Some(9999), None)
        .unwrap()
        .is_none());
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
    let before = get_codes_impl(&conn, None).unwrap();
    let id = before.iter().find(|c| c.label == "기타결과").unwrap().id;
    retire_code_impl(&conn, id, "2026-09-01").unwrap();

    assert_eq!(get_codes_impl(&conn, None).unwrap().len(), before.len() - 1);
    assert!(get_codes_impl(&conn, Some("2026-08-26"))
        .unwrap()
        .iter()
        .any(|c| c.id == id));
    // 경계일에는 이미 마감된 것으로 본다.
    assert!(!get_codes_impl(&conn, Some("2026-09-01"))
        .unwrap()
        .iter()
        .any(|c| c.id == id));
}

#[test]
fn revising_a_code_closes_the_old_row_and_keeps_past_records_pointing_at_the_axes() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let sid = enroll(&conn, class, school, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "조퇴");

    conn.execute(
        "INSERT INTO absence_span
             (class_id, student_id, date, reason_id, type_id, start_slot, memo)
         VALUES (?1, ?2, '2026-08-26', ?3, ?4, '5', '복통')",
        rusqlite::params![class, sid, r, t],
    )
    .unwrap();

    let old = find_code_impl(&conn, r, t, None).unwrap().unwrap();
    let new = revise_code_impl(
        &conn,
        old.id,
        "질병 조퇴",
        Some("{메모}(으)로 {시작교시}부터 질병 조퇴"),
        11,
        "2026-09-01",
    )
    .unwrap();
    assert_ne!(new, old.id);

    // 구간은 코드가 아니라 두 축을 가리킨다. 코드를 바꿔도 기록은 그대로다.
    let (span_reason, span_type): (Option<i64>, Option<i64>) = conn
        .query_row(
            "SELECT reason_id, type_id FROM absence_span WHERE student_id = ?1",
            rusqlite::params![sid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(span_reason, r);
    assert_eq!(span_type, t);

    // 그날 유효했던 코드가 그날 라벨을 준다.
    let then = find_code_impl(&conn, r, t, Some("2026-08-26")).unwrap().unwrap();
    assert_eq!(then.label, "질병조퇴");
    let now = find_code_impl(&conn, r, t, None).unwrap().unwrap();
    assert_eq!(now.label, "질병 조퇴");
}

#[test]
fn revising_a_code_that_does_not_exist_says_so_in_korean() {
    let conn = setup_test_db();
    let err = revise_code_impl(&conn, 9999, "없는코드", None, 0, "2026-09-01").unwrap_err();
    assert!(err.contains("찾을 수 없습니다"), "{err}");

    // 트랜잭션이 열린 채 남지 않았는지 — 다음 쓰기가 정상 동작해야 한다.
    let r = reason_id(&conn, "질병");
    retire_reason_impl(&conn, r, "2026-09-01").unwrap();
}

#[test]
fn retiring_something_twice_says_so_instead_of_doing_nothing() {
    // 조용히 넘기면 교사는 방금 누른 것이 처리된 줄 안다.
    let conn = setup_test_db();
    let id = get_codes_impl(&conn, None)
        .unwrap()
        .iter()
        .find(|c| c.label == "기타결과")
        .unwrap()
        .id;
    retire_code_impl(&conn, id, "2026-09-01").unwrap();
    assert!(retire_code_impl(&conn, id, "2026-09-10")
        .unwrap_err()
        .contains("이미 마감"));
    assert!(retire_code_impl(&conn, 9999, "2026-09-01").is_err());

    let r = reason_id(&conn, "기타");
    retire_reason_impl(&conn, r, "2026-09-01").unwrap();
    assert!(retire_reason_impl(&conn, r, "2026-09-10").is_err());

    let t = type_id(&conn, "결과");
    retire_type_impl(&conn, t, "2026-09-01").unwrap();
    assert!(retire_type_impl(&conn, t, "2026-09-10").is_err());

    // 실패한 마감이 트랜잭션을 열어 둔 채 끝나지 않았는지.
    retire_type_impl(&conn, type_id(&conn, "지각"), "2026-09-01").unwrap();
}

#[test]
fn retiring_an_axis_also_retires_the_pairs_that_use_it() {
    // 남겨두면 사라진 구분을 가진 코드가 계속 유효한 것으로 조회된다.
    let conn = setup_test_db();
    let reasons_before = get_reasons_impl(&conn, None).unwrap().len();
    let types_before = get_types_impl(&conn, None).unwrap().len();

    let r = reason_id(&conn, "기타");
    retire_reason_impl(&conn, r, "2026-09-01").unwrap();

    assert_eq!(
        get_reasons_impl(&conn, None).unwrap().len(),
        reasons_before - 1
    );
    assert_eq!(
        get_codes_impl(&conn, None).unwrap().len(),
        (reasons_before - 1) * types_before
    );
}

#[test]
fn retiring_a_type_also_retires_its_pairs() {
    let conn = setup_test_db();
    let reasons_before = get_reasons_impl(&conn, None).unwrap().len();
    let types_before = get_types_impl(&conn, None).unwrap().len();

    let t = type_id(&conn, "결과");
    retire_type_impl(&conn, t, "2026-09-01").unwrap();

    assert_eq!(get_types_impl(&conn, None).unwrap().len(), types_before - 1);
    assert_eq!(
        get_codes_impl(&conn, None).unwrap().len(),
        reasons_before * (types_before - 1)
    );
}

// ── 축 추가 ───────────────────────────────────────────────────

#[test]
fn a_new_type_must_declare_how_it_asks_for_slots() {
    let conn = setup_test_db();
    let err = create_type_impl(&conn, "공결", "가운데", None, 50, "2026-09-01").unwrap_err();
    assert!(
        err.contains("none / start / end / multi"),
        "허용값 목록이 문구에 없다: {err}"
    );

    let before = get_types_impl(&conn, None).unwrap().len();
    assert!(create_type_impl(&conn, "공결", "multi", None, 50, "2026-09-01").is_ok());
    assert_eq!(get_types_impl(&conn, None).unwrap().len(), before + 1);
}

#[test]
fn the_old_both_is_no_longer_a_slot_prompt() {
    // `both`가 `multi`로 바뀌었다. 옛 값이 조용히 저장되면 화면이 교시를 안 묻는다.
    let conn = setup_test_db();
    let before = get_types_impl(&conn, None).unwrap().len();
    let err = create_type_impl(&conn, "공결", "both", None, 50, "2026-09-01").unwrap_err();
    assert!(err.contains("multi"), "허용값 목록이 낡았다: {err}");
    assert_eq!(get_types_impl(&conn, None).unwrap().len(), before);
}

#[test]
fn axis_labels_cannot_be_blank() {
    let conn = setup_test_db();
    assert!(create_reason_impl(&conn, "  ", None, 0, "2026-09-01").is_err());
    assert!(create_type_impl(&conn, " ", "none", None, 0, "2026-09-01").is_err());
    assert!(create_code_impl(
        &conn,
        reason_id(&conn, "질병"),
        type_id(&conn, "결석"),
        " ",
        None,
        0,
        "2026-09-01"
    )
    .is_err());
}

// ── 메모 자동완성 ─────────────────────────────────────────────

#[test]
fn memo_suggestions_come_from_past_entries_most_used_first() {
    // 별도 테이블 없이 쓸수록 후보가 쌓인다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let sid = enroll(&conn, class, school, 2, "이영희");

    insert_span(&conn, class, sid, "2026-08-24", "몸살");
    insert_span(&conn, class, sid, "2026-08-25", "몸살");
    insert_span(&conn, class, sid, "2026-08-26", "복통");

    assert_eq!(
        get_memo_suggestions_impl(&conn, class, 8).unwrap(),
        vec!["몸살", "복통"]
    );
}

#[test]
fn an_empty_memo_is_not_a_suggestion() {
    // 메모는 비어 있는 것이 정상이다. 그것이 후보 버튼으로 나오면 자리만 차지한다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let sid = enroll(&conn, class, school, 2, "이영희");

    insert_span(&conn, class, sid, "2026-08-24", "");
    insert_span(&conn, class, sid, "2026-08-25", "   ");
    insert_span(&conn, class, sid, "2026-08-26", "복통");

    assert_eq!(
        get_memo_suggestions_impl(&conn, class, 8).unwrap(),
        vec!["복통"]
    );
}

#[test]
fn the_same_memo_typed_with_a_stray_space_is_one_candidate() {
    // 저장은 교사가 친 그대로다. 앞뒤 공백을 제거하지 않고 세면 똑같이 생긴 버튼이 둘 나온다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let sid = enroll(&conn, class, school, 2, "이영희");

    insert_span(&conn, class, sid, "2026-08-24", "복통");
    insert_span(&conn, class, sid, "2026-08-25", "복통 ");
    insert_span(&conn, class, sid, "2026-08-26", " 복통");

    assert_eq!(
        get_memo_suggestions_impl(&conn, class, 8).unwrap(),
        vec!["복통"]
    );
}

#[test]
fn memo_suggestions_stay_inside_the_class() {
    // 후보는 그 반에서 실제로 쓰인 말이어야 도움이 된다. 범위가 학급이므로
    // 작년 반도 옆 학교도 자연히 구별된다 — 학급이 학년도와 학교를 함께 가리킨다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let mine = enroll(&conn, class, school, 2, "이영희");

    let next_door = insert_class(&conn, school, "homeroom", "1학년 2반", Some(1), Some(2));
    let other_class = insert_student_at(&conn, school, 1, 2, 1, "최지훈");
    join_class(&conn, next_door, other_class);

    insert_span(&conn, class, mine, "2026-08-26", "복통");
    insert_span(&conn, next_door, other_class, "2026-08-26", "옆반메모");

    assert_eq!(
        get_memo_suggestions_impl(&conn, class, 8).unwrap(),
        vec!["복통"]
    );
    assert_eq!(
        get_memo_suggestions_impl(&conn, next_door, 8).unwrap(),
        vec!["옆반메모"]
    );
}

#[test]
fn memo_suggestions_respect_the_limit() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let sid = enroll(&conn, class, school, 2, "이영희");

    for (i, memo) in ["몸살", "복통", "두통"].iter().enumerate() {
        insert_span(&conn, class, sid, &format!("2026-08-{:02}", 20 + i), memo);
    }

    assert_eq!(
        get_memo_suggestions_impl(&conn, class, 2)
            .unwrap()
            .len(),
        2
    );
    assert!(get_memo_suggestions_impl(&conn, class, 0)
        .unwrap()
        .is_empty());
}

#[test]
fn a_class_with_no_records_yet_gets_an_empty_list() {
    // 첫 실행에서 후보가 없는 것은 실패가 아니다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    assert!(get_memo_suggestions_impl(&conn, class, 8)
        .unwrap()
        .is_empty());
}
