//! 학생 명단. 붙여넣기·CSV·직접 입력이 모두 `RosterEntry` 목록으로 수렴한 뒤
//! 같은 미리보기·저장 경로를 탄다.
//!
//! 재가져오기는 **교체가 아니라 차분**이다. 사라진 번호를 지우면 그 학생의 출결
//! 기록이 FK CASCADE로 함께 사라진다. 전출은 삭제가 아니므로 `enrolled_to`를 채운다.

use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::state::{constraint_err, DbState};
use crate::types::*;
use rusqlite::Connection;
use tauri::State;

// ── 붙여넣기 파싱 ─────────────────────────────────────────────

/// 나이스 명렬표에서 두 열을 복사한 텍스트를 파싱한다.
///
/// 탭 구분 텍스트만 본다. 명렬표 양식이 무엇이든 상관없게 하려는 것이다.
/// 쉼표 구분(CSV 붙여넣기)도 같이 받는다 — 교사가 어느 쪽을 붙여넣었는지
/// 프로그램이 되묻지 않는 편이 빠르다.
pub fn parse_roster_text(text: &str) -> Vec<RosterEntry> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cells: Vec<&str> = if line.contains('\t') {
            line.split('\t').collect()
        } else {
            line.split(',').collect()
        };
        let cells: Vec<String> = cells.iter().map(|c| c.trim().to_string()).collect();

        // 번호가 될 수 있는 첫 칸과, 그 뒤의 비어 있지 않은 첫 칸을 이름으로 본다.
        let Some(pos) = cells.iter().position(|c| c.parse::<i64>().is_ok()) else {
            continue; // 머리글 줄 등
        };
        let number: i64 = cells[pos].parse().unwrap();
        let Some(name) = cells.iter().skip(pos + 1).find(|c| !c.is_empty()) else {
            continue;
        };
        if number < 1 {
            continue;
        }
        out.push(RosterEntry {
            number,
            name: name.clone(),
            guardian_phone: None,
        });
    }
    out
}

// ── 차분 ──────────────────────────────────────────────────────

/// 재학 중인 학생 목록과 들어온 명단을 비교한다. DB를 건드리지 않는 순수 함수다.
///
/// - 새 번호 → `added`
/// - 사라진 번호 → `withdrawn` (삭제가 아니라 전출)
/// - 번호 같고 이름 다름 → `renamed`. **판정할 수 없으므로 교사가 고른다.**
///   전학 온 학생이 번호를 물려받은 것일 수도, 개명일 수도, 명렬표 오타일 수도 있다.
pub fn diff_roster(
    current: &[(i64, i64, String)], // (student_id, number, name)
    incoming: &[RosterEntry],
) -> Vec<RosterDiffRow> {
    let mut rows = Vec::new();

    for entry in incoming {
        match current.iter().find(|(_, n, _)| *n == entry.number) {
            Some((id, _, name)) if *name == entry.name => rows.push(RosterDiffRow {
                number: entry.number,
                incoming_name: Some(entry.name.clone()),
                current_name: Some(name.clone()),
                student_id: Some(*id),
                action: "unchanged".into(),
            }),
            Some((id, _, name)) => rows.push(RosterDiffRow {
                number: entry.number,
                incoming_name: Some(entry.name.clone()),
                current_name: Some(name.clone()),
                student_id: Some(*id),
                action: "renamed".into(),
            }),
            None => rows.push(RosterDiffRow {
                number: entry.number,
                incoming_name: Some(entry.name.clone()),
                current_name: None,
                student_id: None,
                action: "added".into(),
            }),
        }
    }

    for (id, number, name) in current {
        if !incoming.iter().any(|e| e.number == *number) {
            rows.push(RosterDiffRow {
                number: *number,
                incoming_name: None,
                current_name: Some(name.clone()),
                student_id: Some(*id),
                action: "withdrawn".into(),
            });
        }
    }

    rows.sort_by_key(|r| r.number);
    rows
}

// ── DB ────────────────────────────────────────────────────────

fn map_student(row: &rusqlite::Row) -> rusqlite::Result<StudentItem> {
    Ok(StudentItem {
        id: row.get(0)?,
        year_id: row.get(1)?,
        grade: row.get(2)?,
        class_no: row.get(3)?,
        number: row.get(4)?,
        name: row.get(5)?,
        guardian_phone: row.get(6)?,
        enrolled_from: row.get(7)?,
        enrolled_to: row.get(8)?,
    })
}

const STUDENT_COLS: &str =
    "id, year_id, grade, class_no, number, name, guardian_phone, enrolled_from, enrolled_to";

