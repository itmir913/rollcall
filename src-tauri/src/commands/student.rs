//! 학생 명단과 연락처.
//!
//! 명렬표가 (학년, 반, 번호, 이름)이므로 **학급 정보는 명렬표에서 나온다.** 첫 실행에서
//! "우리 반이 몇 학년 몇 반입니까"를 따로 묻지 않으려는 것이다. 두 열만 들어 있는
//! 파일이면 화면이 고른 학급을 쓴다.
//!
//! **파일 읽기는 프론트에서 한다**(`frontend/src/services/rosterFile.js`). xlsx를 다룰 라이브러리가
//! 거기 있기 때문이고, 그것은 파일 형식 문제지 업무 규칙이 아니다. 여기로 넘어오는
//! 것은 이미 (학년, 반, 번호, 이름)으로 정리된 목록이다.
//!
//! **명렬표 가져오기는 두 가지 일을 한다.** 학생을 만드는 일과 내 명단에 넣는 일이다.
//! 둘은 다른 사건이라 표도 다르다 — 학생은 학교에 속하고(`student`), 소속은 따로
//! 기록된다(`class_member`). 이미 그 학적 자리에 있는 학생은 다시 만들지 않고
//! 명단에만 연결한다. 담임 반 학생이 내 교과 강좌에도 들어오면 학생 행은 하나,
//! 소속이 둘이어야 하기 때문이다.
//!
//! 재가져오기는 **교체가 아니라 차분**이다. 사라진 번호를 지우면 그 학생의 출결
//! 기록이 FK CASCADE로 함께 사라진다. 명단에서 사라진 번호는 `class_member.left_on`을
//! 채운다 — **학생의 `enrolled_to`가 아니다.** 내 명단에서 빠진 것과 학교를 떠난 것은
//! 다른 일이고, 담임이 아는 것은 앞엣것뿐이다.
//!
//! **차분의 열쇠는 역할이 정한다.** 담임은 `번호` 하나이고, 교과는 `(학년, 반, 번호)`
//! 학적 자리 전체다. 교과 강좌는 선택과목이라 1반~n반이 섞여 번호 하나로는 학생을
//! 구별할 수 없다 — 3학년 1반 4번과 3학년 6반 4번이 같은 강좌에 있다.
//!
//! **담임 열쇠를 자리로 넓히지 않는다.** 담임 명렬표는 반이 다른 학생을 막지 않으므로
//! (3학년 6반 명단에 3학년 7반 12번 학생이 있다), 열쇠를 자리로 넓히면 파일의 12번 줄이
//! 그 학생과 짝을 잃어 재가져오기마다 전출 + 중복 학적이 된다.

use crate::commands::class::{
    class_scope, homeroom_scope, homeroom_seat, members_of, ClassScope, HomeroomSeat,
};
use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::{format_date, parse_date};
use crate::state::{constraint_err, DbState};
use crate::types::*;
use rusqlite::Connection;
use tauri::State;

// ── 명렬표가 말하는 학급 ──────────────────────────────────────

/// 들어온 명단이 어느 학급인지. 전부 같아야 확정이고, 섞여 있으면 교사가 고른다.
///
/// 파일에 학년·반 열이 없으면 둘 다 None이다. 그때는 화면이 학급을 묻는다 —
/// 프로그램이 추측해서 엉뚱한 반에 넣는 것보다 낫다.
pub fn detect_class(entries: &[RosterEntry]) -> RosterClass {
    let grades: Vec<i64> = entries.iter().filter_map(|e| e.grade).collect();
    let classes: Vec<i64> = entries.iter().filter_map(|e| e.class_no).collect();

    let uniform = |v: &[i64]| -> Option<i64> {
        let first = *v.first()?;
        v.iter().all(|x| *x == first).then_some(first)
    };

    let grade = uniform(&grades);
    let class_no = uniform(&classes);
    let mixed =
        (!grades.is_empty() && grade.is_none()) || (!classes.is_empty() && class_no.is_none());

    RosterClass {
        grade,
        class_no,
        mixed,
    }
}

// ── 열쇠 ──────────────────────────────────────────────────────

/// 명렬표 한 줄이 어떤 학생을 가리키는가. **역할이 정한다.**
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RosterKeying {
    /// 담임 — `번호` 하나가 열쇠다. 반이 다른 학생이 명단에 있어도 번호로 짝을 찾는다.
    ByNumber,
    /// 교과 — `(학년, 반, 번호)` 학적 자리 전체가 열쇠다. 반이 섞이기 때문이다.
    BySeat,
}

