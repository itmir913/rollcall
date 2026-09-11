//! 교과 차시 — 내 수업에 있었는가 하나만 기록한다.
//!
//! **담임 기록과 표부터 다르다.** 담임은 `absence_span`(구분 · 종류 · 기간 · 서류 ·
//! 나이스)이고, 교과는 `subject_session` · `subject_absence`다. 한 표에 섞으면 교과의
//! 4교시 결석이 나이스 미등재 목록에 올라오고 체험학습 한도에도 섞여 들어간다.
//!
//! **행이 있다는 것이 곧 그 교시를 불렀다는 뜻이다.** 결석자 행만으로는 "빠진 사람이
//! 없는 날"과 "아직 안 부른 날"이 구별되지 않는다. 그래서 수업을 시작할 때 차시 한 행을
//! 먼저 만들고, 그 칸에서 빠진 학생만 적는다.
//!
//! **차시 번호(N차시)를 저장하지 않는다.** 날짜 · 교시 순으로 세면 나온다. 저장하면
//! 차시 하나를 지웠을 때 전부 어긋나고, 다시 매기는 코드가 또 필요하다.
//!
//! **주간 시간표 테이블을 만들지 않는다.** 담임 쪽에서 슬롯을 전개하지 않는 것과 같은
//! 이유다 — 단축수업 · 시험일에 전부 틀린다. 교사가 그 자리에서 교시를 더한다.
//!
//! 트랜잭션 규칙: 여러 문장을 쓰는 경로는 `db::with_transaction`으로만 연다.
//! 조기 반환은 클로저 **안에서** `?`로 한다(db.rs 참고).

use crate::commands::attendance::{max_slot_of, span_text};
use crate::commands::class::{member_count_on, member_seats_on, subject_scope, ClassScope};
use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::{format_date, parse_date};
use crate::slots::{CLOSING, HOMEROOM};
use crate::state::{constraint_err, DbState};
use crate::types::{SubjectRollItem, SubjectSessionItem};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use tauri::State;

// ── 교시 ──────────────────────────────────────────────────────

/// 교과 수업이 놓일 수 있는 교시인가. 스키마의 `CHECK (slot GLOB '[1-9]')`가 같은 것을
/// 막지만, 거기서 걸리면 교사에게는 "입력값이 허용 범위를 벗어났습니다"만 남는다.
///
/// **조회 · 종례는 들어올 수 없다.** 그 둘은 담임이 하루의 양 끝에서 보는 것이지
/// 누가 가르치는 시간이 아니다. 최대 교시는 **학교가 들고 있는 값**이라 앱 상수로
/// 비교하지 않는다.
fn check_slot(conn: &Connection, school_id: i64, slot: &str) -> Result<String, String> {
    let slot = slot.trim();
    if slot == HOMEROOM || slot == CLOSING {
        return Err(format!(
            "{slot}는 교과 수업이 아닙니다. 담임이 하루의 양 끝에서 보는 시간입니다."
        ));
    }
    let max_slot = max_slot_of(conn, school_id)?;
    match slot.parse::<i64>() {
        Ok(n) if (1..=max_slot).contains(&n) => Ok(n.to_string()),
        _ => Err(format!(
            "1교시부터 {max_slot}교시까지만 만들 수 있습니다: {slot}"
        )),
    }
}

// ── 차시 ──────────────────────────────────────────────────────

/// 차시 한 행이 가리키는 강좌와 날짜.
struct SessionAt {
    date: String,
    scope: ClassScope,
}

fn session_at(conn: &Connection, session_id: i64) -> Result<SessionAt, String> {
    let (class_id, date) = conn
        .query_row(
            "SELECT class_id, date FROM subject_session WHERE id = ?1",
            params![session_id],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("차시를 찾을 수 없습니다: {session_id}"),
            other => other.to_string(),
        })?;
    Ok(SessionAt {
        date,
        scope: subject_scope(conn, class_id)?,
    })
}

