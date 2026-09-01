//! 나이스 제출 문구 생성. 순수 함수다.
//!
//! 교사는 증상 단어만 친다. 나머지는 코드의 `phrase_pattern`과 구간에서 조합된다.
//! 생성된 문구는 `daily_reason.reason`에 그대로 저장되고 이후 수정 가능하다.
//! 즉 **생성은 초안일 뿐 진실이 아니다.** 저장된 문구를 다시 생성해 덮어쓰지 말 것.

use crate::slots;

/// 치환 자리표시자
const P_SYMPTOM: &str = "{증상}";
const P_START: &str = "{시작교시}";
const P_END: &str = "{끝교시}";

/// 조사 표기: (받침 있을 때, 받침 없을 때, ㄹ받침을 '없음'으로 볼지)
const JOSA: &[(&str, &str, &str, bool)] = &[
    ("(으)로", "으로", "로", true),
    ("(이)가", "이", "가", false),
    ("(을)를", "을", "를", false),
    ("(은)는", "은", "는", false),
    ("(과)와", "과", "와", false),
];

/// 한글 음절의 종성 인덱스. 한글이 아니면 None.
fn jongseong(ch: char) -> Option<u32> {
    let code = ch as u32;
    if (0xAC00..=0xD7A3).contains(&code) {
        Some((code - 0xAC00) % 28)
    } else {
        None
    }
}

/// 문자열 끝 글자의 받침 상태.
/// 한글이 아닌 글자(숫자·영문)로 끝나면 받침 없음으로 본다.
/// 숫자의 실제 발음(1=일, 받침 ㄹ)까지 따지지 않는 것은, 증상 단어가 숫자로
/// 끝나는 경우가 실무에 없기 때문이다.
fn tail_batchim(s: &str) -> (bool, bool) {
    match s.chars().last().and_then(jongseong) {
        Some(0) | None => (false, false),
        Some(8) => (true, true), // ㄹ
        Some(_) => (true, false),
    }
}

/// 문자열에 남아 있는 조사 표기를 앞 글자에 맞춰 확정한다.
pub fn apply_josa(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    'outer: while !rest.is_empty() {
        for (marker, with_b, without_b, l_as_without) in JOSA {
            if let Some(pos) = rest.find(marker) {
                // 표기 앞부분을 먼저 옮기고, 그 끝 글자로 받침을 판단한다.
                out.push_str(&rest[..pos]);
                let (has_batchim, is_rieul) = tail_batchim(&out);
                let chosen = if has_batchim && !(*l_as_without && is_rieul) {
                    with_b
                } else {
                    without_b
                };
                out.push_str(chosen);
                rest = &rest[pos + marker.len()..];
                continue 'outer;
            }
        }
        out.push_str(rest);
        break;
    }
    out
}

/// 값이 빈 자리표시자를 지운다. 바로 뒤에 붙은 조사 표기와 공백 하나까지 함께 지운다.
///
/// 증상을 아직 안 쳤을 때 `"(으)로 질병결석"` 같은 부스러기가 남지 않게 하려는 것이다.
fn drop_placeholder(text: &str, placeholder: &str) -> String {
    let Some(pos) = text.find(placeholder) else {
        return text.to_string();
    };
    let mut tail = &text[pos + placeholder.len()..];
    for (marker, _, _, _) in JOSA {
        if let Some(stripped) = tail.strip_prefix(marker) {
            tail = stripped;
            break;
        }
    }
    let tail = tail.strip_prefix(' ').unwrap_or(tail);
    format!("{}{}", &text[..pos], tail)
}

fn fill(text: &str, placeholder: &str, value: Option<&str>) -> String {
    match value {
        Some(v) if !v.is_empty() => text.replace(placeholder, v),
        _ => drop_placeholder(text, placeholder),
    }
}

/// 문구 초안을 만든다.
///
/// `pattern`이 없으면 코드 라벨만 돌려준다 — 문구 패턴은 선택 항목이고,
/// 패턴이 없는 코드에서 빈 문자열을 저장하면 나이스에 낼 것이 사라진다.
pub fn render(
    pattern: Option<&str>,
    label: &str,
    symptom: Option<&str>,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
) -> String {
    let pattern = match pattern {
        Some(p) if !p.trim().is_empty() => p,
        _ => return label.to_string(),
    };

    let start = start_slot.map(slots::display);
    let end = end_slot.map(slots::display);

    let mut out = fill(pattern, P_SYMPTOM, symptom);
    out = fill(&out, P_START, start.as_deref());
    out = fill(&out, P_END, end.as_deref());

    apply_josa(out.trim())
}