impl RosterKeying {
    pub fn of(role: &str) -> Self {
        if role == "subject" {
            Self::BySeat
        } else {
            Self::ByNumber
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    Number(i64),
    Seat(i64, i64, i64),
}

fn key_of_member(k: RosterKeying, st: &StudentItem) -> Key {
    match k {
        RosterKeying::ByNumber => Key::Number(st.number),
        RosterKeying::BySeat => Key::Seat(st.grade, st.class_no, st.number),
    }
}

/// 파일 한 줄의 열쇠. 교과에서 학년 · 반이 비면 자리를 정할 수 없어 `None`이다.
fn key_of_entry(k: RosterKeying, e: &RosterEntry) -> Option<Key> {
    match k {
        RosterKeying::ByNumber => Some(Key::Number(e.number)),
        RosterKeying::BySeat => match (e.grade, e.class_no) {
            (Some(grade), Some(class_no)) => Some(Key::Seat(grade, class_no, e.number)),
            _ => None,
        },
    }
}

// ── 자리를 말하는 문구 ────────────────────────────────────────

/// `"3학년 1반 4번"`. 충돌하는 것은 번호가 아니라 자리다.
fn where_at(grade: i64, class_no: i64, number: i64) -> String {
    format!("{grade}학년 {class_no}반 {number}번")
}

/// 그 줄이 가리키는 자리. 자리를 모르면 번호만 말한다.
fn where_of(row: &RosterDiffRow) -> String {
    match (row.grade, row.class_no) {
        (Some(grade), Some(class_no)) => where_at(grade, class_no, row.number),
        _ => format!("{}번", row.number),
    }
}

fn where_of_entry(entry: &RosterEntry) -> String {
    match (entry.grade, entry.class_no) {
        (Some(grade), Some(class_no)) => where_at(grade, class_no, entry.number),
        _ => format!("{}번", entry.number),
    }
}

// ── 차분 ──────────────────────────────────────────────────────

/// 지금 명단과 들어온 명렬표를 비교한다. DB를 건드리지 않는 순수 함수다.
///
/// - 새 열쇠 → `added`
/// - 사라진 열쇠 → `withdrawn` (삭제가 아니라 전출)
/// - 열쇠 같고 이름 다름 → `renamed`. **판정할 수 없으므로 교사가 고른다.**
///   전학 온 학생이 번호를 물려받은 것일 수도, 개명일 수도, 명렬표 오타일 수도 있다.
/// - 열쇠를 만들지 못한 줄과 파일 안에서 열쇠가 겹치는 줄 → `blocked`
///
/// **읽지 못한 줄이 하나라도 있으면 전출을 자동으로 표시하지 않는다.** 그 줄이 가리키던
/// 학생이 짝을 잃어 전출로 잡히기 때문이다 — 교사가 "한 줄만 못 넣었구나" 하고 저장하면
/// 그 학생이 명단에서 빠진다. 줄은 그대로 보여주고, 켜는 것은 교사가 한다.
pub fn diff_roster(
    keying: RosterKeying,
    current: &[StudentItem],
    incoming: &[RosterEntry],
) -> Vec<RosterDiffRow> {
    let keys: Vec<Option<Key>> = incoming.iter().map(|e| key_of_entry(keying, e)).collect();

    // 파일 안에서 같은 자리가 두 줄이면 둘 다 넘긴다. 교과에서 번호 중복은 정상이지만
    // 자리 중복은 파일 오류다. 그대로 두면 둘째 줄이 `ux_student_seat`에 걸려
    // 트랜잭션이 통째로 롤백된다 — 서른 줄을 확인한 교사에게 남는 것은 에러 하나뿐이다.
    let duplicated: Vec<bool> = keys
        .iter()
        .enumerate()
        .map(|(i, key)| match key {
            Some(k) => keys
                .iter()
                .enumerate()
                .any(|(j, other)| j != i && *other == Some(*k)),
            None => false,
        })
        .collect();

    let mut rows: Vec<RosterDiffRow> = Vec::new();

    for (i, entry) in incoming.iter().enumerate() {
        let seen = RosterDiffRow {
            key: 0,
            grade: entry.grade,
            class_no: entry.class_no,
            line: entry.line,
            number: entry.number,
            incoming_name: Some(entry.name.clone()),
            current_name: None,
            student_id: None,
            action: "blocked".into(),
            why: None,
        };

        match (keys[i], duplicated[i]) {
            (None, _) => {
                rows.push(RosterDiffRow {
                    why: Some(format!(
                        "학년 · 반이 비어 있어 어느 반의 {}번인지 구별할 수 없습니다. \
                         파일에서 그 줄을 채운 뒤 다시 가져오세요.",
                        entry.number
                    )),
                    ..seen
                });
            }
            (Some(_), true) => {
                rows.push(RosterDiffRow {
                    why: Some(format!(
                        "같은 자리({})가 파일에 두 줄 이상 있습니다. \
                         한 자리에 두 학생일 수 없으므로 파일을 확인하세요.",
                        where_of_entry(entry)
                    )),
                    ..seen
                });
            }
            (Some(key), false) => {
                let matched = current.iter().find(|st| key_of_member(keying, st) == key);
                rows.push(match matched {
                    Some(st) if st.name == entry.name => RosterDiffRow {
                        current_name: Some(st.name.clone()),
                        student_id: Some(st.id),
                        action: "unchanged".into(),
                        ..seen
                    },
                    Some(st) => RosterDiffRow {
                        current_name: Some(st.name.clone()),
                        student_id: Some(st.id),
                        action: "renamed".into(),
                        ..seen
                    },
                    None => RosterDiffRow {
                        action: "added".into(),
                        ..seen
                    },
                });
            }
        }
    }

    let live: Vec<Key> = keys
        .iter()
        .enumerate()
        .filter(|(i, _)| !duplicated[*i])
        .filter_map(|(_, key)| *key)
        .collect();

    for st in current {
        if live.contains(&key_of_member(keying, st)) {
            continue;
        }
        let gone = RosterDiffRow {
            key: 0,
            grade: Some(st.grade),
            class_no: Some(st.class_no),
            line: None,
            number: st.number,
            incoming_name: None,
            current_name: Some(st.name.clone()),
            student_id: Some(st.id),
            action: "withdrawn".into(),
            why: None,
        };
        rows.push(gone);
    }

    hold_withdrawals(&mut rows);

    match keying {
        RosterKeying::ByNumber => rows.sort_by_key(|r| r.number),
        RosterKeying::BySeat => rows.sort_by_key(|r| (r.grade, r.class_no, r.number)),
    }
    for (i, row) in rows.iter_mut().enumerate() {
        row.key = i;
    }
    rows
}

/// **읽지 못한 줄이 있으면 전출을 자동으로 표시하지 않는다.**
///
/// 건너뛴 줄은 차분에서 중립이 아니다. 짝 찾기에 참여하지 못하므로 그 줄이 가리키던
/// 학생이 짝을 잃고 전출로 잡히고, 교사가 "한 줄만 못 넣었구나" 하고 저장하면 그 학생이
/// 명단에서 빠진다. 파일이 정말 뺀 것인지 그 줄이 그 학생이었는지 알 수 없으므로,
/// 줄은 그대로 보여주고 켜는 것은 교사가 한다.
///
/// **두 번 부른다.** 차분이 막은 줄(자리를 읽지 못함 · 파일 안 자리 중복)과
/// `annotate_seats`가 막은 줄(그 자리를 다른 이름이 쓰고 있음)이 **같은 규칙을 받아야**
/// 하기 때문이다. 뒤엣것도 짝을 빼앗는다 — 파일의 반 오타 한 글자로 `3학년 6반 4번`이
/// `3학년 1반 4번`이 되면, 그 줄은 남의 자리라 막히고 내 명단의 그 학생은 짝을 잃는다.
/// 이미 낮춘 줄에는 `withdrawn`이 남아 있지 않으므로 다시 불러도 안전하다.
fn hold_withdrawals(rows: &mut [RosterDiffRow]) {
    if !rows.iter().any(|r| r.action == "blocked") {
        return;
    }
    for row in rows.iter_mut().filter(|r| r.action == "withdrawn") {
        row.action = "unchanged".into();
        row.why = Some(
            "파일에서 읽지 못한 줄이 있어 명단에서 뺀 것으로 표시하지 않았습니다. \n             그 줄을 고쳐 다시 가져오거나, 이 학생을 직접 [내 명단에서 뺌]으로 바꾸세요."
                .to_string(),
        );
    }
}

// ── DB ────────────────────────────────────────────────────────

/// 지금 내 명단. 소속은 `class_member`가 말하므로 학년 · 반으로 거르지 않는다.
/// 그 학급의 명단. **역할을 가리지 않는다** — 교과 강좌도 명단을 가진다.
/// 구분되는 것은 기록이지 명단이 아니다.
///
/// 교과 강좌만 학적 자리 순으로 다시 정렬한다. 반이 섞여 번호 순으로는 1반 4번과
/// 6반 4번이 나란히 나타나기 때문이다. **`members_of`의 SQL은 번호 순 그대로 둔다** —
/// 담임 격자와 나이스 가져오기가 그 순서를 전제한다.
pub fn get_students_impl(conn: &Connection, class_id: i64) -> Result<Vec<StudentItem>, String> {
    let scope = class_scope(conn, class_id)?;
    let mut rows = members_of(conn, scope.id)?;
    if scope.role == "subject" {
        rows.sort_by_key(|s| (s.grade, s.class_no, s.number, s.id));
    }
    Ok(rows)
}

/// 명렬표 차분. 담임 학급과 교과 강좌가 같은 길을 쓰고, **열쇠만 달라진다.**
pub fn preview_roster_impl(
    conn: &Connection,
    class_id: i64,
    incoming: &[RosterEntry],
) -> Result<Vec<RosterDiffRow>, String> {
    let scope = class_scope(conn, class_id)?;
    let current = members_of(conn, scope.id)?;
    let mut rows = diff_roster(RosterKeying::of(&scope.role), &current, incoming);
    annotate_seats(conn, &scope, &mut rows)?;
    // **자리를 맞춰 본 뒤에 한 번 더 판단한다.** `annotate_seats`가 막은 줄도 짝을
    // 빼앗으므로, 차분 단계에서만 판단하면 그 줄이 가리키던 학생이 전출로 남는다.
    hold_withdrawals(&mut rows);
    Ok(rows)
}

/// 배치할 자리를 **읽기만** 하며 줄에 말을 붙인다. 저장 전에 교사가 알아야 하는 것들이다.
///
/// 자리가 비어 있으면 그대로 둔다(학적을 새로 만든다). 같은 이름이 있으면 그 학생을
/// 다시 만들지 않고 명단에만 연결한다고 알린다 — **`student_id`를 채우지 않는다.**
/// 화면의 토글이 `studentId` 유무로 구분되기 때문이다.
///
/// 다른 이름이 있으면 두 모드가 달라진다. 담임은 자리를 넘겨받되 저장 전에 알리고,
/// 교과는 넘겨받을 수 없으므로 `blocked`로 표시한다. 같은 학생이 맞다면 교사가
/// [그 학생이 맞습니다]를 눌러 `linked`로 바꾼다.
fn annotate_seats(
    conn: &Connection,
    scope: &ClassScope,
    rows: &mut [RosterDiffRow],
) -> Result<(), String> {
    // 담임 학급의 학년 · 반이 비어 있으면 붙일 말이 없다. 저장할 때 거절된다.
    let home: Option<HomeroomSeat> = homeroom_seat(scope).ok();

    for row in rows.iter_mut() {
        if row.action != "added" {
            continue;
        }
        // **담임은 줄에 적힌 학년 · 반을 읽지 않는다.** 학급이 들고 있는 값으로만 배치한다.
        let (grade, class_no) = match (&home, row.grade, row.class_no) {
            (Some(seat), _, _) => seat.at(),
            (None, Some(grade), Some(class_no)) if scope.role == "subject" => (grade, class_no),
            _ => continue,
        };
        let Some((id, held)) = seat_holder(conn, scope.school_id, grade, class_no, row.number)?
        else {
            continue;
        };

        let name = row.incoming_name.as_deref().unwrap_or("").trim();
        let at = where_at(grade, class_no, row.number);
        if held == name {
            row.why =
                Some("이미 있는 학생입니다. 학적을 만들지 않고 명단에만 연결합니다.".to_string());
        } else if scope.role == "subject" {
            row.action = "blocked".into();
            row.student_id = Some(id);
            row.why = Some(format!(
                "{at} 자리에는 「{held}」이(가) 있습니다. 파일의 학년 · 반이 맞는지 확인하세요. \
                 같은 학생이 맞으면 [그 학생이 맞습니다]를 눌러 학적을 만들지 않고 명단에만 연결합니다."
            ));
        } else {
            row.why = Some(format!(
                "{at} 자리에 「{held}」이(가) 있습니다. \
                 저장하면 그 학적을 마감하고 「{name}」을(를) 새로 등록합니다."
            ));
        }
    }
    Ok(())
}

// ── 학적 자리와 소속 ──────────────────────────────────────────

/// 그 학적 자리를 이미 쓰고 있는 학생. `ux_student_seat`이 가리키는 한 자리다.
///
/// **학년도로 다시 거르지 않는다.** 학교가 이미 학년도 안에 있어, 해가 바뀌면
/// 학교가 새로 만들어지고 자리도 저절로 새로 열린다.
fn seat_holder(
    conn: &Connection,
    school_id: i64,
    grade: i64,
    class_no: i64,
    number: i64,
) -> Result<Option<(i64, String)>, String> {
    conn.query_row(
        "SELECT id, name FROM student
          WHERE school_id = ?1 AND grade = ?2 AND class_no = ?3
            AND number = ?4 AND enrolled_to IS NULL
          ORDER BY id LIMIT 1",
        rusqlite::params![school_id, grade, class_no, number],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other.to_string()),
    })
}