/// 기간 안의 차시 목록. **날짜 · 교시 순이다** — 차시 번호를 저장하지 않으므로
/// 이 순서가 곧 `N차시`다.
///
/// `total`은 그 **날짜의** 명단 인원이다. 지금 인원이 아니다 — 명렬표를 다시 가져와
/// 인원이 달라져도 지난 차시의 분모가 조용히 움직이면 안 된다.
pub fn get_subject_sessions_impl(
    conn: &Connection,
    class_id: i64,
    from: &str,
    to: &str,
) -> Result<Vec<SubjectSessionItem>, String> {
    // 자리를 채운 ISO로 맞춘다. 날짜 비교가 문자열 비교라 `2026-9-1`이 오면
    // 그 달의 차시가 통째로 빠진다.
    let from = format_date(parse_date(from)?);
    let to = format_date(parse_date(to)?);
    if to < from {
        return Err("끝 날짜가 시작 날짜보다 앞입니다.".to_string());
    }
    let scope = subject_scope(conn, class_id)?;

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.date, s.slot, s.memo,
                    (SELECT COUNT(*) FROM subject_absence a WHERE a.session_id = s.id)
               FROM subject_session s
              WHERE s.class_id = ?1 AND s.date >= ?2 AND s.date <= ?3
              ORDER BY s.date, s.slot, s.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![scope.id, from, to], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 같은 날짜의 차시가 여럿이므로 분모는 날짜마다 한 번만 센다.
    let mut totals: HashMap<String, i64> = HashMap::new();
    let mut out = Vec::with_capacity(rows.len());
    for (id, date, slot, memo, absent_count) in rows {
        let total = match totals.get(&date) {
            Some(n) => *n,
            None => {
                let n = member_count_on(conn, scope.id, &date)?;
                totals.insert(date.clone(), n);
                n
            }
        };
        out.push(SubjectSessionItem {
            id,
            date,
            slot,
            memo,
            absent_count,
            total,
        });
    }
    Ok(out)
}

/// 차시 한 칸을 만든다. **같은 칸이 이미 있으면 그 id를 돌려준다.**
///
/// 교사가 [교시 추가]를 두 번 누른 것은 실수지 새 칸을 만들라는 뜻이 아니다. 여기서
/// UNIQUE 위반으로 실패시키면 "이미 있습니다"를 보고 목록에서 그 칸을 다시 찾아야 한다.
///
/// **연강(3 · 4교시)도 두 칸이다.** 중간에 나간 학생이 실제로 있다. 누르는 수고는
/// 화면에서 교시를 여럿 고르게 해 줄인다 — 화면 문제지 구조 문제가 아니다.
pub fn create_subject_session_impl(
    conn: &Connection,
    class_id: i64,
    date: &str,
    slot: &str,
) -> Result<i64, String> {
    // **저장은 언제나 자리를 채운 ISO다.** `2026-9-10`을 그대로 넣으면 날짜로 거르는
    // 모든 화면에서 그 행이 사라진다.
    let date = format_date(parse_date(date)?);
    let scope = subject_scope(conn, class_id)?;
    let slot = check_slot(conn, scope.school_id, slot)?;

    with_transaction(conn, || {
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM subject_session
                  WHERE class_id = ?1 AND date = ?2 AND slot = ?3",
                params![scope.id, date, slot],
                |r| r.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.to_string()),
            })?;
        if let Some(id) = existing {
            return Ok(id);
        }

        conn.execute(
            "INSERT INTO subject_session (class_id, date, slot) VALUES (?1, ?2, ?3)",
            params![scope.id, date, slot],
        )
        .map_err(|e| constraint_err(&e, "이미 그 교시가 있습니다."))?;
        Ok(conn.last_insert_rowid())
    })
}

/// 차시를 지운다. **그 칸의 결석 기록도 함께 사라진다**
/// (`subject_absence.session_id`가 `ON DELETE CASCADE`다).
///
/// 담임 쪽 구간과 달리 마감하지 않는다 — 차시는 "그 교시를 불렀다"는 사실 자체라,
/// 잘못 만든 칸을 남겨 두면 부르지 않은 교시가 부른 것으로 남는다.
pub fn delete_subject_session_impl(conn: &Connection, session_id: i64) -> Result<(), String> {
    let removed = conn
        .execute(
            "DELETE FROM subject_session WHERE id = ?1",
            params![session_id],
        )
        .map_err(|e| e.to_string())?;
    if removed == 0 {
        return Err(format!("차시를 찾을 수 없습니다: {session_id}"));
    }
    Ok(())
}

/// 차시 메모. 자유 문장이고 앱은 해석하지 않는다(`수행평가` · `보강`).
pub fn set_session_memo_impl(
    conn: &Connection,
    session_id: i64,
    memo: &str,
) -> Result<(), String> {
    let changed = conn
        .execute(
            "UPDATE subject_session SET memo = ?1 WHERE id = ?2",
            params![memo, session_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("차시를 찾을 수 없습니다: {session_id}"));
    }
    Ok(())
}

// ── 담임 출결 참고 ────────────────────────────────────────────

