use crate::phrase::*;

// ── 조사 ──────────────────────────────────────────────────────

#[test]
fn josa_euro_follows_batchim() {
    assert_eq!(apply_josa("몸살(으)로"), "몸살로"); // ㄹ 받침 → 로
    assert_eq!(apply_josa("감기(으)로"), "감기로"); // 받침 없음 → 로
    assert_eq!(apply_josa("복통(으)로"), "복통으로"); // 받침 있음 → 으로
}

#[test]
fn josa_handles_other_markers() {
    assert_eq!(apply_josa("학생(이)가"), "학생이");
    assert_eq!(apply_josa("철수(이)가"), "철수가");
    assert_eq!(apply_josa("서류(을)를"), "서류를");
    assert_eq!(apply_josa("증상(을)를"), "증상을");
}

#[test]
fn josa_resolves_every_marker_in_one_string() {
    assert_eq!(apply_josa("몸살(으)로 서류(을)를"), "몸살로 서류를");
}

#[test]
fn non_hangul_tail_is_treated_as_no_batchim() {
    assert_eq!(apply_josa("COVID(으)로"), "COVID로");
}

#[test]
fn a_string_without_any_marker_is_left_alone() {
    assert_eq!(apply_josa("질병결석"), "질병결석");
    assert_eq!(apply_josa(""), "");
}

// ── 문구 ──────────────────────────────────────────────────────
//
// 자리표시자는 `{메모}`다. 교사가 치는 칸이 메모 하나이기 때문이다.

#[test]
fn full_day_absence_phrase() {
    let out = render(
        Some("{메모}(으)로 질병결석"),
        "질병결석",
        Some("몸살"),
        None,
        None,
    );
    assert_eq!(out, "몸살로 질병결석");
}

#[test]
fn early_leave_phrase_uses_start_slot() {
    let out = render(
        Some("{메모}(으)로 {시작교시}부터 질병조퇴"),
        "질병조퇴",
        Some("복통"),
        Some("5"),
        None,
    );
    assert_eq!(out, "복통으로 5교시부터 질병조퇴");
}

#[test]
fn late_phrase_uses_end_slot() {
    let out = render(
        Some("{메모}(으)로 {끝교시}까지 질병지각"),
        "질병지각",
        Some("감기"),
        None,
        Some("4"),
    );
    assert_eq!(out, "감기로 4교시까지 질병지각");
}

#[test]
fn homeroom_slot_keeps_its_own_label() {
    // 조회와 종례는 교시 번호가 아니다. "조회교시"가 되면 안 된다.
    let out = render(
        Some("{메모}(으)로 {시작교시}부터 질병조퇴"),
        "질병조퇴",
        Some("두통"),
        Some("조회"),
        None,
    );
    assert_eq!(out, "두통으로 조회부터 질병조퇴");

    let out = render(
        Some("{메모}(으)로 {끝교시}까지 질병지각"),
        "질병지각",
        Some("두통"),
        None,
        Some("종례"),
    );
    assert_eq!(out, "두통으로 종례까지 질병지각");
}

#[test]
fn empty_memo_drops_placeholder_and_its_josa() {
    // 메모를 아직 안 쳤을 때 "(으)로 질병결석" 같은 부스러기가 남지 않아야 한다.
    let out = render(Some("{메모}(으)로 질병결석"), "질병결석", None, None, None);
    assert_eq!(out, "질병결석");

    // 빈 문자열도 안 친 것과 같다.
    let out = render(Some("{메모}(으)로 질병결석"), "질병결석", Some(""), None, None);
    assert_eq!(out, "질병결석");
}

#[test]
fn an_open_span_drops_the_slot_placeholder() {
    // 열린 구간은 NULL이다. `?`를 문구에 넣으면 그 문장이 나이스에 그대로 올라간다.
    let out = render(
        Some("{메모}(으)로 {시작교시}부터 질병조퇴"),
        "질병조퇴",
        Some("몸살"),
        None,
        None,
    );
    assert!(!out.contains('?'), "열린 구간이 문구로 새어나왔다: {out}");
    assert!(out.starts_with("몸살로"));
}

#[test]
fn a_placeholder_written_twice_is_dropped_everywhere() {
    // 값이 있을 때는 전부 바뀌므로 빈 값일 때도 전부 지워져야 한다.
    // 하나라도 남으면 `{메모}`가 적힌 문장이 그대로 나이스에 올라간다.
    let out = render(
        Some("{메모} — {메모}(으)로 질병결석"),
        "질병결석",
        None,
        None,
        None,
    );
    assert!(!out.contains('{'), "자리표시자가 남았다: {out}");
}

#[test]
fn missing_pattern_falls_back_to_label() {
    // 패턴이 없다고 빈 문자열을 돌려주면 나이스에 낼 것이 사라진다.
    assert_eq!(render(None, "기타결석", Some("몸살"), None, None), "기타결석");
    assert_eq!(render(Some("  "), "기타결석", None, None, None), "기타결석");
}

#[test]
fn a_memo_written_as_a_full_sentence_goes_in_as_is() {
    // 메모는 자유 문장이고 앱은 해석하지 않는다.
    let out = render(
        Some("{메모}(으)로 질병결석"),
        "질병결석",
        Some("병원 진료 예정"),
        None,
        None,
    );
    assert_eq!(out, "병원 진료 예정으로 질병결석");
}

/// 한 문장에 조사 표기가 둘 이상이면 **앞선 것부터** 바꿔야 한다.
/// 표기 목록의 차례대로 찾으면 뒤쪽 표기가 먼저 걸려 앞의 표기가 그대로 남는다.
#[test]
fn every_josa_marker_in_one_sentence_is_replaced() {
    assert_eq!(apply_josa("서류(을)를 몸살(으)로"), "서류를 몸살로");
    assert_eq!(apply_josa("감기(으)로 결석(을)를"), "감기로 결석을");
    assert_eq!(apply_josa("학생(은)는 복통(으)로"), "학생은 복통으로");
}
