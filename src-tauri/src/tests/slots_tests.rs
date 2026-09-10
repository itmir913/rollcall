//! `slots.rs` — 슬롯 순서와 부재 구간의 순수 로직.
//!
//! 여기서 확인하는 것은 두 가지다.
//!   · 최대 교시가 **학교 설정값**이라는 사실 — 모든 함수가 `max_slot`을 받고,
//!     같은 토큰이 학교에 따라 유효하기도 무효하기도 하다.
//!   · 프로그램이 판정하지 않는다는 원칙 — 표현할 수 없는 구간만 거부하고,
//!     종류와 구간이 어긋나는 것은 통과시킨다.

use crate::slots::*;

// ─── 순서 ──────────────────────────────────────────────────────

#[test]
fn slot_list_runs_from_homeroom_to_closing() {
    assert_eq!(
        slots(7),
        vec!["조회", "1", "2", "3", "4", "5", "6", "7", "종례"]
    );
}

#[test]
fn slot_list_follows_the_school_setting() {
    // 최대 교시는 학교가 들고 있는 값이다. 배열에 1~9를 박아 두고 자르지 않는다.
    assert_eq!(slots(1), vec!["조회", "1", "종례"]);
    assert_eq!(slots(9).len(), 11);
    assert_eq!(slots(9).last().unwrap(), "종례");
}

#[test]
fn ordinal_matches_the_position_in_the_list() {
    // 목록의 순서와 ordinal이 어긋나면 화면의 버튼 순서와 저장된 구간이 갈라진다.
    for max_slot in 1..=9 {
        for (index, slot) in slots(max_slot).iter().enumerate() {
            assert_eq!(
                ordinal(slot, max_slot),
                Some(index),
                "max_slot={max_slot}에서 {slot}의 순서값이 목록 위치와 다르다"
            );
        }
    }
}

#[test]
fn homeroom_is_first_and_closing_is_last() {
    // 조회와 종례는 설정 대상이 아니라 언제나 하루의 양 끝이다.
    for max_slot in [1, 5, 7, 9] {
        assert_eq!(ordinal(HOMEROOM, max_slot), Some(0));
        assert_eq!(ordinal(CLOSING, max_slot), Some(max_slot + 1));
    }
}

// ─── 알 수 없는 토큰 ───────────────────────────────────────────

#[test]
fn ordinal_rejects_a_period_beyond_the_school_maximum() {
    // 7교시까지인 학교에서 8교시는 존재하지 않는다.
    assert_eq!(ordinal("8", 7), None);
    assert_eq!(ordinal("7", 7), Some(7));
}

#[test]
fn the_same_token_depends_on_the_school() {
    // 같은 "8"이 학교마다 유효하기도 무효하기도 하다.
    assert_eq!(ordinal("8", 7), None);
    assert_eq!(ordinal("8", 9), Some(8));
}

#[test]
fn ordinal_rejects_tokens_that_are_not_periods() {
    assert_eq!(ordinal("0", 7), None);
    assert_eq!(ordinal("-1", 7), None);
    assert_eq!(ordinal("", 7), None);
    assert_eq!(ordinal("점심", 7), None);
    assert_eq!(ordinal(" 3", 7), None);
    assert_eq!(ordinal(UNKNOWN, 7), None);
}

// ─── 표기 ──────────────────────────────────────────────────────

#[test]
fn display_appends_gyosi_only_to_numbers() {
    assert_eq!(display("5"), "5교시");
    assert_eq!(display(HOMEROOM), "조회");
    assert_eq!(display(CLOSING), "종례");
}

#[test]
fn display_keeps_the_open_mark_as_is() {
    // 열린 쪽에 "?교시"라고 적히면 실제 교시처럼 읽힌다.
    assert_eq!(display(UNKNOWN), "?");
}

