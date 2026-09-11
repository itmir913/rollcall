//! `slots.rs` — 슬롯 순서와 부재 구간의 순수 로직.
//!
//! 여기서 확인하는 것은 두 가지다.
//!   · 최대 교시가 **학교 설정값**이라는 사실 — 모든 함수가 `max_slot`을 받고,
//!     같은 토큰이 학교에 따라 유효하기도 무효하기도 하다.
//!   · 프로그램이 판정하지 않는다는 원칙 — 표현할 수 없는 구간만 거부하고,
//!     종류와 구간이 어긋나는 것은 통과시킨다.
//!
//! 순서 · 표기 · 묶기는 프런트의 `services/slots.js`가 같은 규칙을 따로 구현한다.
//! 그래서 그 셋은 **`slot_vectors.json` 한 파일**을 양쪽 테스트가 읽어 비교한다.
//! 기대값을 양쪽에 손으로 베껴 두면 구현이 갈라질 때 테스트도 함께 갈라진다.
//! 나머지(구간 검사 · 겹침 · slot_prompt)는 Rust에만 있어 여기서만 확인한다.

use crate::slots::*;
use serde::Deserialize;

// ─── 고정 벡터 ─────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Vectors {
    slot_list: Vec<SlotListCase>,
    ordinal: Vec<OrdinalCase>,
    display: Vec<DisplayCase>,
    group_runs: Vec<GroupRunsCase>,
    divergences: Vec<Divergence>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SlotListCase {
    max_slot: usize,
    slots: Vec<String>,
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrdinalCase {
    slot: String,
    max_slot: usize,
    /// null이면 순서값이 없다는 뜻. JS는 같은 자리를 -1로 돌려준다.
    order: Option<usize>,
    note: Option<String>,
}

#[derive(Deserialize)]
struct DisplayCase {
    slot: String,
    label: String,
    note: Option<String>,
}

#[derive(Deserialize)]
struct GroupRunsCase {
    periods: Vec<usize>,
    runs: Vec<(usize, usize)>,
    note: Option<String>,
}

/// 두 구현이 실제로 다르게 답하는 자리. 합의된 값이 아니라 현재 값을 적어 둔 것이다.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Divergence {
    #[serde(rename = "fn")]
    func: String,
    slot: String,
    max_slot: usize,
    rust: Option<usize>,
    note: Option<String>,
}

fn vectors() -> Vectors {
    serde_json::from_str(include_str!("slot_vectors.json"))
        .expect("slot_vectors.json을 읽지 못했다")
}

/// 실패 메시지에 벡터의 설명을 함께 적는다. 어느 줄이 왜 있는지 모르면
/// 다음 사람이 기대값을 고쳐 통과시키는 쪽을 고른다.
fn why(note: &Option<String>) -> String {
    note.as_deref().map(|n| format!(" — {n}")).unwrap_or_default()
}

#[test]
fn no_vector_group_is_empty() {
    // 항목 이름이 바뀌면 아래 반복문이 조용히 한 번도 돌지 않고 통과한다.
    let v = vectors();
    assert!(!v.slot_list.is_empty(), "slotList 묶음이 비어 있다");
    assert!(!v.ordinal.is_empty(), "ordinal 묶음이 비어 있다");
    assert!(!v.display.is_empty(), "display 묶음이 비어 있다");
    assert!(!v.group_runs.is_empty(), "groupRuns 묶음이 비어 있다");
    assert!(!v.divergences.is_empty(), "divergences 묶음이 비어 있다");
}

// ─── 순서 ──────────────────────────────────────────────────────

#[test]
fn slot_list_matches_the_shared_vectors() {
    for case in vectors().slot_list {
        assert_eq!(
            slots(case.max_slot),
            case.slots,
            "max_slot={}의 목록이 고정 벡터와 다르다{}",
            case.max_slot,
            why(&case.note)
        );
    }
}

#[test]
fn ordinal_matches_the_shared_vectors() {
    for case in vectors().ordinal {
        assert_eq!(
            ordinal(&case.slot, case.max_slot),
            case.order,
            "max_slot={}에서 {:?}의 순서값이 고정 벡터와 다르다{}",
            case.max_slot,
            case.slot,
            why(&case.note)
        );
    }
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

// ─── 표기 ──────────────────────────────────────────────────────

#[test]
fn display_matches_the_shared_vectors() {
    for case in vectors().display {
        assert_eq!(
            display(&case.slot),
            case.label,
            "{:?}의 표기가 고정 벡터와 다르다{}",
            case.slot,
            why(&case.note)
        );
    }
}

#[test]
fn open_ends_render_as_a_question_mark() {
    // 저장은 NULL이고 화면에만 ?로 적는다.
    // `format_span`은 기계가 읽는 표기라 프런트에 짝이 없다.
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
fn group_runs_matches_the_shared_vectors() {
    // 이어지지 않은 것을 한 구간으로 저장하면 사이의 교시가 조용히 포함된다.
    for case in vectors().group_runs {
        assert_eq!(
            group_runs(case.periods.clone()),
            case.runs,
            "{:?}를 묶은 결과가 고정 벡터와 다르다{}",
            case.periods,
            why(&case.note)
        );
    }
}

// ─── 갈라진 자리 ───────────────────────────────────────────────

#[test]
fn recorded_divergences_still_behave_as_recorded() {
    // 두 구현이 다르게 답하는 자리를 벡터 파일에 적어 두었다. 이 테스트는 그 차이를
    // 옳다고 인정하는 것이 아니라, 어느 한쪽이 말없이 또 움직이는 것을 막는다.
    // 차이를 없앨 때는 벡터 파일의 divergences 항목을 ordinal 쪽으로 옮긴다.
    for case in vectors().divergences {
        assert_eq!(case.func, "ordinal", "아직 ordinal 말고는 기록한 것이 없다");
        assert_eq!(
            ordinal(&case.slot, case.max_slot),
            case.rust,
            "{:?}에 대한 Rust 쪽 값이 기록과 다르다{}",
            case.slot,
            why(&case.note)
        );
    }
}