/// 담임으로 적어 둔 출결 한 건을 사람이 읽는 한 조각으로.
///
/// 두 축이 다 채워졌으면 코드 라벨(`질병조퇴`), 아니면 채워진 쪽만 적는다 —
/// **미완성 기록이 정상 상태다.** 기간 문구는 담임 화면과 같은 생성기를 쓴다.
fn note_piece(
    code_label: Option<String>,
    reason_label: Option<String>,
    type_label: Option<String>,
    slot_prompt: Option<&str>,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
) -> String {
    let axis = match code_label {
        Some(label) => label,
        None => format!(
            "{} {}",
            reason_label.unwrap_or_else(|| "구분 미정".to_string()),
            type_label.unwrap_or_else(|| "종류 미정".to_string())
        ),
    };
    format!("{axis} · {}", span_text(slot_prompt, start_slot, end_slot))
}

/// 그 날짜에 **내가 담임으로 적어 둔** 출결. 학생 id → 한 문장.
///
/// **의도한 참고이지 유출이 아니다.** 교과 수업에서 빈 자리를 보았는데 아침에 담임으로
/// 질병결석을 찍어 두었으면 그것을 보여주는 것이 기능이다. 다만 두 조건이 붙는다 —
/// 읽기 전용이고(여기서 아무것도 쓰지 않는다), 교과 화면에서 적은 것과 눈에 띄게
/// 구별되어야 한다(그래서 별도 칸으로 돌려준다).
///
/// **내 담임 학급의 기록만 본다.** 같은 학교의 `role = 'homeroom'`인 맡은 것에 속한
/// 구간만 읽는다 — 교사 한 명이 로컬에 설치해 쓰므로 그것이 곧 내 기록이다.
fn homeroom_notes(
    conn: &Connection,
    school_id: i64,
    date: &str,
) -> Result<HashMap<i64, String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT s.student_id, c.label, r.label, t.label, t.slot_prompt,
                    s.start_slot, s.end_slot
               FROM absence_span s
                        JOIN teaching_class tc ON tc.id = s.class_id
                        LEFT JOIN attendance_reason r ON r.id = s.reason_id
                        LEFT JOIN attendance_type t ON t.id = s.type_id
                        LEFT JOIN attendance_code c
                               ON c.reason_id = s.reason_id
                              AND c.type_id = s.type_id
                              AND c.valid_from <= s.date
                              AND (c.valid_to IS NULL OR s.date < c.valid_to)
              WHERE s.date = ?1
                AND tc.school_id = ?2
                AND tc.role = 'homeroom'
              ORDER BY s.student_id, s.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![date, school_id], |r| {
            let start: Option<String> = r.get(5)?;
            let end: Option<String> = r.get(6)?;
            let prompt: Option<String> = r.get(4)?;
            Ok((
                r.get::<_, i64>(0)?,
                note_piece(
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    prompt.as_deref(),
                    start.as_deref(),
                    end.as_deref(),
                ),
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 하루 두 구간이 정상이므로 한 학생에게 여러 조각이 온다.
    let mut out: HashMap<i64, String> = HashMap::new();
    for (student_id, piece) in rows {
        out.entry(student_id)
            .and_modify(|acc| {
                acc.push_str(", ");
                acc.push_str(&piece);
            })
            .or_insert(piece);
    }
    Ok(out)
}

// ── 차시 명단 ─────────────────────────────────────────────────

/// 그 차시의 명단. **그 날짜에 명단이던 학생 전원이 줄로 나온다.**
///
/// 빠진 학생만 돌려주면 "빠진 사람이 없는 날"을 그릴 수 없다. 출석은 저장하지 않으므로
/// `absent = false`인 줄이 곧 출석이다.
pub fn get_session_roll_impl(
    conn: &Connection,
    session_id: i64,
) -> Result<Vec<SubjectRollItem>, String> {
    let at = session_at(conn, session_id)?;
    let members = member_seats_on(conn, at.scope.id, &at.date)?;

    let mut stmt = conn
        .prepare("SELECT student_id, memo FROM subject_absence WHERE session_id = ?1")
        .map_err(|e| e.to_string())?;
    let absent: HashMap<i64, String> = stmt
        .query_map(params![session_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();

    let notes = homeroom_notes(conn, at.scope.school_id, &at.date)?;

    Ok(members
        .into_iter()
        .map(|(id, grade, class_no, number, name)| SubjectRollItem {
            student_id: id,
            grade,
            class_no,
            number,
            name,
            absent: absent.contains_key(&id),
            memo: absent.get(&id).cloned().unwrap_or_default(),
            homeroom_note: notes.get(&id).cloned(),
        })
        .collect())
}

/// 그 학생이 이 강좌 명단인지 확인한다. **날짜로 좁히지 않는다** — 지난 차시를 열어
/// 정리하는 동안 지금 명단에서 빠진 학생의 그날 기록을 고칠 수 있어야 한다.
///
/// 확인하는 이유는 판정이 아니라 메시지다. 그냥 넣으면 외래키 위반이 "참조하는 항목이
/// 없습니다"로 올라와, 교사는 무엇이 잘못됐는지 알 수 없다.
fn ensure_member(conn: &Connection, class_id: i64, student_id: i64) -> Result<(), String> {
    let found: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM class_member WHERE class_id = ?1 AND student_id = ?2",
            params![class_id, student_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if found == 0 {
        return Err(format!("이 강좌 명단에 없는 학생입니다: {student_id}"));
    }
    Ok(())
}

/// 찍으면 결석, 같은 학생을 다시 찍으면 취소. **돌려주는 값이 찍은 뒤의 결석 여부다.**
///
/// 담임 쪽 "같은 조합을 다시 찍으면 취소"와 같은 규칙이다. 실수로 두 번 누른 것과
/// "방금 찍은 것을 무르고 싶다"가 교사에게는 같은 동작이라, 확인 대화상자도 띄우지 않는다.
pub fn toggle_subject_absence_impl(
    conn: &Connection,
    session_id: i64,
    student_id: i64,
) -> Result<bool, String> {
    let at = session_at(conn, session_id)?;
    ensure_member(conn, at.scope.id, student_id)?;

    with_transaction(conn, || {
        let removed = conn
            .execute(
                "DELETE FROM subject_absence WHERE session_id = ?1 AND student_id = ?2",
                params![session_id, student_id],
            )
            .map_err(|e| e.to_string())?;
        if removed > 0 {
            return Ok(false);
        }
        conn.execute(
            "INSERT INTO subject_absence (session_id, student_id) VALUES (?1, ?2)",
            params![session_id, student_id],
        )
        .map_err(|e| constraint_err(&e, "이미 그 차시에 적힌 학생입니다."))?;
        Ok(true)
    })
}

/// 결석 한 줄의 메모. 자유 문장이고 앱은 해석하지 않는다.
///
/// **없는 결석에 메모만 달지 않는다.** 메모는 결석에 붙는 말이라, 빠지지 않은 학생에게
/// 메모만 남으면 그 줄이 결석인지 아닌지 화면이 말할 수 없다.
pub fn set_subject_absence_memo_impl(
    conn: &Connection,
    session_id: i64,
    student_id: i64,
    memo: &str,
) -> Result<(), String> {
    let changed = conn
        .execute(
            "UPDATE subject_absence SET memo = ?1 WHERE session_id = ?2 AND student_id = ?3",
            params![memo, session_id, student_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!(
            "그 차시에 빠진 것으로 적힌 학생이 아닙니다: {student_id}"
        ));
    }
    Ok(())
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_subject_sessions(
    db: State<DbState>,
    class_id: i64,
    from: String,
    to: String,
) -> Result<Vec<SubjectSessionItem>, String> {
    with_conn(&db, |c| get_subject_sessions_impl(c, class_id, &from, &to))
}

#[tauri::command]
pub fn create_subject_session(
    db: State<DbState>,
    class_id: i64,
    date: String,
    slot: String,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        create_subject_session_impl(c, class_id, &date, &slot)
    })
}

#[tauri::command]
pub fn delete_subject_session(db: State<DbState>, session_id: i64) -> Result<(), String> {
    with_conn(&db, |c| delete_subject_session_impl(c, session_id))
}

#[tauri::command]
pub fn set_session_memo(
    db: State<DbState>,
    session_id: i64,
    memo: String,
) -> Result<(), String> {
    with_conn(&db, |c| set_session_memo_impl(c, session_id, &memo))
}

#[tauri::command]
pub fn get_session_roll(
    db: State<DbState>,
    session_id: i64,
) -> Result<Vec<SubjectRollItem>, String> {
    with_conn(&db, |c| get_session_roll_impl(c, session_id))
}

#[tauri::command]
pub fn toggle_subject_absence(
    db: State<DbState>,
    session_id: i64,
    student_id: i64,
) -> Result<bool, String> {
    with_conn(&db, |c| {
        toggle_subject_absence_impl(c, session_id, student_id)
    })
}

#[tauri::command]
pub fn set_subject_absence_memo(
    db: State<DbState>,
    session_id: i64,
    student_id: i64,
    memo: String,
) -> Result<(), String> {
    with_conn(&db, |c| {
        set_subject_absence_memo_impl(c, session_id, student_id, &memo)
    })
}
