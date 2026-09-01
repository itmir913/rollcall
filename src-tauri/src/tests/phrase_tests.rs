use crate::phrase::*;

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
fn full_day_absence_phrase() {
    let out = render(
        Some("{증상}(으)로 질병결석"),
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
        Some("{증상}(으)로 {시작교시}부터 질병조퇴"),
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
        Some("{증상}(으)로 {끝교시}까지 질병지각"),
        "질병지각",
        Some("감기"),
        None,
        Some("4"),
    );
    assert_eq!(out, "감기로 4교시까지 질병지각");
}

#[test]
fn homeroom_slot_keeps_its_own_label() {
    let out = render(
        Some("{증상}(으)로 {시작교시}부터 질병조퇴"),
        "질병조퇴",
        Some("두통"),
        Some("조회"),
        None,
    );
    assert_eq!(out, "두통으로 조회부터 질병조퇴");
}

#[test]
fn empty_symptom_drops_placeholder_and_its_josa() {
    // 증상을 아직 안 쳤을 때 "(으)로 질병결석" 같은 부스러기가 남지 않아야 한다.
    let out = render(Some("{증상}(으)로 질병결석"), "질병결석", None, None, None);
    assert_eq!(out, "질병결석");
}

#[test]
fn empty_slot_drops_placeholder() {
    let out = render(
        Some("{증상}(으)로 {시작교시}부터 질병조퇴"),
        "질병조퇴",
        Some("몸살"),
        None,
        None,
    );
    assert_eq!(out, "몸살로 부터 질병조퇴");
}

#[test]
fn missing_pattern_falls_back_to_label() {
    // 패턴이 없다고 빈 문자열을 저장하면 나이스에 낼 것이 사라진다.
    assert_eq!(render(None, "기타결석", Some("몸살"), None, None), "기타결석");
    assert_eq!(render(Some("  "), "기타결석", None, None, None), "기타결석");
}
