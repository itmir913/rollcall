//! 출결 코드와 체크 항목 설정.
//!
//! **수정은 마감 후 추가다.** 코드를 UPDATE로 고치면 그 코드를 참조하는 과거
//! `absence_span`의 의미가 소급 변경된다. "질병조퇴"를 "질병 조퇴"로 고치는 정도라면
//! 무해해 보이지만, 사유나 유형을 고치면 작년 기록의 뜻이 바뀐다. 그래서
//! `revise_code_impl`은 구 행에 `valid_to`를 찍고 새 행을 만든다.

use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::slots::{prompt_for_type, SlotPrompt};
use crate::state::DbState;
use crate::types::AttendanceCodeItem;
use rusqlite::Connection;
use tauri::State;

fn prompt_name(p: SlotPrompt) -> &'static str {
    match p {
        SlotPrompt::None => "none",
        SlotPrompt::Start => "start",
        SlotPrompt::End => "end",
        SlotPrompt::Both => "both",
    }
}

const CODE_COLS: &str = "id, reason, type, label, phrase_pattern, default_start, default_end, \
                         shortcut, sort_order, valid_from, valid_to";

fn map_code(row: &rusqlite::Row) -> rusqlite::Result<AttendanceCodeItem> {
    let code_type: String = row.get(2)?;
    let slot_prompt = prompt_name(prompt_for_type(&code_type)).to_string();
    Ok(AttendanceCodeItem {
        id: row.get(0)?,
        reason: row.get(1)?,
        code_type,
        label: row.get(3)?,
        phrase_pattern: row.get(4)?,
        default_start: row.get(5)?,
        default_end: row.get(6)?,
        shortcut: row.get(7)?,
        sort_order: row.get(8)?,
        valid_from: row.get(9)?,
        valid_to: row.get(10)?,
        slot_prompt,
    })
}

/// `on_date`가 있으면 그 날짜에 유효했던 코드만. 없으면 현재 유효한 것만.
///
/// 과거 날짜를 입력할 때 그날의 코드 목록을 쓰기 위한 것이다. 마감된 코드가
/// 목록에서 사라져도 그 코드를 쓴 과거 기록은 그대로 남아 조회된다.
pub fn get_codes_impl(
    conn: &Connection,
    on_date: Option<&str>,
) -> Result<Vec<AttendanceCodeItem>, String> {
    let (sql, date) = match on_date {
        Some(d) => (
            format!(
                "SELECT {CODE_COLS} FROM attendance_code
                 WHERE valid_from <= ?1 AND (valid_to IS NULL OR ?1 < valid_to)
                 ORDER BY sort_order, id"
            ),
            Some(d.to_string()),
        ),
        None => (
            format!(
                "SELECT {CODE_COLS} FROM attendance_code
                 WHERE valid_to IS NULL ORDER BY sort_order, id"
            ),
            None,
        ),
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = match date {
        Some(d) => stmt
            .query_map(rusqlite::params![d], map_code)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>(),
        None => stmt
            .query_map([], map_code)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>(),
    }
    .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 한 코드를 id로. 마감 여부와 무관하게 찾는다 — 과거 기록의 문구를 다시 그릴 때 쓴다.
pub fn get_code_impl(conn: &Connection, id: i64) -> Result<AttendanceCodeItem, String> {
    let sql = format!("SELECT {CODE_COLS} FROM attendance_code WHERE id = ?1");
    conn.query_row(&sql, rusqlite::params![id], map_code)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => "출결 코드를 찾을 수 없습니다.".to_string(),
            other => other.to_string(),
        })
}

#[allow(clippy::too_many_arguments)]
pub fn create_code_impl(
    conn: &Connection,
    reason: &str,
    code_type: &str,
    label: &str,
    phrase_pattern: Option<&str>,
    default_start: Option<&str>,
    default_end: Option<&str>,
    shortcut: Option<&str>,
    sort_order: i64,
    valid_from: &str,
) -> Result<i64, String> {
    if label.trim().is_empty() {
        return Err("코드 이름이 비어 있습니다.".to_string());
    }
    if reason.trim().is_empty() {
        return Err("사유 구분이 비어 있습니다.".to_string());
    }
    crate::slots::validate_span(default_start, default_end)?;

    conn.execute(
        "INSERT INTO attendance_code
           (reason, type, label, phrase_pattern, default_start, default_end,
            shortcut, sort_order, valid_from)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            reason,
            code_type,
            label,
            phrase_pattern,
            default_start,
            default_end,
            shortcut,
            sort_order,
            valid_from
        ],
    )
    .map_err(|e| crate::state::constraint_err(&e, "이미 있는 코드입니다."))?;
    Ok(conn.last_insert_rowid())
}

