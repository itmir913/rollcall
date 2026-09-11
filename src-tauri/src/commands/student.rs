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
//! 명단에만 잇는다. 담임 반 학생이 내 교과 강좌에도 들어오면 학생 행은 하나,
//! 소속이 둘이어야 하기 때문이다.
//!
//! 재가져오기는 **교체가 아니라 차분**이다. 사라진 번호를 지우면 그 학생의 출결
//! 기록이 FK CASCADE로 함께 사라진다. 명단에서 사라진 번호는 `class_member.left_on`을
//! 채운다 — **학생의 `enrolled_to`가 아니다.** 내 명단에서 빠진 것과 학교를 떠난 것은
//! 다른 일이고, 담임이 아는 것은 앞엣것뿐이다.

use crate::commands::class::{homeroom_scope, homeroom_seat, members_of};
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

/// 지금 내 명단. 소속은 `class_member`가 말하므로 학년 · 반으로 거르지 않는다.
pub fn get_students_impl(conn: &Connection, class_id: i64) -> Result<Vec<StudentItem>, String> {
    let scope = homeroom_scope(conn, class_id)?;
    members_of(conn, scope.id)
}

fn current_tuples(
    conn: &Connection,
    class_id: i64,
) -> Result<Vec<(i64, i64, String)>, String> {
    Ok(members_of(conn, class_id)?
        .into_iter()
        .map(|s| (s.id, s.number, s.name))
        .collect())
}

pub fn preview_roster_impl(
    conn: &Connection,
    class_id: i64,
    incoming: &[RosterEntry],
) -> Result<Vec<RosterDiffRow>, String> {
    let scope = homeroom_scope(conn, class_id)?;
    let current = current_tuples(conn, scope.id)?;
    Ok(diff_roster(&current, incoming))
}

// ── 학적 자리와 소속 ──────────────────────────────────────────

/// 그 학적 자리에 이미 앉아 있는 학생. `ux_student_seat`이 가리키는 한 자리다.
fn seat_holder(
    conn: &Connection,
    school_id: i64,
    year_id: i64,
    grade: i64,
    class_no: i64,
    number: i64,
) -> Result<Option<(i64, String)>, String> {
    conn.query_row(
        "SELECT id, name FROM student
          WHERE school_id = ?1 AND year_id = ?2 AND grade = ?3 AND class_no = ?4
            AND number = ?5 AND enrolled_to IS NULL
          ORDER BY id LIMIT 1",
        rusqlite::params![school_id, year_id, grade, class_no, number],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other.to_string()),
    })
}

/// 학생을 내 명단에 잇는다. 예전에 있다가 빠진 학생이면 `left_on`을 지워 되살린다.
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
    // 5월 격자에도 그 학생이 선다. 소속을 구간으로 바꾸면 모든 명단 질의가 날짜마다
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

