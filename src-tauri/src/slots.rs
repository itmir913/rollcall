//! 슬롯(교시) 순서와 부재 구간의 순수 로직. DB를 모른다.
//!
//! 하루는 `조회 · 1교시 … N교시 · 종례`로 이어진다. **N은 학교 설정값이다** —
//! 앱 상수가 아니다. 학교마다 다르고, 순회 교사가 학교를 둘 이상 등록하면
//! 학교마다 따로 가진다. 그래서 이 모듈의 함수는 대부분 `max_slot`을 받는다.
//!
//! 그날 몇 교시까지 있었는지는 **저장하지 않는다.**
//! 근거 — 나이스 월별 파일 분석에서 그날 슬롯 수가 1·3·6·7교시로 제각각이었다.
//! 단축수업·시험일 때문이다. 시간표를 들고 구간을 전개하는 설계였다면 그 날들에서
//! 전부 틀렸을 것이다. 열린 쪽은 NULL로 두고 대조 시점에만 전개한다.

pub const HOMEROOM: &str = "조회";
pub const CLOSING: &str = "종례";

/// 열린 쪽을 화면에 적을 때 쓰는 기호. 저장은 NULL이다.
pub const UNKNOWN: &str = "?";

/// 조회 < 1교시 < … < N교시 < 종례
///
/// **이 목록이 기준이다.** 프런트의 `services/slots.js`가 같은 순서를 따로 만드는데,
/// 두 구현이 달라지면 조용히 엉뚱한 기간이 저장된다. 그래서 양쪽에 같은 경계 테스트를
/// 두고, 기준을 여기에 남긴다. 지금은 테스트만 부르지만 지우지 않는다.
#[allow(dead_code)]
pub fn slots(max_slot: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(max_slot + 2);
    out.push(HOMEROOM.to_string());
    for n in 1..=max_slot {
        out.push(n.to_string());
    }
    out.push(CLOSING.to_string());
    out
}

/// 슬롯 토큰의 순서값. 모르는 토큰이면 None.
pub fn ordinal(slot: &str, max_slot: usize) -> Option<usize> {
    if slot == HOMEROOM {
        return Some(0);
    }
    if slot == CLOSING {
        return Some(max_slot + 1);
    }
    match slot.parse::<usize>() {
        Ok(n) if n >= 1 && n <= max_slot => Some(n),
        _ => None,
    }
}

/// 화면·문구에 쓸 표기. `"5"` → `"5교시"`, `"조회"` → `"조회"`.
pub fn display(slot: &str) -> String {
    match slot {
        HOMEROOM | CLOSING | UNKNOWN => slot.to_string(),
        n => format!("{n}교시"),
    }
}

/// 구간 요약 표기. 열린 쪽은 `?`.
///
/// 사람이 읽는 문구는 `attendance.rs`의 `span_text`가 만든다(조회부터 2교시까지).
/// 이것은 **기계가 읽는 짧은 표기**다 — 검증·내보내기에서 두 구간을 나란히 비교할 때 쓴다.
#[allow(dead_code)]
pub fn format_span(start: Option<&str>, end: Option<&str>) -> String {
    let s = start.unwrap_or(UNKNOWN);
    let e = end.unwrap_or(UNKNOWN);
    format!("{s} ~ {e}")
}

/// `attendance_type.slot_prompt`가 가질 수 있는 값.
///
/// 값 자체는 DB에 있다. 여기서는 저장 전에 알 수 없는 값이 들어오는 것만 막는다.
///   none  = 하루 종일이라 묻지 않는다 (결석)
///   start = 나간 때 하나 (조퇴)
///   end   = 온 때 하나 (지각)
///   multi = 빠진 교시 여러 개 (결과)
pub const SLOT_PROMPTS: &[&str] = &["none", "start", "end", "multi"];

pub fn is_slot_prompt(value: &str) -> bool {
    SLOT_PROMPTS.contains(&value)
}

/// 구간의 유효성. 저장 직전에 부른다.
///
/// 프로그램은 판정하지 않는다는 원칙에 따라 "결석인데 5교시부터"처럼 종류와
/// 구간이 어긋나는 것은 막지 않는다. 여기서 막는 것은 **표현할 수 없는 구간**뿐이다.
pub fn validate_span(
    start: Option<&str>,
    end: Option<&str>,
    max_slot: usize,
) -> Result<(), String> {
    let s = match start {
        Some(v) => Some(
            ordinal(v, max_slot).ok_or_else(|| format!("알 수 없는 시작 교시입니다: {v}"))?,
        ),
        None => None,
    };
    let e = match end {
        Some(v) => {
            Some(ordinal(v, max_slot).ok_or_else(|| format!("알 수 없는 끝 교시입니다: {v}"))?)
        }
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
/// 하루 2구간은 정상 입력이므로 겹침을 **막지 않는다.** 화면에서 표시만 하는
/// 용도다(3교시 결과 + 3교시 조퇴 같은 실수).
pub fn overlaps(
    a: (Option<&str>, Option<&str>),
    b: (Option<&str>, Option<&str>),
    max_slot: usize,
) -> bool {
    let last = max_slot + 1;
    let range = |(s, e): (Option<&str>, Option<&str>)| {
        (
            s.and_then(|v| ordinal(v, max_slot)).unwrap_or(0),
            e.and_then(|v| ordinal(v, max_slot)).unwrap_or(last),
        )
    };
    let (a_start, a_end) = range(a);
    let (b_start, b_end) = range(b);
    a_start <= b_end && b_start <= a_end
}

/// 결과처럼 여러 교시를 고른 경우, **연속한 것끼리 묶어** 구간으로 만든다.
///
/// `1,2,3` → `[(1,3)]`, `1,3,5` → `[(1,1),(3,3),(5,5)]`.
/// 연속하지 않은 것을 한 구간으로 저장하면 2교시가 조용히 포함된다.
pub fn group_runs(mut periods: Vec<usize>) -> Vec<(usize, usize)> {
    periods.sort_unstable();
    periods.dedup();
    let mut out: Vec<(usize, usize)> = Vec::new();
    for n in periods {
        match out.last_mut() {
            Some(last) if n == last.1 + 1 => last.1 = n,
            _ => out.push((n, n)),
        }
    }
    out
}