/// 그날 재학 중인 학생만. 격자가 이 목록으로 그려진다.
pub fn get_students_on_impl(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
    date: &str,
) -> Result<Vec<StudentItem>, String> {
    let sql = format!(
        "SELECT {STUDENT_COLS} FROM student
         WHERE year_id = ?1 AND grade = ?2 AND class_no = ?3
           AND enrolled_from <= ?4
           AND (enrolled_to IS NULL OR ?4 <= enrolled_to)
         ORDER BY number"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![year_id, grade, class_no, date], |r| {
            map_student(r)
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 재학 중인 학생 전체(전출 제외).
pub fn get_students_impl(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
) -> Result<Vec<StudentItem>, String> {
    let sql = format!(
        "SELECT {STUDENT_COLS} FROM student
         WHERE year_id = ?1 AND grade = ?2 AND class_no = ?3 AND enrolled_to IS NULL
         ORDER BY number"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![year_id, grade, class_no], |r| {
            map_student(r)
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn current_tuples(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
) -> Result<Vec<(i64, i64, String)>, String> {
    Ok(get_students_impl(conn, year_id, grade, class_no)?
        .into_iter()
        .map(|s| (s.id, s.number, s.name))
        .collect())
}

pub fn preview_roster_impl(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
    incoming: &[RosterEntry],
) -> Result<Vec<RosterDiffRow>, String> {
    let current = current_tuples(conn, year_id, grade, class_no)?;
    Ok(diff_roster(&current, incoming))
}

/// 미리보기에서 교사가 확정한 행만 받아 적용한다.
///
/// 프론트가 `action`을 바꿔 보낼 수 있다는 것이 요점이다. 번호 같고 이름 다름을
/// 개명(`renamed`)으로 볼지, 전출+전입(`withdrawn` + `added`)으로 볼지는 교사가 정한다.
pub fn apply_roster_impl(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
    effective_date: &str,
    rows: &[RosterDiffRow],
) -> Result<RosterApplyResult, String> {
    with_transaction(conn, || {
        let mut result = RosterApplyResult {
            added: 0,
            renamed: 0,
            withdrawn: 0,
        };

        // 전출을 먼저 처리한다. 같은 번호를 새 학생이 물려받는 경우,
        // 부분 유니크 인덱스(ux_student_active)가 순서를 강제하기 때문이다.
        for row in rows.iter().filter(|r| r.action == "withdrawn") {
            let id = row
                .student_id
                .ok_or_else(|| format!("{}번: 전출 대상 학생을 찾을 수 없습니다.", row.number))?;
            conn.execute(
                "UPDATE student SET enrolled_to = ?1 WHERE id = ?2 AND enrolled_to IS NULL",
                rusqlite::params![effective_date, id],
            )
            .map_err(|e| e.to_string())?;
            result.withdrawn += 1;
        }

        for row in rows {
            match row.action.as_str() {
                "added" => {
                    let name = row.incoming_name.as_deref().ok_or_else(|| {
                        format!("{}번: 추가할 이름이 비어 있습니다.", row.number)
                    })?;
                    if name.trim().is_empty() {
                        return Err(format!("{}번: 추가할 이름이 비어 있습니다.", row.number));
                    }
                    conn.execute(
                        "INSERT INTO student
                           (year_id, grade, class_no, number, name, enrolled_from)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![year_id, grade, class_no, row.number, name, effective_date],
                    )
                    .map_err(|e| {
                        constraint_err(
                            &e,
                            &format!("이미 같은 번호의 재학생이 있습니다: {}번", row.number),
                        )
                    })?;
                    result.added += 1;
                }
                "renamed" => {
                    let id = row.student_id.ok_or_else(|| {
                        format!("{}번: 이름을 고칠 학생을 찾을 수 없습니다.", row.number)
                    })?;
                    let name = row
                        .incoming_name
                        .as_deref()
                        .ok_or_else(|| format!("{}번: 새 이름이 비어 있습니다.", row.number))?;
                    conn.execute(
                        "UPDATE student SET name = ?1 WHERE id = ?2",
                        rusqlite::params![name, id],
                    )
                    .map_err(|e| e.to_string())?;
                    result.renamed += 1;
                }
                // unchanged / withdrawn(위에서 처리) 은 여기서 할 일이 없다.
                _ => {}
            }
        }

        Ok(result)
    })
}

pub fn update_student_impl(
    conn: &Connection,
    id: i64,
    number: i64,
    name: &str,
    guardian_phone: Option<&str>,
) -> Result<(), String> {
    if number < 1 {
        return Err(format!("번호는 1 이상이어야 합니다: {number}"));
    }
    if name.trim().is_empty() {
        return Err("이름이 비어 있습니다.".to_string());
    }
    conn.execute(
        "UPDATE student SET number = ?1, name = ?2, guardian_phone = ?3 WHERE id = ?4",
        rusqlite::params![number, name, guardian_phone, id],
    )
    .map_err(|e| constraint_err(&e, &format!("이미 같은 번호의 재학생이 있습니다: {number}번")))?;
    Ok(())
}

/// 전출 처리. 삭제가 아니다.
pub fn withdraw_student_impl(conn: &Connection, id: i64, date: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE student SET enrolled_to = ?1 WHERE id = ?2",
        rusqlite::params![date, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn parse_roster(text: String) -> Vec<RosterEntry> {
    parse_roster_text(&text)
}

#[tauri::command]
pub fn get_students(
    db: State<DbState>,
    year_id: i64,
    grade: i64,
    class_no: i64,
) -> Result<Vec<StudentItem>, String> {
    with_conn(&db, |c| get_students_impl(c, year_id, grade, class_no))
}

#[tauri::command]
pub fn preview_roster(
    db: State<DbState>,
    year_id: i64,
    grade: i64,
    class_no: i64,
    entries: Vec<RosterEntry>,
) -> Result<Vec<RosterDiffRow>, String> {
    with_conn(&db, |c| {
        preview_roster_impl(c, year_id, grade, class_no, &entries)
    })
}

#[tauri::command]
pub fn apply_roster(
    db: State<DbState>,
    year_id: i64,
    grade: i64,
    class_no: i64,
    effective_date: String,
    rows: Vec<RosterDiffRow>,
) -> Result<RosterApplyResult, String> {
    with_conn(&db, |c| {
        apply_roster_impl(c, year_id, grade, class_no, &effective_date, &rows)
    })
}

#[tauri::command]
pub fn update_student(
    db: State<DbState>,
    id: i64,
    number: i64,
    name: String,
    guardian_phone: Option<String>,
) -> Result<(), String> {
    with_conn(&db, |c| {
        update_student_impl(c, id, number, &name, guardian_phone.as_deref())
    })
}

#[tauri::command]
pub fn withdraw_student(db: State<DbState>, id: i64, date: String) -> Result<(), String> {
    with_conn(&db, |c| withdraw_student_impl(c, id, &date))
}