/// 미리보기에서 교사가 확정한 행만 받아 적용한다.
///
/// 프론트가 `action`을 바꿔 보낼 수 있다는 것이 요점이다. 번호 같고 이름 다름을
/// 개명(`renamed`)으로 볼지, 명단 교체(`withdrawn` + `added`)로 볼지는 교사가 정한다.
pub fn apply_roster_impl(
    conn: &Connection,
    class_id: i64,
    effective_date: &str,
    rows: &[RosterDiffRow],
) -> Result<RosterApplyResult, String> {
    let scope = homeroom_scope(conn, class_id)?;
    let (grade, class_no) = homeroom_seat(&scope)?;
    let (school_id, year_id) = (scope.school_id, scope.year_id);

    with_transaction(conn, || {
        let mut result = RosterApplyResult {
            added: 0,
            renamed: 0,
            withdrawn: 0,
        };

        // 명단에서 빼는 것을 먼저 처리한다. 같은 번호를 새 학생이 물려받는 경우,
        // 학적 자리를 비우는 순서가 강제되기 때문이다.
        for row in rows.iter().filter(|r| r.action == "withdrawn") {
            let id = row
                .student_id
                .ok_or_else(|| format!("{}번: 명단에서 뺄 학생을 찾을 수 없습니다.", row.number))?;
            // **학생의 `enrolled_to`를 채우지 않는다.** 내 명단에서 빠진 것과 학교를
            // 떠난 것은 다른 일이고, 담임이 아는 것은 앞엣것뿐이다.
            let n = conn
                .execute(
                    "UPDATE class_member SET left_on = ?1
                      WHERE class_id = ?2 AND student_id = ?3 AND left_on IS NULL",
                    rusqlite::params![effective_date, scope.id, id],
                )
                .map_err(|e| e.to_string())?;
            if n == 0 {
                return Err(format!(
                    "{}번: 명단에서 뺄 학생을 찾을 수 없습니다: {id}",
                    row.number
                ));
            }
            result.withdrawn += 1;
        }

        for row in rows {
            match row.action.as_str() {
                "added" => {
                    let name = row.incoming_name.as_deref().unwrap_or("").trim();
                    if name.is_empty() {
                        return Err(format!("{}번: 추가할 이름이 비어 있습니다.", row.number));
                    }

                    // **이미 있는 학생을 다시 만들지 않는다.** 같은 학적 자리에 같은
                    // 이름이 앉아 있으면 그 학생이고(다른 명단에서 들어온 경우다),
                    // 명단에만 이으면 된다.
                    let holder =
                        seat_holder(conn, school_id, year_id, grade, class_no, row.number)?;
                    let student_id = match holder {
                        Some((id, held)) if held == name => id,
                        Some((id, _)) => {
                            // 같은 자리에 다른 이름이 앉아 있다. 한 자리에 두 명일 수
                            // 없으므로 앞사람은 그 자리를 떠난 것이다 — 학적을 마감해
                            // 자리를 비운다. 교사의 판단을 대신하는 것이 아니라,
                            // 가리킬 수 없는 행이 생기지 않게 하는 것이다.
                            conn.execute(
                                "UPDATE student SET enrolled_to = ?1 WHERE id = ?2",
                                rusqlite::params![effective_date, id],
                            )
                            .map_err(|e| e.to_string())?;
                            insert_student(
                                conn,
                                school_id,
                                year_id,
                                grade,
                                class_no,
                                row.number,
                                name,
                                effective_date,
                            )?
                        }
                        None => insert_student(
                            conn,
                            school_id,
                            year_id,
                            grade,
                            class_no,
                            row.number,
                            name,
                            effective_date,
                        )?,
                    };
                    join_class(conn, scope.id, student_id, effective_date)?;
                    result.added += 1;
                }
                "renamed" => {
                    let id = row.student_id.ok_or_else(|| {
                        format!("{}번: 이름을 고칠 학생을 찾을 수 없습니다.", row.number)
                    })?;
                    let name = row
                        .incoming_name
                        .as_deref()
                        .map(str::trim)
                        .filter(|n| !n.is_empty())
                        .ok_or_else(|| format!("{}번: 새 이름이 비어 있습니다.", row.number))?;
                    let n = conn
                        .execute(
                            "UPDATE student SET name = ?1 WHERE id = ?2 AND school_id = ?3",
                            rusqlite::params![name, id, school_id],
                        )
                        .map_err(|e| e.to_string())?;
                    if n == 0 {
                        return Err(format!(
                            "{}번: 이름을 고칠 학생을 찾을 수 없습니다: {id}",
                            row.number
                        ));
                    }
                    result.renamed += 1;
                }
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
    year_id: i64,
    grade: i64,
    class_no: i64,
    number: i64,
    name: &str,
    enrolled_from: &str,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO student
           (school_id, year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![school_id, year_id, grade, class_no, number, name, enrolled_from],
    )
    .map_err(|e| {
        constraint_err(
            &e,
            &format!("이미 같은 번호의 재학생이 있습니다: {number}번"),
        )
    })?;
    Ok(conn.last_insert_rowid())
}

/// 번호와 이름을 고친다. **내 명단의 학생만 고친다.**
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
    let scope = homeroom_scope(conn, class_id)?;
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
pub fn withdraw_student_impl(
    conn: &Connection,
    class_id: i64,
    id: i64,
    date: &str,
) -> Result<(), String> {
    let scope = homeroom_scope(conn, class_id)?;
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