/// 학생을 내 명단에 연결한다. 예전에 있다가 빠진 학생이면 `left_on`을 지워 되살린다.
///
/// 지웠다 다시 넣지 않는 이유는 소속 줄에 달린 것이 없어도 줄 자체가 기록이기 때문이다 —
/// 언제부터 내 명단이었는지가 남아야 지난 기록이 어느 명단의 것이었는지 말할 수 있다.
fn join_class(
    conn: &Connection,
    class_id: i64,
    student_id: i64,
    joined_on: &str,
) -> Result<(), String> {
    // 명단에 되돌아온 학생은 `left_on`만 지운다.
    //
    // **빠져 있던 기간은 남지 않는다.** `class_member`가 (학급, 학생) 한 쌍에 한 줄이라
    // 소속 기간을 여러 구간으로 담을 수 없기 때문이다. 4월에 빠졌다가 6월에 돌아오면
    // 5월 격자에도 그 학생이 나타난다. 소속을 구간으로 바꾸면 모든 명단 질의가 날짜마다
    // 어느 구간인지 골라야 해서, 실제로 그런 일이 생기는 것을 보기 전에는 값이 비싸다.
    // 그때 고칠 자리는 이 함수와 `class.rs`의 명단 조건 둘뿐이다.
    let revived = conn
        .execute(
            "UPDATE class_member SET left_on = NULL
              WHERE class_id = ?1 AND student_id = ?2 AND left_on IS NOT NULL",
            rusqlite::params![class_id, student_id],
        )
        .map_err(|e| e.to_string())?;
    if revived > 0 {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO class_member (class_id, student_id, joined_on) VALUES (?1, ?2, ?3)
         ON CONFLICT(class_id, student_id) DO NOTHING",
        rusqlite::params![class_id, student_id, joined_on],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 자리를 넘겨받으며 앞사람의 학적을 마감한다. **이 변경에서 되돌릴 수 없는 유일한 쓰기다.**
///
/// `&HomeroomSeat`을 받는 것이 요점이다. 그 값은 `class.rs`의 `homeroom_seat`만 만들 수
/// 있어 교과 경로에서는 얻을 수 없고, 따라서 교과가 이 함수를 부를 수 없다. 교과 파일의
/// 반 오타 하나가 남의 반 학생을 전출시키는 일을 타입으로 막는다.
fn hand_over_seat(
    conn: &Connection,
    seat: &HomeroomSeat,
    holder: i64,
    date: &str,
) -> Result<(), String> {
    let (grade, class_no) = seat.at();
    let n = conn
        .execute(
            "UPDATE student SET enrolled_to = ?1
              WHERE id = ?2 AND grade = ?3 AND class_no = ?4 AND enrolled_to IS NULL",
            rusqlite::params![date, holder, grade, class_no],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("자리를 넘겨받을 학적을 찾을 수 없습니다: {holder}"));
    }
    Ok(())
}

/// 미리보기에서 교사가 확정한 행만 받아 적용한다.
///
/// 프론트가 `action`을 바꿔 보낼 수 있다는 것이 요점이다. 번호 같고 이름 다름을
/// 개명(`renamed`)으로 볼지, 명단 교체(`withdrawn` + `added`)로 볼지는 교사가 정한다.
/// 자리를 다른 이름이 쓰고 있는 줄을 `linked`로 바꾸는 것도 교사다.
pub fn apply_roster_impl(
    conn: &Connection,
    class_id: i64,
    effective_date: &str,
    rows: &[RosterDiffRow],
) -> Result<RosterApplyResult, String> {
    // **형식을 확인하고 자리를 채운 ISO로 맞춘다.** `class_member`의 날짜 비교가
    // 문자열 비교라, `2026-9-1`이 그대로 들어가면 `joined_on <= 날짜`가 어긋나
    // 그 학급이 조용히 빈 명단으로 표시된다. 학년도 밖의 날짜는 그대로 받는다 —
    // 학년도는 기준 연도일 뿐 날짜 울타리가 아니다.
    let effective_date = &format_date(parse_date(effective_date)?);
    let scope = class_scope(conn, class_id)?;
    // 담임일 때만 자리를 넘겨받을 **권한을 한 번 얻는다.** 교과는 None이다.
    let home: Option<HomeroomSeat> = match scope.role.as_str() {
        "homeroom" => Some(homeroom_seat(&scope)?),
        _ => None,
    };
    // 범위는 학교 하나다. 학년도는 그 학교가 들고 있다.
    let school_id = scope.school_id;

    with_transaction(conn, || {
        let mut result = RosterApplyResult {
            added: 0,
            created: 0,
            renamed: 0,
            withdrawn: 0,
            blocked: 0,
            seat_closed: 0,
        };

        // 명단에서 빼는 것을 먼저 처리한다. 같은 번호를 새 학생이 물려받는 경우,
        // 학적 자리를 비우는 순서가 강제되기 때문이다.
        for row in rows.iter().filter(|r| r.action == "withdrawn") {
            let id = row.student_id.ok_or_else(|| {
                format!("{}: 명단에서 뺄 학생을 찾을 수 없습니다.", where_of(row))
            })?;
            // **학생의 `enrolled_to`를 채우지 않는다.** 담임도 교과도 건드리지 않는다 —
            // 내 명단에서 빠진 것과 학교를 떠난 것은 다른 일이고, 아는 것은 앞엣것뿐이다.
            let n = conn
                .execute(
                    "UPDATE class_member SET left_on = ?1
                      WHERE class_id = ?2 AND student_id = ?3 AND left_on IS NULL",
                    rusqlite::params![effective_date, scope.id, id],
                )
                .map_err(|e| e.to_string())?;
            if n == 0 {
                return Err(format!(
                    "{}: 명단에서 뺄 학생을 찾을 수 없습니다: {id}",
                    where_of(row)
                ));
            }
            result.withdrawn += 1;
        }

        for row in rows {
            match row.action.as_str() {
                "added" => {
                    let name = row.incoming_name.as_deref().unwrap_or("").trim();
                    if name.is_empty() {
                        return Err(format!("{}: 추가할 이름이 비어 있습니다.", where_of(row)));
                    }
                    // 담임은 학급이 들고 있는 자리, 교과는 그 줄이 적어 온 자리다.
                    let (grade, class_no) = match (&home, row.grade, row.class_no) {
                        (Some(seat), _, _) => seat.at(),
                        (None, Some(grade), Some(class_no)) => (grade, class_no),
                        (None, _, _) => {
                            return Err(format!(
                                "{}: 학년 · 반이 없어 학적 자리를 정할 수 없습니다.",
                                where_of(row)
                            ))
                        }
                    };

                    // **이미 있는 학생을 다시 만들지 않는다.** 같은 학적 자리에 같은
                    // 이름이 있으면 그 학생이고(다른 명단에서 들어온 경우다),
                    // 명단에만 연결하면 된다.
                    let holder = seat_holder(conn, school_id, grade, class_no, row.number)?;
                    let student_id = match holder {
                        Some((id, held)) if held == name => id,
                        Some((id, held)) => {
                            // 같은 자리를 다른 이름이 쓰고 있다.
                            let Some(seat) = home.as_ref() else {
                                // 교과는 남의 반 학적을 마감할 수 없다. **조용히 넘기지 않는다.**
                                return Err(format!(
                                    "{} 자리에는 「{held}」이(가) 있습니다. \
                                     교과 강좌는 남의 반 학적을 마감할 수 없습니다. \
                                     파일의 학년 · 반을 확인하거나, 같은 학생이 맞으면 \
                                     [그 학생이 맞습니다]를 눌러 명단에만 연결하세요.",
                                    where_at(grade, class_no, row.number)
                                ));
                            };
                            // 담임은 한 자리에 두 명일 수 없으므로 앞사람이 그 자리를
                            // 떠난 것으로 본다 — 학적을 마감해 자리를 비운다. 교사의
                            // 판단을 대신하는 것이 아니라, 가리킬 수 없는 행이 생기지
                            // 않게 하는 것이다.
                            hand_over_seat(conn, seat, id, effective_date)?;
                            result.seat_closed += 1;
                            result.created += 1;
                            insert_student(
                                conn,
                                school_id,
                                grade,
                                class_no,
                                row.number,
                                name,
                                effective_date,
                            )?
                        }
                        None => {
                            result.created += 1;
                            insert_student(
                                conn,
                                school_id,
                                grade,
                                class_no,
                                row.number,
                                name,
                                effective_date,
                            )?
                        }
                    };
                    join_class(conn, scope.id, student_id, effective_date)?;
                    result.added += 1;
                }
                "linked" => {
                    // 교사가 "그 학생이 맞습니다"를 누른 줄. **학적을 건드리지 않고**
                    // 명단에만 연결한다. 화면이 보낸 `student_id`를 그대로 믿지 않고
                    // 그 학교의 재학생인지 확인한다.
                    let id = row
                        .student_id
                        .ok_or_else(|| format!("{}: 연결할 학생이 없습니다.", where_of(row)))?;
                    let found: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM student
                              WHERE id = ?1 AND school_id = ?2 AND enrolled_to IS NULL",
                            rusqlite::params![id, school_id],
                            |r| r.get(0),
                        )
                        .map_err(|e| e.to_string())?;
                    if found == 0 {
                        return Err(format!(
                            "{}: 연결할 학생을 찾을 수 없습니다: {id}",
                            where_of(row)
                        ));
                    }
                    join_class(conn, scope.id, id, effective_date)?;
                    result.added += 1;
                }
                "renamed" => {
                    let id = row.student_id.ok_or_else(|| {
                        format!("{}: 이름을 고칠 학생을 찾을 수 없습니다.", where_of(row))
                    })?;
                    let name = row
                        .incoming_name
                        .as_deref()
                        .map(str::trim)
                        .filter(|n| !n.is_empty())
                        .ok_or_else(|| format!("{}: 새 이름이 비어 있습니다.", where_of(row)))?;
                    let n = conn
                        .execute(
                            "UPDATE student SET name = ?1 WHERE id = ?2 AND school_id = ?3",
                            rusqlite::params![name, id, school_id],
                        )
                        .map_err(|e| e.to_string())?;
                    if n == 0 {
                        return Err(format!(
                            "{}: 이름을 고칠 학생을 찾을 수 없습니다: {id}",
                            where_of(row)
                        ));
                    }
                    result.renamed += 1;
                }
                // 배치할 수 없어 넘긴 줄. 아무것도 쓰지 않되 **센다** —
                // 조용히 사라지면 교사가 몇 줄이 빠졌는지 알 방법이 없다.
                "blocked" => result.blocked += 1,
                // unchanged / withdrawn(위에서 처리) 은 여기서 할 일이 없다.
                _ => {}
            }
        }

        Ok(result)
    })
}

