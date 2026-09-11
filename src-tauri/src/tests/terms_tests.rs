//! 화면에 쓰는 말을 잠근다.
//!
//! Rust가 돌려주는 오류 문자열과 트리거의 `RAISE(ABORT, …)` 문구는 그대로 화면에
//! 표시된다. 현직 교사가 실제로 확인한 것은 세 가지다 — `맡은 것`은 교사가 쓰지 않는
//! 통칭이고, `출결을 찍다`와 `정하다`도 교무실에서 나오지 않는 말이다.
//!
//! 한 번 고쳐 놓기만 하면 곧 되돌아온다. 다음 사람이 옆 파일의 문장을 보고 따라 쓰기
//! 때문이다. 그래서 프런트의 `style.test.js`가 글자 크기를 강제하듯 여기서 낱말을
//! 강제한다. **이 파일만 검사 대상에서 뺀다** — 금칙어 목록 자체를 들고 있다.

use crate::commands::class::{homeroom_scope, homeroom_seat};
use crate::tests::{homeroom, insert_class, school_id, setup_test_db};
use std::fs;
use std::path::{Path, PathBuf};

/// 금칙어 목록을 **`terms.md`에서 읽는다.**
///
/// 목록을 여기에 적어 두면 `terms.md` · 이 파일 · `frontend/src/style.test.js` 세 곳이
/// 곧 갈라진다. 셋이 다른 말을 하면 어느 것이 규칙인지 아무도 모른다. 고칠 곳은
/// `terms.md`의 표 하나다.
///
/// 표의 모양은 `| `찾을 문자열` | 대신 쓸 말 |`이고 **백틱 안의 앞뒤 빈칸도 문자열의
/// 일부다** — `정하다`류는 앞 빈칸이 있어야 `판정하지` · `결정한다`가 걸리지 않는다.
fn banned() -> Vec<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(TERMS_FILE);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}를 읽지 못했다: {e}", path.display()));

    let section = text
        .split("## 4. 검사기가 읽는 목록")
        .nth(1)
        .unwrap_or_else(|| panic!("{TERMS_FILE}에 `## 4. 검사기가 읽는 목록` 절이 없다"));

    let mut out = Vec::new();
    for line in section.lines() {
        let line = line.trim();
        if !line.starts_with("| `") {
            continue;
        }
        let mut cells = line.trim_matches('|').split('|');
        let word = cells.next().unwrap_or("").trim();
        let hint = cells.next().unwrap_or("").trim();
        // 백틱 안쪽이 곧 찾을 문자열이다. `trim`으로 빈칸을 없애면 안 된다.
        if let Some(word) = word.strip_prefix('`').and_then(|w| w.strip_suffix('`')) {
            out.push((word.to_string(), hint.to_string()));
        }
    }
    assert!(
        out.len() > 10,
        "{TERMS_FILE}의 금칙어 표를 읽지 못했다. 표의 모양이 바뀌었는지 확인하라."
    );
    out
}

/// 용어집. 저장소 뿌리에 있다.
const TERMS_FILE: &str = "terms.md";

/// 검사할 확장자. 사람이 읽는 문장이 들어가는 파일 전부다.
const SCANNED: &[&str] = &["rs", "sql", "json"];

/// 이 파일은 금칙어 목록 자체라 검사하지 않는다.
const SELF_FILE: &str = "terms_tests.rs";

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, out);
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if SCANNED.contains(&ext) && name != SELF_FILE {
            out.push(path);
        }
    }
}

/// 소스 전체를 훑어 금칙어를 찾는다. 주석 · 문서 · 오류 문자열을 가리지 않는다 —
/// 주석의 낱말이 다음 오류 메시지의 낱말이 되기 때문이다.
#[test]
fn 교사가_쓰지_않는_말이_소스에_남아_있지_않다() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect(&root, &mut files);
    assert!(
        files.len() > 10,
        "검사한 파일이 너무 적다. 경로가 틀렸을 수 있다: {}",
        root.display()
    );

    let rules = banned();
    let mut hits: Vec<String> = Vec::new();
    for path in &files {
        let text = fs::read_to_string(path).unwrap();
        for (line_no, line) in text.lines().enumerate() {
            for (word, hint) in &rules {
                if line.contains(word) {
                    hits.push(format!(
                        "{}:{} [{}] → {}\n    {}",
                        path.file_name().unwrap().to_string_lossy(),
                        line_no + 1,
                        word.trim(),
                        hint,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        hits.is_empty(),
        "화면에 쓰지 않는 말이 남아 있다. 낱말만 바꾸고 뜻은 그대로 둘 것:\n{}",
        hits.join("\n")
    );
}

/// 역할을 바꾸려 하면 트리거가 거절한다. **그 문구가 화면에 그대로 표시된다.**
///
/// `맡은 것의 역할은…`이었다. 교사는 담임 학급과 교과 강좌를 묶어 부르지 않으므로
/// 둘을 나열한다.
#[test]
fn 역할_변경_거절_문구는_담당_학급_강좌로_말한다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);

    let err = conn
        .execute(
            "UPDATE teaching_class SET role = 'subject' WHERE id = ?1",
            rusqlite::params![class],
        )
        .unwrap_err()
        .to_string();

    assert!(
        err.contains("담당 학급 · 강좌의 역할은 바꿀 수 없습니다"),
        "{err}"
    );
    assert!(
        err.contains("새로 만들고 옛것을 마감하세요"),
        "거절만 하고 다음에 할 일을 말하지 않으면 교사는 화면에서 막힌다: {err}"
    );
}

/// 담임 학급에 학년 · 반이 비어 있으면 채우라고 말한다.
///
/// `정해져 있지 않습니다`였다. 같은 화면의 단추가 [설정]이므로 문장도 `설정`으로 쓴다 —
/// 말이 어긋나면 교사가 어느 화면으로 가야 하는지 한 번 더 생각한다.
#[test]
fn 학년_반이_빈_담임_학급은_설정하라고_말한다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = insert_class(&conn, school, "homeroom", "이름만 있는 반", None, None);

    let scope = homeroom_scope(&conn, class).unwrap();
    // `HomeroomSeat`은 만들 수 없는 타입이라 `Debug`도 없다. 그래서 match로 받는다.
    let err = match homeroom_seat(&scope) {
        Ok(_) => panic!("학년 · 반이 비어 있는데 학적 자리를 돌려주었다"),
        Err(e) => e,
    };

    assert!(err.contains("학년 · 반이 설정되어 있지 않습니다"), "{err}");
    assert!(err.contains("설정에서 먼저 채워주세요"), "{err}");
}