#[test]
fn open_ends_render_as_a_question_mark() {
    // 저장은 NULL이고 화면에만 ?로 적는다.
    assert_eq!(format_span(None, None), "? ~ ?");
    assert_eq!(format_span(Some("5"), None), "5 ~ ?");
    assert_eq!(format_span(None, Some("4")), "? ~ 4");
    assert_eq!(format_span(Some("조회"), Some("종례")), "조회 ~ 종례");
    assert_eq!(format_span(Some("4"), Some("4")), "4 ~ 4");
}

// ─── slot_prompt ───────────────────────────────────────────────

#[test]
fn slot_prompt_values_are_fixed() {
    // 값 자체는 attendance_type 행에 있다. 여기서는 알 수 없는 값만 거부한다.
    assert_eq!(SLOT_PROMPTS, &["none", "start", "end", "multi"]);
    for value in SLOT_PROMPTS {
        assert!(is_slot_prompt(value));
    }
}

#[test]
fn unknown_slot_prompt_is_rejected() {
    assert!(!is_slot_prompt("both"));
    assert!(!is_slot_prompt("가운데"));
    assert!(!is_slot_prompt("NONE"));
    assert!(!is_slot_prompt(""));
}

// ─── 구간 검사 ─────────────────────────────────────────────────

#[test]
fn validate_rejects_a_reversed_span() {
    let err = validate_span(Some("5"), Some("3"), 7).unwrap_err();
    assert!(err.contains("뒤입니다"), "실제 문구: {err}");
    assert!(err.contains("5교시"), "실제 문구: {err}");
    assert!(err.contains("3교시"), "실제 문구: {err}");
}

#[test]
fn validate_orders_homeroom_before_closing() {
    assert!(validate_span(Some("종례"), Some("조회"), 7).is_err());
    assert!(validate_span(Some("조회"), Some("종례"), 7).is_ok());
}

#[test]
fn validate_accepts_a_single_period_span() {
    assert!(validate_span(Some("4"), Some("4"), 7).is_ok());
    assert!(validate_span(Some("3"), Some("5"), 7).is_ok());
}

#[test]
fn validate_accepts_open_ends() {
    // 열린 구간이 정상 상태다. 슬롯을 펼치지 않기 때문이다.
    assert!(validate_span(None, None, 7).is_ok());
    assert!(validate_span(Some("5"), None, 7).is_ok());
    assert!(validate_span(None, Some("3"), 7).is_ok());
    assert!(validate_span(Some("종례"), None, 7).is_ok());
}

#[test]
fn validate_rejects_an_unknown_slot_on_each_side() {
    let start_err = validate_span(Some("8"), None, 7).unwrap_err();
    assert!(start_err.contains("시작 교시"), "실제 문구: {start_err}");
    assert!(start_err.contains('8'), "실제 문구: {start_err}");

    let end_err = validate_span(None, Some("점심"), 7).unwrap_err();
    assert!(end_err.contains("끝 교시"), "실제 문구: {end_err}");
    assert!(end_err.contains("점심"), "실제 문구: {end_err}");
}

#[test]
fn validate_follows_the_school_maximum() {
    // 9교시까지인 학교에서는 같은 구간이 통과한다.
    assert!(validate_span(Some("8"), Some("9"), 7).is_err());
    assert!(validate_span(Some("8"), Some("9"), 9).is_ok());
}

#[test]
fn validate_does_not_judge_the_type_against_the_span() {
    // 프로그램은 판정하지 않는다. "결석인데 5교시부터"도 저장은 된다.
    assert!(validate_span(Some("5"), None, 7).is_ok());
    assert!(validate_span(Some("조회"), Some("조회"), 7).is_ok());
}

// ─── 겹침 ──────────────────────────────────────────────────────

#[test]
fn overlap_treats_an_open_start_as_the_first_slot() {
    // 열린 시작은 조회부터로 본다.
    assert!(overlaps((None, Some("2")), (Some("조회"), Some("조회")), 7));
    assert!(!overlaps((None, Some("2")), (Some("3"), Some("3")), 7));
}