#[allow(clippy::too_many_arguments)]
fn insert_student(
    conn: &Connection,
    school_id: i64,
    grade: i64,
    class_no: i64,
    number: i64,
    name: &str,
    enrolled_from: &str,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO student
           (school_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![school_id, grade, class_no, number, name, enrolled_from],
    )
    .map_err(|e| {
        // 충돌하는 것은 번호가 아니라 **자리**다. 교과는 번호가 겹치는 것이 정상이라
        // "3번이 겹칩니다"는 교사에게 어느 반을 보라는 말이 되지 않는다.
        constraint_err(
            &e,
            &format!(
                "이미 같은 자리에 재학생이 있습니다: {}",
                where_at(grade, class_no, number)
            ),
        )
    })?;
    Ok(conn.last_insert_rowid())
}

/// 학생 한 명을 내 명단에 등록한다. **학기 중 전입생이 오는 길이다.**
///
/// 전입생 한 명 때문에 명렬표 파일을 다시 만들게 하지 않는다 — 학기 중에 흔히 발생하고,
/// 파일을 다시 만들면 나머지 학생이 전부 차분을 거치므로 실수가 끼어들 여지가 커진다.
///
/// **역할을 구분하지 않는다.** 교과 강좌에도 수강생이 중간에 추가된다.
/// 학적 자리가 비어 있으면 새로 만들고, 이미 그 자리에 같은 이름이 있으면 명단에만
/// 연결한다 — 담임 반 학생이 내 강좌에 추가되는 경우다. 자리에 다른 이름이 있으면
/// **거절한다.** 그 자리를 비우는 것은 명렬표 차분이 담당하고, 한 명을 추가하는
/// 동작이 남의 학적을 마감해서는 안 된다.
pub fn add_student_impl(
    conn: &Connection,
    class_id: i64,
    grade: i64,
    class_no: i64,
    number: i64,
    name: &str,
    joined_on: &str,
) -> Result<i64, String> {
    let scope = class_scope(conn, class_id)?;
    let name = name.trim();
    if name.is_empty() {
        return Err("이름이 비어 있습니다.".to_string());
    }
    if grade < 1 || class_no < 1 || number < 1 {
        return Err(format!(
            "학년 · 반 · 번호는 1 이상이어야 합니다: {}",
            where_at(grade, class_no, number)
        ));
    }
    // 형식만 확인하고 자리를 채운 ISO로 맞춘다. **학년도 밖의 날짜도 그대로 받는다** —
    // 학년도는 기준 연도일 뿐 날짜 울타리가 아니다.
    let joined_on = &format_date(parse_date(joined_on)?);

    with_transaction(conn, || {
        let student_id = match seat_holder(conn, scope.school_id, grade, class_no, number)? {
            Some((id, held)) if held == name => id,
            Some((_, held)) => {
                return Err(format!(
                    "{} 자리에는 「{held}」이(가) 있습니다. \n                     번호를 확인하거나 명렬표 가져오기로 정리해주세요.",
                    where_at(grade, class_no, number)
                ))
            }
            None => insert_student(conn, scope.school_id, grade, class_no, number, name, joined_on)?,
        };
        join_class(conn, scope.id, student_id, joined_on)?;
        Ok(student_id)
    })
}

