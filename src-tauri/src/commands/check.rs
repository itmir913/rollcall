//! 체크 항목과 하루 단위 체크.
//!
//! 항목은 행이지 컬럼이 아니다. 설정에서 행을 추가하면 격자에 열이 하나 붙는다.
//! 리로스쿨·하이클래스처럼 학교마다 다른 항목을 시드에 넣지 않는 이유이기도 하다.

use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::{days_overdue, due_date, format_date, parse_date};
use crate::state::DbState;
use crate::types::{CheckItemDef, PendingRow, PendingSummary};
use chrono::Local;
use rusqlite::Connection;
use tauri::State;

// ── 항목 정의 ─────────────────────────────────────────────────

fn map_item(row: &rusqlite::Row) -> rusqlite::Result<CheckItemDef> {
    Ok(CheckItemDef {
        id: row.get(0)?,
        name: row.get(1)?,
        due_days: row.get(2)?,
        include_weekend: row.get::<_, i64>(3)? != 0,
        default_done: row.get::<_, i64>(4)? != 0,
        sort_order: row.get(5)?,
        active: row.get::<_, i64>(6)? != 0,
    })
}

pub fn get_check_items_impl(
    conn: &Connection,
    active_only: bool,
) -> Result<Vec<CheckItemDef>, String> {
    let sql = if active_only {
        "SELECT id, name, due_days, include_weekend, default_done, sort_order, active
         FROM check_item WHERE active = 1 ORDER BY sort_order, id"
    } else {
        "SELECT id, name, due_days, include_weekend, default_done, sort_order, active
         FROM check_item ORDER BY sort_order, id"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_item)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

pub fn create_check_item_impl(
    conn: &Connection,
    name: &str,
    due_days: Option<i64>,
    include_weekend: bool,
    default_done: bool,
    sort_order: i64,
) -> Result<i64, String> {
    if name.trim().is_empty() {
        return Err("항목 이름이 비어 있습니다.".to_string());
    }
    if matches!(due_days, Some(d) if d < 0) {
        return Err("마감 일수는 0 이상이어야 합니다.".to_string());
    }
    conn.execute(
        "INSERT INTO check_item (name, due_days, include_weekend, default_done, sort_order, active)
         VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        rusqlite::params![
            name,
            due_days,
            include_weekend as i64,
            default_done as i64,
            sort_order
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// 항목 수정. 이름·마감일수 변경은 **이후 생성되는 체크에만** 적용된다.
/// 이미 저장된 `daily_check.due_date`는 그대로 둔다 — 교사가 고쳐 둔 값을
/// 설정 변경이 소급해 덮으면 안 되기 때문이다.
pub fn update_check_item_impl(
    conn: &Connection,
    id: i64,
    name: &str,
    due_days: Option<i64>,
    include_weekend: bool,
    default_done: bool,
    sort_order: i64,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("항목 이름이 비어 있습니다.".to_string());
    }
    conn.execute(
        "UPDATE check_item
         SET name = ?1, due_days = ?2, include_weekend = ?3, default_done = ?4, sort_order = ?5
         WHERE id = ?6",
        rusqlite::params![
            name,
            due_days,
            include_weekend as i64,
            default_done as i64,
            sort_order,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 삭제 대신 비활성. 지우면 과거 체크 기록이 CASCADE로 함께 사라진다.
pub fn deactivate_check_item_impl(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute(
        "UPDATE check_item SET active = 0 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ── 하루 단위 체크 ────────────────────────────────────────────

/// 구간이 생긴 (학생, 날짜)에 활성 항목만큼 체크 행을 채운다.
///
/// `due_base`는 마감일 계산의 기준일이다. 하루 입력이면 그날, 기간 일괄 입력이면
/// 그룹의 마지막 날이다. 이미 있는 행은 건드리지 않는다 — 교사가 고친 마감일이나
/// 완료 표시를 덮어쓰면 안 된다.
pub fn ensure_daily_checks_inner(
    conn: &Connection,
    student_id: i64,
    date: &str,
    due_base: &str,
) -> Result<(), String> {
    let items = get_check_items_impl(conn, true)?;
    let base = parse_date(due_base)?;

    for item in items {
        let due = item
            .due_days
            .map(|d| format_date(due_date(base, d, item.include_weekend)));

        conn.execute(
            "INSERT OR IGNORE INTO daily_check (student_id, date, item_id, done, due_date, done_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                student_id,
                date,
                item.id,
                item.default_done as i64,
                due,
                if item.default_done {
                    Some(Local::now().to_rfc3339())
                } else {
                    None
                }
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn set_check_impl(
    conn: &Connection,
    student_id: i64,
    date: &str,
    item_id: i64,
    done: bool,
) -> Result<(), String> {
    let done_at = if done {
        Some(Local::now().to_rfc3339())
    } else {
        None
    };
    let changed = conn
        .execute(
            "UPDATE daily_check SET done = ?1, done_at = ?2
             WHERE student_id = ?3 AND date = ?4 AND item_id = ?5",
            rusqlite::params![done as i64, done_at, student_id, date, item_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("체크할 항목이 없습니다. 먼저 출결을 입력해주세요.".to_string());
    }
    Ok(())
}

/// 교사가 고친 마감일. 계산이 틀린 경우의 탈출구다.
pub fn set_check_due_impl(
    conn: &Connection,
    student_id: i64,
    date: &str,
    item_id: i64,
    due: Option<&str>,
) -> Result<(), String> {
    if let Some(d) = due {
        parse_date(d)?;
    }
    conn.execute(
        "UPDATE daily_check SET due_date = ?1
         WHERE student_id = ?2 AND date = ?3 AND item_id = ?4",
        rusqlite::params![due, student_id, date, item_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 기간 일괄 입력으로 만들어진 묶음을 한 번에 토글한다.
/// 서류 한 장이 기간 전체를 덮으므로, 날짜별로 스무 번 누르게 하지 않는다.
pub fn set_group_check_impl(
    conn: &Connection,
    group_id: &str,
    item_id: i64,
    done: bool,
) -> Result<i64, String> {
    let done_at = if done {
        Some(Local::now().to_rfc3339())
    } else {
        None
    };
    with_transaction(conn, || {
        let n = conn
            .execute(
                "UPDATE daily_check SET done = ?1, done_at = ?2
                 WHERE item_id = ?3
                   AND (student_id, date) IN (
                       SELECT student_id, date FROM absence_span WHERE group_id = ?4
                   )",
                rusqlite::params![done as i64, done_at, item_id, group_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(n as i64)
    })
}

// ── 미제출자 ──────────────────────────────────────────────────

/// 미완료 체크 목록. 마감이 지난 것부터 보여준다.
pub fn get_pending_impl(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
    today: &str,
) -> Result<Vec<PendingRow>, String> {
    let today_d = parse_date(today)?;
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.grade, s.class_no, s.number, s.name, s.guardian_phone,
                    dc.date, dc.item_id, ci.name, dc.due_date
             FROM daily_check dc
             JOIN student s ON s.id = dc.student_id
             JOIN check_item ci ON ci.id = dc.item_id
             WHERE dc.done = 0 AND ci.active = 1
               AND s.year_id = ?1 AND s.grade = ?2 AND s.class_no = ?3
             ORDER BY (dc.due_date IS NULL), dc.due_date, s.number, ci.sort_order",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![year_id, grade, class_no], |r| {
            let due: Option<String> = r.get(9)?;
            Ok(PendingRow {
                student_id: r.get(0)?,
                grade: r.get(1)?,
                class_no: r.get(2)?,
                number: r.get(3)?,
                name: r.get(4)?,
                guardian_phone: r.get(5)?,
                date: r.get(6)?,
                item_id: r.get(7)?,
                item_name: r.get(8)?,
                due_date: due,
                days_overdue: None,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 경과일은 SQL이 아니라 여기서 센다. 날짜 산술을 한 군데(due.rs)로 모아 둔다.
    let rows = rows
        .into_iter()
        .map(|mut row| {
            row.days_overdue = row
                .due_date
                .as_deref()
                .and_then(|d| parse_date(d).ok())
                .map(|d| days_overdue(d, today_d));
            row
        })
        .collect();

    Ok(rows)
}

pub fn get_pending_summary_impl(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
) -> Result<Vec<PendingSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT ci.id, ci.name, COUNT(*)
             FROM daily_check dc
             JOIN student s ON s.id = dc.student_id
             JOIN check_item ci ON ci.id = dc.item_id
             WHERE dc.done = 0 AND ci.active = 1
               AND s.year_id = ?1 AND s.grade = ?2 AND s.class_no = ?3
             GROUP BY ci.id, ci.name
             ORDER BY ci.sort_order, ci.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![year_id, grade, class_no], |r| {
            Ok(PendingSummary {
                item_id: r.get(0)?,
                item_name: r.get(1)?,
                count: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_check_items(
    db: State<DbState>,
    active_only: Option<bool>,
) -> Result<Vec<CheckItemDef>, String> {
    with_conn(&db, |c| {
        get_check_items_impl(c, active_only.unwrap_or(true))
    })
}

#[tauri::command]
pub fn create_check_item(
    db: State<DbState>,
    name: String,
    due_days: Option<i64>,
    include_weekend: bool,
    default_done: bool,
    sort_order: i64,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        create_check_item_impl(c, &name, due_days, include_weekend, default_done, sort_order)
    })
}

#[tauri::command]
pub fn update_check_item(
    db: State<DbState>,
    id: i64,
    name: String,
    due_days: Option<i64>,
    include_weekend: bool,
    default_done: bool,
    sort_order: i64,
) -> Result<(), String> {
    with_conn(&db, |c| {
        update_check_item_impl(
            c,
            id,
            &name,
            due_days,
            include_weekend,
            default_done,
            sort_order,
        )
    })
}

#[tauri::command]
pub fn deactivate_check_item(db: State<DbState>, id: i64) -> Result<(), String> {
    with_conn(&db, |c| deactivate_check_item_impl(c, id))
}

#[tauri::command]
pub fn set_check(
    db: State<DbState>,
    student_id: i64,
    date: String,
    item_id: i64,
    done: bool,
) -> Result<(), String> {
    with_conn(&db, |c| set_check_impl(c, student_id, &date, item_id, done))
}

#[tauri::command]
pub fn set_check_due(
    db: State<DbState>,
    student_id: i64,
    date: String,
    item_id: i64,
    due: Option<String>,
) -> Result<(), String> {
    with_conn(&db, |c| {
        set_check_due_impl(c, student_id, &date, item_id, due.as_deref())
    })
}

#[tauri::command]
pub fn set_group_check(
    db: State<DbState>,
    group_id: String,
    item_id: i64,
    done: bool,
) -> Result<i64, String> {
    with_conn(&db, |c| set_group_check_impl(c, &group_id, item_id, done))
}

#[tauri::command]
pub fn get_pending(
    db: State<DbState>,
    year_id: i64,
    grade: i64,
    class_no: i64,
    today: String,
) -> Result<Vec<PendingRow>, String> {
    with_conn(&db, |c| {
        get_pending_impl(c, year_id, grade, class_no, &today)
    })
}

#[tauri::command]
pub fn get_pending_summary(
    db: State<DbState>,
    year_id: i64,
    grade: i64,
    class_no: i64,
) -> Result<Vec<PendingSummary>, String> {
    with_conn(&db, |c| get_pending_summary_impl(c, year_id, grade, class_no))
}