/// 코드 마감. 목록에서 사라지지만 과거 기록은 그대로 남는다.
pub fn retire_code_impl(conn: &Connection, id: i64, valid_to: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE attendance_code SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
        rusqlite::params![valid_to, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 코드 "수정" = 구 행 마감 + 새 행 추가. UPDATE가 아니다.
///
/// 새 행의 `valid_from`은 구 행의 `valid_to`와 같다. 두 행의 유효 구간이 겹치지
/// 않도록 `get_codes_impl`은 `valid_from <= d < valid_to`로 조회한다.
#[allow(clippy::too_many_arguments)]
pub fn revise_code_impl(
    conn: &Connection,
    id: i64,
    reason: &str,
    code_type: &str,
    label: &str,
    phrase_pattern: Option<&str>,
    default_start: Option<&str>,
    default_end: Option<&str>,
    shortcut: Option<&str>,
    sort_order: i64,
    effective_date: &str,
) -> Result<i64, String> {
    with_transaction(conn, || {
        retire_code_impl(conn, id, effective_date)?;
        create_code_impl(
            conn,
            reason,
            code_type,
            label,
            phrase_pattern,
            default_start,
            default_end,
            shortcut,
            sort_order,
            effective_date,
        )
    })
}

/// 증상 자동완성 후보.
///
/// 과거 `absence_span.symptom`에서 뽑는다. **별도 테이블을 두지 않는다** —
/// 쓸수록 후보가 쌓이고, 관리할 목록이 하나 줄어든다.
pub fn get_symptom_suggestions_impl(
    conn: &Connection,
    prefix: &str,
    limit: i64,
) -> Result<Vec<String>, String> {
    let pattern = format!("{}%", prefix.replace('%', "\\%").replace('_', "\\_"));
    let mut stmt = conn
        .prepare(
            "SELECT symptom, COUNT(*) AS n
             FROM absence_span
             WHERE symptom IS NOT NULL AND symptom <> '' AND symptom LIKE ?1 ESCAPE '\\'
             GROUP BY symptom
             ORDER BY n DESC, symptom
             LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![pattern, limit], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_codes(
    db: State<DbState>,
    on_date: Option<String>,
) -> Result<Vec<AttendanceCodeItem>, String> {
    with_conn(&db, |c| get_codes_impl(c, on_date.as_deref()))
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn create_code(
    db: State<DbState>,
    reason: String,
    code_type: String,
    label: String,
    phrase_pattern: Option<String>,
    default_start: Option<String>,
    default_end: Option<String>,
    shortcut: Option<String>,
    sort_order: i64,
    valid_from: String,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        create_code_impl(
            c,
            &reason,
            &code_type,
            &label,
            phrase_pattern.as_deref(),
            default_start.as_deref(),
            default_end.as_deref(),
            shortcut.as_deref(),
            sort_order,
            &valid_from,
        )
    })
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn revise_code(
    db: State<DbState>,
    id: i64,
    reason: String,
    code_type: String,
    label: String,
    phrase_pattern: Option<String>,
    default_start: Option<String>,
    default_end: Option<String>,
    shortcut: Option<String>,
    sort_order: i64,
    effective_date: String,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        revise_code_impl(
            c,
            id,
            &reason,
            &code_type,
            &label,
            phrase_pattern.as_deref(),
            default_start.as_deref(),
            default_end.as_deref(),
            shortcut.as_deref(),
            sort_order,
            &effective_date,
        )
    })
}

#[tauri::command]
pub fn retire_code(db: State<DbState>, id: i64, valid_to: String) -> Result<(), String> {
    with_conn(&db, |c| retire_code_impl(c, id, &valid_to))
}

#[tauri::command]
pub fn get_symptom_suggestions(
    db: State<DbState>,
    prefix: String,
    limit: Option<i64>,
) -> Result<Vec<String>, String> {
    with_conn(&db, |c| {
        get_symptom_suggestions_impl(c, &prefix, limit.unwrap_or(8))
    })
}