#[test]
fn overlap_treats_an_open_end_as_the_last_slot() {
    // 열린 끝은 종례까지로 본다.
    assert!(overlaps((Some("5"), None), (Some("종례"), Some("종례")), 7));
    assert!(overlaps((None, None), (Some("3"), Some("3")), 7));
}

#[test]
fn overlap_is_true_when_the_spans_touch_at_one_period() {
    // 3교시 결과와 3~7교시 조퇴는 3교시에서 겹친다.
    assert!(overlaps((Some("3"), Some("3")), (Some("3"), Some("7")), 7));
}

#[test]
fn disjoint_spans_do_not_overlap() {
    // 1교시 지각 + 5~7교시 조퇴 — 하루 2구간의 정상 입력이다.
    assert!(!overlaps((Some("조회"), Some("1")), (Some("5"), None), 7));
    assert!(!overlaps((Some("1"), Some("2")), (Some("3"), Some("4")), 7));
}

#[test]
fn overlap_is_symmetric() {
    let a = (Some("2"), Some("4"));
    let b = (Some("4"), None);
    assert_eq!(overlaps(a, b, 7), overlaps(b, a, 7));
    // 같은 구간끼리는 언제나 겹친다 — 같은 조합 재클릭을 걸러내는 근거다.
    assert!(overlaps(a, a, 7));
}

#[test]
fn overlap_follows_the_school_maximum() {
    // 열린 끝이 어디까지인지가 학교 설정에 따라 달라진다.
    // 7교시 학교에서 종례는 8번째이므로 열린 끝과 만난다.
    assert!(overlaps((Some("5"), None), (Some("종례"), Some("종례")), 7));
    // 두 구간을 하루의 양 끝에 두면 어느 학교에서도 만나지 않는다.
    assert!(!overlaps((None, Some("조회")), (Some("종례"), None), 7));
    assert!(!overlaps((None, Some("조회")), (Some("종례"), None), 9));
}

#[test]
fn overlap_widens_an_unknown_token_to_the_whole_day() {
    // `overlaps`는 파싱하지 못한 토큰을 하루의 처음·끝으로 대신한다. 7교시 학교에
    // "9"가 들어오면 그 구간이 하루 전체가 되어 무엇과도 겹친 것으로 나온다.
    // 저장 경로는 `validate_span`이 먼저 거부하므로 실제로는 생기지 않지만,
    // 검증 없이 `overlaps`를 직접 부르면 그 자리에서 오검출이 발생한다.
    assert!(overlaps(
        (Some("9"), Some("9")),
        (Some("조회"), Some("조회")),
        7
    ));
    assert!(validate_span(Some("9"), Some("9"), 7).is_err());
}

// ─── 이어진 교시 묶기 ──────────────────────────────────────────

#[test]
fn consecutive_periods_become_one_span() {
    assert_eq!(group_runs(vec![1, 2, 3]), vec![(1, 3)]);
}

#[test]
fn gapped_periods_become_separate_spans() {
    // 1,3,5를 한 구간으로 저장하면 2교시와 4교시가 조용히 포함된다.
    assert_eq!(group_runs(vec![1, 3, 5]), vec![(1, 1), (3, 3), (5, 5)]);
}

#[test]
fn group_runs_sorts_and_dedups_first() {
    assert_eq!(group_runs(vec![3, 1, 2]), vec![(1, 3)]);
    assert_eq!(group_runs(vec![2, 3, 2, 3]), vec![(2, 3)]);
}

#[test]
fn group_runs_handles_mixed_input() {
    assert_eq!(
        group_runs(vec![9, 1, 5, 2, 4, 6]),
        vec![(1, 2), (4, 6), (9, 9)]
    );
}

#[test]
fn group_runs_handles_the_empty_and_single_cases() {
    assert!(group_runs(vec![]).is_empty());
    assert_eq!(group_runs(vec![7]), vec![(7, 7)]);
}
