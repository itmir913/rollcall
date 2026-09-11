//! 학년도. **이 앱 계층의 뿌리다** — 학년도 → 학교 → 맡은 것(담임 | 교과).
//!
//! 학년도가 바뀌면 학교 설정도 처음부터 다시 한다. 2026학년도는 A학교 하나,
//! 2027학년도는 B · C 두 학교인 일이 실제로 있다(순회 교사). 그래서 지난해 행을
//! 고치지 않고 새 학년도를 만든다 — 작년 기록이 작년 학교에 그대로 남는다.
//!
//! **`starts_on` · `ends_on`은 날짜 울타리가 아니다.** 학년도가 2026이어도
//! 2027년 3월 출결 입력을 막지 않는다. 두 값은 통계가 참고하는 기준일 뿐이다.

use crate::commands::with_conn;
use crate::due::parse_date;
use crate::state::{constraint_err, DbState};
use crate::types::AcademicYearItem;
use rusqlite::Connection;
use tauri::State;

/// 비어 있어도 되는 날짜 칸. 값이 있으면 ISO여야 한다.
///
/// **다른 커맨드가 전부 `parse_date`로 확인하는데 여기만 빠져 있었다.** 형식이 깨진
/// 날짜가 들어가면 통계가 그 학년도를 세지 못하고, 그때는 이미 기록이 쌓인 뒤다.
fn check_optional_date(value: Option<&str>) -> Result<(), String> {
    match value {
        Some(v) if !v.trim().is_empty() => parse_date(v).map(|_| ()),
        _ => Ok(()),
    }
}

pub fn get_years_impl(conn: &Connection) -> Result<Vec<AcademicYearItem>, String> {
    let mut stmt = conn
        .prepare("SELECT id, year, starts_on, ends_on FROM academic_year ORDER BY year DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(AcademicYearItem {
                id: r.get(0)?,
                year: r.get(1)?,
                starts_on: r.get(2)?,
                ends_on: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

pub fn create_year_impl(
    conn: &Connection,
    year: i64,
    starts_on: Option<&str>,
    ends_on: Option<&str>,
) -> Result<i64, String> {
    if year < 1900 {
        return Err(format!("학년도가 올바르지 않습니다: {year}"));
    }
    check_optional_date(starts_on)?;
    check_optional_date(ends_on)?;
    conn.execute(
        "INSERT INTO academic_year (year, starts_on, ends_on) VALUES (?1, ?2, ?3)",
        rusqlite::params![year, starts_on, ends_on],
    )
    .map_err(|e| constraint_err(&e, &format!("이미 있는 학년도입니다: {year}")))?;
    Ok(conn.last_insert_rowid())
}

pub fn update_year_impl(
    conn: &Connection,
    id: i64,
    starts_on: Option<&str>,
    ends_on: Option<&str>,
) -> Result<(), String> {
    check_optional_date(starts_on)?;
    check_optional_date(ends_on)?;
    let changed = conn
        .execute(
            "UPDATE academic_year SET starts_on = ?1, ends_on = ?2 WHERE id = ?3",
            rusqlite::params![starts_on, ends_on, id],
        )
        .map_err(|e| e.to_string())?;
    // 바뀐 행이 없으면 없는 학년도다. **조용히 성공하지 않는다** — 화면은 저장된 줄
    // 알고 넘어가고, 교사는 왜 값이 그대로인지 알 방법이 없다.
    if changed == 0 {
        return Err(format!("학년도를 찾을 수 없습니다: {id}"));
    }
    Ok(())
}

#[tauri::command]
pub fn get_years(db: State<DbState>) -> Result<Vec<AcademicYearItem>, String> {
    with_conn(&db, get_years_impl)
}

#[tauri::command]
pub fn create_year(
    db: State<DbState>,
    year: i64,
    starts_on: Option<String>,
    ends_on: Option<String>,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        create_year_impl(c, year, starts_on.as_deref(), ends_on.as_deref())
    })
}

#[tauri::command]
pub fn update_year(
    db: State<DbState>,
    id: i64,
    starts_on: Option<String>,
    ends_on: Option<String>,
) -> Result<(), String> {
    with_conn(&db, |c| {
        update_year_impl(c, id, starts_on.as_deref(), ends_on.as_deref())
    })
}