/// 번호와 이름을 고친다. **내 명단의 학생만, 담임 학급에서만 고친다.**
///
/// 번호와 이름은 **학적**이지 소속이 아니다. 특히 번호는 그 반의 순서라, 교과 강좌에서
/// 고치면 남의 반 격자 순서와 `member_by_number_on`(나이스가 번호를 열쇠로 쓰는 자리)이
/// 함께 어긋난다. 그래서 `class_scope`로 넓히지 않고 `homeroom_scope` 그대로 둔다.
pub fn update_student_impl(
    conn: &Connection,
    class_id: i64,
    id: i64,
    number: i64,
    name: &str,
) -> Result<(), String> {
    if number < 1 {
        return Err(format!("번호는 1 이상이어야 합니다: {number}"));
    }
    let name = name.trim();
    if name.is_empty() {
        return Err("이름이 비어 있습니다.".to_string());
    }
    let scope = homeroom_scope(conn, class_id).map_err(|e| {
        if e.starts_with("담임 학급이 아닙니다") {
            format!(
                "{e} — 번호와 이름은 학적입니다. 특히 번호는 그 반의 순서라 \
                 담임 학급에서만 고칩니다."
            )
        } else {
            e
        }
    })?;
    let n = conn
        .execute(
            "UPDATE student SET number = ?1, name = ?2
              WHERE id = ?3
                AND id IN (SELECT student_id FROM class_member
                            WHERE class_id = ?4 AND left_on IS NULL)",
            rusqlite::params![number, name, id, scope.id],
        )
        .map_err(|e| {
            constraint_err(&e, &format!("이미 같은 번호의 재학생이 있습니다: {number}번"))
        })?;
    if n == 0 {
        return Err(format!("학생을 찾을 수 없습니다: {id}"));
    }
    Ok(())
}

