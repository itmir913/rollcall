//! 출결의 두 축(구분·종류)과 그 쌍인 코드.
//!
//! 축이 둘로 나뉘어 있는 이유는 입력이 그렇게 이뤄지기 때문이다. 교사는 구분을
//! 누르고 종류를 누른다. 그 사이의 순간 — 구분만 정해진 상태 — 도 저장 가능해야
//! 하고, 학생이 안 왔는데 연락이 닿지 않으면 둘 다 빈 채로 남는다.
//! 한 테이블의 `code_id` 하나로는 이 중간 상태를 표현할 수 없다.
//!
//! **수정은 마감 후 추가다.** UPDATE로 고치면 그 축을 참조하는 과거 기록의 의미가
//! 소급 변경된다. `valid_to`를 찍고 새 행을 만든다.

use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::slots::is_slot_prompt;
use crate::state::{constraint_err, DbState};
use crate::types::{AttendanceCodeItem, ReasonItem, TypeItem};
use rusqlite::Connection;
use tauri::State;

/// `valid_from <= d < valid_to` — 경계일에는 새 행만 유효하다.
/// `on_date`가 없으면 현재 유효한 것만.
fn validity_clause(alias: &str, on_date: Option<&str>) -> String {
    match on_date {
        Some(_) => format!("{alias}.valid_from <= ?1 AND ({alias}.valid_to IS NULL OR ?1 < {alias}.valid_to)"),
        None => format!("{alias}.valid_to IS NULL"),
    }
}

fn query_axis<T, F>(
    conn: &Connection,
    sql: &str,
    on_date: Option<&str>,
    map: F,
) -> Result<Vec<T>, String>
where
    F: Fn(&rusqlite::Row) -> rusqlite::Result<T>,
{
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = match on_date {
        Some(d) => stmt
            .query_map(rusqlite::params![d], |r| map(r))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>(),
        None => stmt
            .query_map([], |r| map(r))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>(),
    };
    rows.map_err(|e| e.to_string())
}

// ── 축 1: 구분 ────────────────────────────────────────────────

pub fn get_reasons_impl(
    conn: &Connection,
    on_date: Option<&str>,
) -> Result<Vec<ReasonItem>, String> {
    let sql = format!(
        "SELECT id, label, shortcut, sort_order, valid_from, valid_to
         FROM attendance_reason r WHERE {} ORDER BY sort_order, id",
        validity_clause("r", on_date)
    );
    query_axis(conn, &sql, on_date, |r| {
        Ok(ReasonItem {
            id: r.get(0)?,
            label: r.get(1)?,
            shortcut: r.get(2)?,
            sort_order: r.get(3)?,
            valid_from: r.get(4)?,
            valid_to: r.get(5)?,
        })
    })
}

pub fn create_reason_impl(
    conn: &Connection,
    label: &str,
    shortcut: Option<&str>,
    sort_order: i64,
    valid_from: &str,
) -> Result<i64, String> {
    if label.trim().is_empty() {
        return Err("구분 이름이 비어 있습니다.".to_string());
    }
    conn.execute(
        "INSERT INTO attendance_reason (label, shortcut, sort_order, valid_from)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![label, shortcut, sort_order, valid_from],
    )
    .map_err(|e| constraint_err(&e, "이미 있는 구분입니다."))?;
    Ok(conn.last_insert_rowid())
}

