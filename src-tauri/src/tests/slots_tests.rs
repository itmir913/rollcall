use crate::slots::*;

#[test]
fn slot_order_is_homeroom_first_and_dismissal_last() {
    assert_eq!(ordinal("조회"), Some(0));
    assert_eq!(ordinal("종례"), Some(SLOTS.len() - 1));
    assert!(ordinal("조회").unwrap() < ordinal("1").unwrap());
    assert!(ordinal("9").unwrap() < ordinal("종례").unwrap());
}

#[test]
fn unknown_slot_has_no_ordinal() {
    assert_eq!(ordinal("10"), None);
    assert_eq!(ordinal("점심"), None);
}

#[test]
fn display_adds_gyosi_only_to_numbers() {
    assert_eq!(display("5"), "5교시");
    assert_eq!(display("조회"), "조회");
    assert_eq!(display("종례"), "종례");
}

#[test]
fn open_ends_render_as_asterisk() {
    assert_eq!(format_span(None, None), "* ~ *");
    assert_eq!(format_span(Some("5"), None), "5 ~ *");
    assert_eq!(format_span(None, Some("4")), "* ~ 4");
    assert_eq!(format_span(Some("4"), Some("4")), "4 ~ 4");
}

#[test]
fn slot_prompt_follows_type_definition() {
    assert_eq!(prompt_for_type("결석"), SlotPrompt::None);
    assert_eq!(prompt_for_type("조퇴"), SlotPrompt::Start);
    assert_eq!(prompt_for_type("지각"), SlotPrompt::End);
    assert_eq!(prompt_for_type("결과"), SlotPrompt::Both);
}

#[test]
fn unknown_type_asks_for_both_ends() {
    // 사용자가 유형을 늘려도 입력이 막히지 않아야 한다.
    assert_eq!(prompt_for_type("공결"), SlotPrompt::Both);
}

#[test]
fn validate_rejects_reversed_span() {
    assert!(validate_span(Some("5"), Some("3")).is_err());
    assert!(validate_span(Some("3"), Some("5")).is_ok());
    assert!(validate_span(Some("4"), Some("4")).is_ok());
}

#[test]
fn validate_rejects_unknown_slot() {
    assert!(validate_span(Some("11"), None).is_err());
    assert!(validate_span(None, Some("점심")).is_err());
}

#[test]
fn open_ends_always_validate() {
    assert!(validate_span(None, None).is_ok());
    assert!(validate_span(Some("종례"), None).is_ok());
}

#[test]
fn validate_does_not_judge_type_against_span() {
    // 프로그램은 판정하지 않는다. "결석인데 5교시부터"도 저장은 된다.
    assert!(validate_span(Some("5"), None).is_ok());
}

#[test]
fn overlap_treats_open_ends_as_full_day() {
    assert!(overlaps((None, None), (Some("3"), Some("3"))));
    assert!(overlaps((Some("5"), None), (Some("종례"), Some("종례"))));
}

#[test]
fn disjoint_spans_do_not_overlap() {
    // 3교시 결과 + 6교시 조퇴 — 실제로 있는 하루 2구간
    assert!(!overlaps((Some("3"), Some("3")), (Some("6"), None)));
}
