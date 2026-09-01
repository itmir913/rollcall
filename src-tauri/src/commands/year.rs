//! 학년도. 학생은 학년도에 속하므로, 해가 바뀌어도 작년 기록이 그대로 남는다.

use crate::commands::with_conn;
use crate::state::{constraint_err, DbState};
use crate::types::AcademicYearItem;
use rusqlite::Connection;
use tauri::State;

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
    conn.execute(
        "UPDATE academic_year SET starts_on = ?1, ends_on = ?2 WHERE id = ?3",
        rusqlite::params![starts_on, ends_on, id],
    )
    .map_err(|e| e.to_string())?;
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
