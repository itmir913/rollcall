//! 학생 명단과 연락처.
//!
//! 명렬표가 (학년, 반, 번호, 이름)이므로 **학급 정보는 명렬표에서 나온다.** 첫 실행에서
//! "우리 반이 몇 학년 몇 반입니까"를 따로 묻지 않으려는 것이다. 두 열만 들어 있는
//! 파일이면 화면이 고른 학급을 쓴다.
//!
//! **파일 읽기는 프론트에서 한다**(`services/rosterFile.js`). xlsx를 다룰 라이브러리가
//! 거기 있기 때문이고, 그것은 파일 형식 문제지 업무 규칙이 아니다. 여기로 넘어오는
//! 것은 이미 (학년, 반, 번호, 이름)으로 정리된 목록이다.
//!
//! 재가져오기는 **교체가 아니라 차분**이다. 사라진 번호를 지우면 그 학생의 출결
//! 기록이 FK CASCADE로 함께 사라진다. 전출은 삭제가 아니므로 `enrolled_to`를 채운다.

use crate::commands::with_conn;
use crate::db::with_transaction;
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

const STUDENT_COLS: &str = "id, year_id, grade, class_no, number, name, enrolled_from, enrolled_to";

fn map_student(row: &rusqlite::Row) -> rusqlite::Result<StudentItem> {
    Ok(StudentItem {
        id: row.get(0)?,
        year_id: row.get(1)?,
        grade: row.get(2)?,
        class_no: row.get(3)?,
        number: row.get(4)?,
        name: row.get(5)?,
        enrolled_from: row.get(6)?,
        enrolled_to: row.get(7)?,
    })
}

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
        .query_map(rusqlite::params![year_id, grade, class_no, date], map_student)
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
        .query_map(rusqlite::params![year_id, grade, class_no], map_student)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 이 학년도에 명단이 들어 있는 학급 목록. 첫 실행 뒤 학급 전환에 쓴다.
pub fn get_classes_impl(conn: &Connection, year_id: i64) -> Result<Vec<(i64, i64)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT grade, class_no FROM student
             WHERE year_id = ?1 AND enrolled_to IS NULL
             ORDER BY grade, class_no",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![year_id], |r| Ok((r.get(0)?, r.get(1)?)))
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
                    let name = row.incoming_name.as_deref().unwrap_or("");
                    if name.trim().is_empty() {
                        return Err(format!("{}번: 추가할 이름이 비어 있습니다.", row.number));
                    }
                    conn.execute(
                        "INSERT INTO student
                           (year_id, grade, class_no, number, name, enrolled_from)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        rusqlite::params![
                            year_id, grade, class_no, row.number, name, effective_date
                        ],
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
) -> Result<(), String> {
    if number < 1 {
        return Err(format!("번호는 1 이상이어야 합니다: {number}"));
    }
    if name.trim().is_empty() {
        return Err("이름이 비어 있습니다.".to_string());
    }
    conn.execute(
        "UPDATE student SET number = ?1, name = ?2 WHERE id = ?3",
        rusqlite::params![number, name, id],
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

// ── 연락처 ────────────────────────────────────────────────────

pub fn get_contacts_impl(conn: &Connection, student_id: i64) -> Result<Vec<ContactItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, label, value, note, sort_order FROM contact
             WHERE student_id = ?1 ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![student_id], |r| {
            Ok(ContactItem {
                id: r.get(0)?,
                label: r.get(1)?,
                value: r.get(2)?,
                note: r.get(3)?,
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
        if c.label.trim().is_empty() {
            return Err("연락처 이름(관계)이 비어 있습니다.".to_string());
        }
        if c.value.trim().is_empty() {
            return Err(format!("{}의 번호가 비어 있습니다.", c.label));
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
                "INSERT INTO contact (student_id, label, value, note, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![student_id, c.label.trim(), c.value.trim(), c.note, i as i64],
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
pub fn get_students(
    db: State<DbState>,
    year_id: i64,
    grade: i64,
    class_no: i64,
) -> Result<Vec<StudentItem>, String> {
    with_conn(&db, |c| get_students_impl(c, year_id, grade, class_no))
}

#[tauri::command]
pub fn get_classes(db: State<DbState>, year_id: i64) -> Result<Vec<(i64, i64)>, String> {
    with_conn(&db, |c| get_classes_impl(c, year_id))
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
) -> Result<(), String> {
    with_conn(&db, |c| update_student_impl(c, id, number, &name))
}

#[tauri::command]
pub fn withdraw_student(db: State<DbState>, id: i64, date: String) -> Result<(), String> {
    with_conn(&db, |c| withdraw_student_impl(c, id, &date))
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