/// 내 명단에서 뺀다. 삭제가 아니고, **학교를 떠난 것도 아니다.**
///
/// 담임이 아는 것은 "이 학생이 더는 내 명단에 없다"까지다. 학적을 마감하면
/// 다른 명단에서도 함께 사라지므로, 여기서 닫는 것은 소속 기간뿐이다.
///
/// `class_member.left_on`만 건드리므로 **소속의 일이고, 소속은 역할을 가리지 않는다** —
/// 교과 강좌도 명단에서 학생을 뺀다.
pub fn withdraw_student_impl(
    conn: &Connection,
    class_id: i64,
    id: i64,
    date: &str,
) -> Result<(), String> {
    // 형식을 확인하고 자리를 채운 ISO로 맞춘다. 깨진 날짜가 `left_on`에 들어가면
    // 그날 명단을 세는 모든 화면이 그 학생을 잘못 구분한다.
    let date = &format_date(parse_date(date)?);
    let scope = class_scope(conn, class_id)?;
    let n = conn
        .execute(
            "UPDATE class_member SET left_on = ?1
              WHERE class_id = ?2 AND student_id = ?3 AND left_on IS NULL",
            rusqlite::params![date, scope.id, id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("학생을 찾을 수 없습니다: {id}"));
    }
    Ok(())
}