pub fn retire_reason_impl(conn: &Connection, id: i64, valid_to: &str) -> Result<(), String> {
    with_transaction(conn, || {
        conn.execute(
            "UPDATE attendance_reason SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
            rusqlite::params![valid_to, id],
        )
        .map_err(|e| e.to_string())?;
        // 축이 마감되면 그 축을 쓰는 쌍도 함께 마감된다. 남겨두면 목록에서
        // 사라진 구분을 가진 코드가 계속 유효한 것으로 조회된다.
        conn.execute(
            "UPDATE attendance_code SET valid_to = ?1 WHERE reason_id = ?2 AND valid_to IS NULL",
            rusqlite::params![valid_to, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

// ── 축 2: 종류 ────────────────────────────────────────────────

pub fn get_types_impl(conn: &Connection, on_date: Option<&str>) -> Result<Vec<TypeItem>, String> {
    let sql = format!(
        "SELECT id, label, slot_prompt, shortcut, sort_order, valid_from, valid_to
         FROM attendance_type t WHERE {} ORDER BY sort_order, id",
        validity_clause("t", on_date)
    );
    query_axis(conn, &sql, on_date, |r| {
        Ok(TypeItem {
            id: r.get(0)?,
            label: r.get(1)?,
            slot_prompt: r.get(2)?,
            shortcut: r.get(3)?,
            sort_order: r.get(4)?,
            valid_from: r.get(5)?,
            valid_to: r.get(6)?,
        })
    })
}

pub fn create_type_impl(
    conn: &Connection,
    label: &str,
    slot_prompt: &str,
    shortcut: Option<&str>,
    sort_order: i64,
    valid_from: &str,
) -> Result<i64, String> {
    if label.trim().is_empty() {
        return Err("종류 이름이 비어 있습니다.".to_string());
    }
    if !is_slot_prompt(slot_prompt) {
        return Err(format!(
            "교시를 묻는 방식이 올바르지 않습니다: {slot_prompt} (none / start / end / both)"
        ));
    }
    conn.execute(
        "INSERT INTO attendance_type (label, slot_prompt, shortcut, sort_order, valid_from)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![label, slot_prompt, shortcut, sort_order, valid_from],
    )
    .map_err(|e| constraint_err(&e, "이미 있는 종류입니다."))?;
    Ok(conn.last_insert_rowid())
}

pub fn retire_type_impl(conn: &Connection, id: i64, valid_to: &str) -> Result<(), String> {
    with_transaction(conn, || {
        conn.execute(
            "UPDATE attendance_type SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
            rusqlite::params![valid_to, id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE attendance_code SET valid_to = ?1 WHERE type_id = ?2 AND valid_to IS NULL",
            rusqlite::params![valid_to, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
}

// ── 쌍: 코드 ──────────────────────────────────────────────────

const CODE_SELECT: &str = "SELECT c.id, c.reason_id, c.type_id, r.label, t.label, c.label,
                                  c.phrase_pattern, c.sort_order, c.valid_from, c.valid_to
                           FROM attendance_code c
                           JOIN attendance_reason r ON r.id = c.reason_id
                           JOIN attendance_type t ON t.id = c.type_id";

fn map_code(row: &rusqlite::Row) -> rusqlite::Result<AttendanceCodeItem> {
    Ok(AttendanceCodeItem {
        id: row.get(0)?,
        reason_id: row.get(1)?,
        type_id: row.get(2)?,
        reason_label: row.get(3)?,
        type_label: row.get(4)?,
        label: row.get(5)?,
        phrase_pattern: row.get(6)?,
        sort_order: row.get(7)?,
        valid_from: row.get(8)?,
        valid_to: row.get(9)?,
    })
}

pub fn get_codes_impl(
    conn: &Connection,
    on_date: Option<&str>,
) -> Result<Vec<AttendanceCodeItem>, String> {
    let sql = format!(
        "{CODE_SELECT} WHERE {} ORDER BY c.sort_order, c.id",
        validity_clause("c", on_date)
    );
    query_axis(conn, &sql, on_date, map_code)
}

/// 두 축으로 쌍을 찾는다. 한쪽이라도 비면 코드가 없다 — 그것이 정상이다.
pub fn find_code_impl(
    conn: &Connection,
    reason_id: Option<i64>,
    type_id: Option<i64>,
    on_date: Option<&str>,
) -> Result<Option<AttendanceCodeItem>, String> {
    let (Some(reason_id), Some(type_id)) = (reason_id, type_id) else {
        return Ok(None);
    };
    let (clause, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match on_date {
        Some(d) => (
            "c.valid_from <= ?3 AND (c.valid_to IS NULL OR ?3 < c.valid_to)".to_string(),
            vec![Box::new(reason_id), Box::new(type_id), Box::new(d.to_string())],
        ),
        None => (
            "c.valid_to IS NULL".to_string(),
            vec![Box::new(reason_id), Box::new(type_id)],
        ),
    };
    let sql = format!("{CODE_SELECT} WHERE c.reason_id = ?1 AND c.type_id = ?2 AND {clause}");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
    stmt.query_row(refs.as_slice(), map_code)
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other.to_string()),
        })
}

pub fn create_code_impl(
    conn: &Connection,
    reason_id: i64,
    type_id: i64,
    label: &str,
    phrase_pattern: Option<&str>,
    sort_order: i64,
    valid_from: &str,
) -> Result<i64, String> {
    if label.trim().is_empty() {
        return Err("코드 이름이 비어 있습니다.".to_string());
    }
    conn.execute(
        "INSERT INTO attendance_code
           (reason_id, type_id, label, phrase_pattern, sort_order, valid_from)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![reason_id, type_id, label, phrase_pattern, sort_order, valid_from],
    )
    .map_err(|e| constraint_err(&e, "이미 그 구분과 종류의 조합이 있습니다."))?;
    Ok(conn.last_insert_rowid())
}

pub fn retire_code_impl(conn: &Connection, id: i64, valid_to: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE attendance_code SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
        rusqlite::params![valid_to, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 코드 "수정" = 구 행 마감 + 새 행 추가. UPDATE가 아니다.
pub fn revise_code_impl(
    conn: &Connection,
    id: i64,
    label: &str,
    phrase_pattern: Option<&str>,
    sort_order: i64,
    effective_date: &str,
) -> Result<i64, String> {
    with_transaction(conn, || {
        let (reason_id, type_id): (i64, i64) = conn
            .query_row(
                "SELECT reason_id, type_id FROM attendance_code WHERE id = ?1",
                rusqlite::params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => "코드를 찾을 수 없습니다.".to_string(),
                other => other.to_string(),
            })?;
        retire_code_impl(conn, id, effective_date)?;
        create_code_impl(
            conn,
            reason_id,
            type_id,
            label,
            phrase_pattern,
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
pub fn get_reasons(db: State<DbState>, on_date: Option<String>) -> Result<Vec<ReasonItem>, String> {
    with_conn(&db, |c| get_reasons_impl(c, on_date.as_deref()))
}

#[tauri::command]
pub fn create_reason(
    db: State<DbState>,
    label: String,
    shortcut: Option<String>,
    sort_order: i64,
    valid_from: String,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        create_reason_impl(c, &label, shortcut.as_deref(), sort_order, &valid_from)
    })
}

#[tauri::command]
pub fn retire_reason(db: State<DbState>, id: i64, valid_to: String) -> Result<(), String> {
    with_conn(&db, |c| retire_reason_impl(c, id, &valid_to))
}

#[tauri::command]
pub fn get_types(db: State<DbState>, on_date: Option<String>) -> Result<Vec<TypeItem>, String> {
    with_conn(&db, |c| get_types_impl(c, on_date.as_deref()))
}

#[tauri::command]
pub fn create_type(
    db: State<DbState>,
    label: String,
    slot_prompt: String,
    shortcut: Option<String>,
    sort_order: i64,
    valid_from: String,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        create_type_impl(
            c,
            &label,
            &slot_prompt,
            shortcut.as_deref(),
            sort_order,
            &valid_from,
        )
    })
}

#[tauri::command]
pub fn retire_type(db: State<DbState>, id: i64, valid_to: String) -> Result<(), String> {
    with_conn(&db, |c| retire_type_impl(c, id, &valid_to))
}

#[tauri::command]
pub fn get_codes(
    db: State<DbState>,
    on_date: Option<String>,
) -> Result<Vec<AttendanceCodeItem>, String> {
    with_conn(&db, |c| get_codes_impl(c, on_date.as_deref()))
}

#[tauri::command]
pub fn create_code(
    db: State<DbState>,
    reason_id: i64,
    type_id: i64,
    label: String,
    phrase_pattern: Option<String>,
    sort_order: i64,
    valid_from: String,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        create_code_impl(
            c,
            reason_id,
            type_id,
            &label,
            phrase_pattern.as_deref(),
            sort_order,
            &valid_from,
        )
    })
}

#[tauri::command]
pub fn revise_code(
    db: State<DbState>,
    id: i64,
    label: String,
    phrase_pattern: Option<String>,
    sort_order: i64,
    effective_date: String,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        revise_code_impl(
            c,
            id,
            &label,
            phrase_pattern.as_deref(),
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
