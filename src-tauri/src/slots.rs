//! 슬롯(교시) 순서와 부재 구간의 순수 로직. DB를 모른다.
//!
//! 슬롯 순서는 앱 상수다. 그날 몇 교시까지 있었는지는 **저장하지 않는다.**
//! 근거 — 나이스 월별 파일 분석에서 그날 슬롯 수가 1·3·6·7교시로 제각각이었다.
//! 단축수업·시험일 때문이다. 시간표를 들고 구간을 펼치는 설계였다면 그 날들에서
//! 전부 틀렸을 것이다. 열린 쪽은 NULL('*')로 두고 대조 시점에만 펼친다.

/// 조회 < 1교시 < … < 9교시 < 종례
pub const SLOTS: &[&str] = &[
    "조회", "1", "2", "3", "4", "5", "6", "7", "8", "9", "종례",
];

/// 슬롯 토큰의 순서값. 모르는 토큰이면 None.
pub fn ordinal(slot: &str) -> Option<usize> {
    SLOTS.iter().position(|s| *s == slot)
}

/// 화면·문구에 쓸 표기. `"5"` → `"5교시"`, `"조회"` → `"조회"`.
pub fn display(slot: &str) -> String {
    match slot {
        "조회" | "종례" => slot.to_string(),
        n => format!("{n}교시"),
    }
}

/// 구간 요약 표기. 열린 쪽은 `*`.
pub fn format_span(start: Option<&str>, end: Option<&str>) -> String {
    let s = start.unwrap_or("*");
    let e = end.unwrap_or("*");
    format!("{s} ~ {e}")
}

/// `attendance_type.slot_prompt`가 가질 수 있는 값.
///
/// 값 자체는 DB에 있다. 여기서는 저장 전에 알 수 없는 값이 들어오는 것만 막는다.
pub const SLOT_PROMPTS: &[&str] = &["none", "start", "end", "both"];

pub fn is_slot_prompt(value: &str) -> bool {
    SLOT_PROMPTS.contains(&value)
}

/// 구간의 유효성. 저장 직전에 부른다.
///
/// 프로그램은 판정하지 않는다는 원칙에 따라 "결석인데 5교시부터"처럼 유형과
/// 구간이 어긋나는 것은 막지 않는다. 여기서 막는 것은 **표현할 수 없는 구간**뿐이다.
pub fn validate_span(start: Option<&str>, end: Option<&str>) -> Result<(), String> {
    let s = match start {
        Some(v) => Some(ordinal(v).ok_or_else(|| format!("알 수 없는 시작 교시입니다: {v}"))?),
        None => None,
    };
    let e = match end {
        Some(v) => Some(ordinal(v).ok_or_else(|| format!("알 수 없는 끝 교시입니다: {v}"))?),
        None => None,
    };
    if let (Some(s), Some(e)) = (s, e) {
        if s > e {
            return Err(format!(
                "시작 교시가 끝 교시보다 뒤입니다: {} ~ {}",
                display(start.unwrap()),
                display(end.unwrap())
            ));
        }
    }
    Ok(())
}

/// 두 구간이 겹치는지. 열린 쪽은 각각 처음/끝으로 본다.
///
/// 하루 2구간은 정상 입력이므로 겹침을 **막지 않는다.** 화면에서 경고를 띄우는
/// 용도다(3교시 결과 + 3교시 조퇴 같은 실수).
pub fn overlaps(
    a: (Option<&str>, Option<&str>),
    b: (Option<&str>, Option<&str>),
) -> bool {
    let last = SLOTS.len() - 1;
    let range = |(s, e): (Option<&str>, Option<&str>)| {
        (
            s.and_then(ordinal).unwrap_or(0),
            e.and_then(ordinal).unwrap_or(last),
        )
    };
    let (a0, a1) = range(a);
    let (b0, b1) = range(b);
    a0 <= b1 && b0 <= a1
}
