//! 나이스 제출 문구 생성. 순수 함수다. DB를 모른다.
//!
//! 교사가 직접 치는 것은 **메모 한 칸**뿐이다. 나머지는 코드의 `phrase_pattern`과
//! 구간에서 조합된다.
//!
//! 생성된 문구는 **초안일 뿐 진실이 아니다.** 앱이 저장하는 것은 교사가 친
//! `absence_span.memo`이고, 문구는 화면과 파일 저장에서 그때그때 만든다.
//! 만들어진 문구로 메모를 덮어쓰지 말 것 — 교사가 고쳐 둔 말이 사라진다.

use crate::slots;

/// 치환 자리표시자
const P_MEMO: &str = "{메모}";
const P_START: &str = "{시작교시}";
const P_END: &str = "{끝교시}";

/// 조사 표기: (표기, 받침 있을 때, 받침 없을 때, ㄹ받침을 '없음'으로 볼지)
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
/// 숫자의 실제 발음(1=일, 받침 ㄹ)까지 따지지 않는 것은, 메모가 숫자로 끝나는
/// 경우가 실무에 거의 없기 때문이다.
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

    while !rest.is_empty() {
        // **가장 앞선 표기**를 선택한다. 표기 목록의 차례대로 찾으면, 뒤쪽 표기가 먼저
        // 걸릴 때 그 앞의 표기가 그대로 옮겨져 `서류(을)를 몸살로`가 된다.
        let next = JOSA
            .iter()
            .filter_map(|entry| rest.find(entry.0).map(|pos| (pos, entry)))
            .min_by_key(|(pos, _)| *pos);

        let Some((pos, (marker, with_b, without_b, l_as_without))) = next else {
            out.push_str(rest);
            break;
        };

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
    }
    out
}

/// 값이 빈 자리표시자를 지운다. 바로 뒤에 붙은 조사 표기와 공백 하나까지 함께 지운다.
///
/// 메모를 아직 안 쳤을 때 `"(으)로 질병결석"` 같은 부스러기가 남지 않게 하려는 것이다.
///
/// 값이 있을 때 `replace`가 **전부** 바꾸므로 지울 때도 전부 지운다. 한 번만 지우면
/// 같은 자리표시자를 두 번 쓴 패턴에서 남은 `{메모}`가 그대로 나이스 문구가 된다.
fn drop_placeholder(text: &str, placeholder: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(pos) = rest.find(placeholder) {
        out.push_str(&rest[..pos]);
        let mut tail = &rest[pos + placeholder.len()..];
        for (marker, _, _, _) in JOSA {
            if let Some(stripped) = tail.strip_prefix(marker) {
                tail = stripped;
                break;
            }
        }
        rest = tail.strip_prefix(' ').unwrap_or(tail);
    }
    out.push_str(rest);
    out
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
/// 패턴이 없는 코드에서 빈 문자열을 돌려주면 나이스에 낼 것이 사라진다.
///
/// 열린 구간(NULL)은 자리표시자째 지운다. `?`를 문구에 그대로 넣으면 그 문장이
/// 나이스에 그대로 올라간다.
pub fn render(
    pattern: Option<&str>,
    label: &str,
    memo: Option<&str>,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
) -> String {
    let pattern = match pattern {
        Some(p) if !p.trim().is_empty() => p,
        _ => return label.to_string(),
    };

    let start = start_slot.map(slots::display);
    let end = end_slot.map(slots::display);

    let mut out = fill(pattern, P_MEMO, memo);
    out = fill(&out, P_START, start.as_deref());
    out = fill(&out, P_END, end.as_deref());

    apply_josa(out.trim())
}