// ── 연락처 ────────────────────────────────────────────────────

/// 연락처는 `type` · `phone` · `memo` · 순서다.
///
/// 덧붙일 말은 `memo`에 적는다(`직장 번호 · 주간에는 받지 않음`). 관계와 번호 외에 무엇을 더 적을지는
/// 아직 정해지지 않았고, 자리를 먼저 만들면 무엇을 넣을지부터 되묻게 된다.
pub fn get_contacts_impl(conn: &Connection, student_id: i64) -> Result<Vec<ContactItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, type, phone, memo, sort_order FROM contact
             WHERE student_id = ?1 ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![student_id], |r| {
            Ok(ContactItem {
                id: r.get(0)?,
                kind: r.get(1)?,
                phone: r.get(2)?,
                memo: r.get(3)?,
                sort_order: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 한 학생의 연락처를 통째로 바꾼다.
///
/// 개별 추가·삭제 커맨드를 따로 두지 않는 이유는, 화면이 연락처 목록 전체를
/// 편집한 뒤 한 번에 저장하는 형태이기 때문이다. 행 단위 커맨드를 만들면
/// 화면과 저장 시점이 어긋나 순서가 뒤엉킨다.
pub fn set_contacts_impl(
    conn: &Connection,
    student_id: i64,
    contacts: &[ContactItem],
) -> Result<(), String> {
    for c in contacts {
        if c.kind.trim().is_empty() {
            return Err("연락처 이름(관계)이 비어 있습니다.".to_string());
        }
        if c.phone.trim().is_empty() {
            return Err(format!("{}의 번호가 비어 있습니다.", c.kind));
        }
    }
    with_transaction(conn, || {
        conn.execute(
            "DELETE FROM contact WHERE student_id = ?1",
            rusqlite::params![student_id],
        )
        .map_err(|e| e.to_string())?;
        for (i, c) in contacts.iter().enumerate() {
            conn.execute(
                "INSERT INTO contact (student_id, type, phone, memo, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    student_id,
                    c.kind.trim(),
                    c.phone.trim(),
                    c.memo.trim(),
                    i as i64
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
}

// ── 커맨드 ────────────────────────────────────────────────────

/// 명렬표가 말하는 학급을 돌려준다. 화면이 "우리 반"을 되묻지 않기 위한 것이다.
#[tauri::command]
pub fn detect_roster_class(entries: Vec<RosterEntry>) -> RosterClass {
    detect_class(&entries)
}

#[tauri::command]
pub fn get_students(db: State<DbState>, class_id: i64) -> Result<Vec<StudentItem>, String> {
    with_conn(&db, |c| get_students_impl(c, class_id))
}

#[tauri::command]
pub fn preview_roster(
    db: State<DbState>,
    class_id: i64,
    entries: Vec<RosterEntry>,
) -> Result<Vec<RosterDiffRow>, String> {
    with_conn(&db, |c| preview_roster_impl(c, class_id, &entries))
}

#[tauri::command]
pub fn apply_roster(
    db: State<DbState>,
    class_id: i64,
    effective_date: String,
    rows: Vec<RosterDiffRow>,
) -> Result<RosterApplyResult, String> {
    with_conn(&db, |c| {
        apply_roster_impl(c, class_id, &effective_date, &rows)
    })
}

#[tauri::command]
pub fn add_student(
    db: State<DbState>,
    class_id: i64,
    grade: i64,
    class_no: i64,
    number: i64,
    name: String,
    joined_on: String,
) -> Result<i64, String> {
    with_conn(&db, |conn| {
        add_student_impl(conn, class_id, grade, class_no, number, &name, &joined_on)
    })
}

#[tauri::command]
pub fn update_student(
    db: State<DbState>,
    class_id: i64,
    id: i64,
    number: i64,
    name: String,
) -> Result<(), String> {
    with_conn(&db, |c| update_student_impl(c, class_id, id, number, &name))
}

#[tauri::command]
pub fn withdraw_student(
    db: State<DbState>,
    class_id: i64,
    id: i64,
    date: String,
) -> Result<(), String> {
    with_conn(&db, |c| withdraw_student_impl(c, class_id, id, &date))
}

#[tauri::command]
pub fn get_contacts(db: State<DbState>, student_id: i64) -> Result<Vec<ContactItem>, String> {
    with_conn(&db, |c| get_contacts_impl(c, student_id))
}

#[tauri::command]
pub fn set_contacts(
    db: State<DbState>,
    student_id: i64,
    contacts: Vec<ContactItem>,
) -> Result<(), String> {
    with_conn(&db, |c| set_contacts_impl(c, student_id, &contacts))
}
