//! 학년도. **이 앱 계층의 뿌리다** — 학년도 → 학교 → 담당 학급 · 강좌(담임 | 교과).
//!
//! 학년도가 바뀌면 학교 설정도 처음부터 다시 한다. 2026학년도는 A학교 하나,
//! 2027학년도는 B · C 두 학교인 일이 실제로 있다(순회 교사). 그래서 지난해 행을
//! 고치지 않고 새 학년도를 만든다 — 작년 기록이 작년 학교에 그대로 남는다.
//!
//! **`starts_on` · `ends_on`은 날짜 울타리가 아니다.** 학년도가 2026이어도
//! 2027년 3월 출결 입력을 막지 않는다. 두 값은 통계가 참고하는 기준일 뿐이다.

use crate::commands::config::get_config_impl;
use crate::commands::with_conn;
use crate::db::with_transaction;
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

/// 학년도 목록과 **그 아래에 든 것의 수**.
///
/// 수를 함께 싣는 이유는 삭제 때문이다. 학년도를 지우면 그 아래가 통째로 사라지므로
/// 화면이 묻기 전에 무엇이 사라지는지 보여줘야 하는데, 따로 받아 오는 커맨드를 두면
/// 대화상자를 여는 순간 한 번 더 기다린다.
///
/// **부팅마다 도는 질의라 비용을 확인했다.** 다섯 수가 전부 색인 앞자리로 세는 것이다 —
/// `ix_school_year(year_id, …)` · `ix_class_scope(school_id, …)` ·
/// `ix_student_class(school_id, …)` · `ix_span_class(class_id, …)` ·
/// `ix_session_date(class_id, …)`. 학년도는 많아야 몇 줄이고 한 학년도의 학교도
/// 한둘이라, 가장 큰 표(출결 · 차시)까지 포함해도 색인 구간 훑기 몇 번이다.
/// 여기서 빼면 교사는 "출결 몇 건이 사라지는가"를 모른 채 지우기를 누르게 된다.
const YEAR_SELECT: &str = "
    SELECT y.id, y.year, y.starts_on, y.ends_on,
           (SELECT COUNT(*) FROM school s WHERE s.year_id = y.id),
           (SELECT COUNT(*) FROM teaching_class t
                  JOIN school s ON s.id = t.school_id
                 WHERE s.year_id = y.id),
           (SELECT COUNT(*) FROM student st
                  JOIN school s ON s.id = st.school_id
                 WHERE s.year_id = y.id),
           (SELECT COUNT(*) FROM absence_span a
                  JOIN teaching_class t ON t.id = a.class_id
                  JOIN school s ON s.id = t.school_id
                 WHERE s.year_id = y.id),
           (SELECT COUNT(*) FROM subject_session ss
                  JOIN teaching_class t ON t.id = ss.class_id
                  JOIN school s ON s.id = t.school_id
                 WHERE s.year_id = y.id)
      FROM academic_year y
     ORDER BY y.year DESC";

pub fn get_years_impl(conn: &Connection) -> Result<Vec<AcademicYearItem>, String> {
    let mut stmt = conn.prepare(YEAR_SELECT).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(AcademicYearItem {
                id: r.get(0)?,
                year: r.get(1)?,
                starts_on: r.get(2)?,
                ends_on: r.get(3)?,
                school_count: r.get(4)?,
                class_count: r.get(5)?,
                student_count: r.get(6)?,
                span_count: r.get(7)?,
                session_count: r.get(8)?,
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

/// 지금 보고 있는 자리를 가리키는 `app_config` 열쇠. 학년도가 뿌리이고 나머지는
/// 그 아래를 가리킨다 — 학년도가 사라지면 넷이 함께 가리킬 곳을 잃는다.
/// (`mode`는 화면 취향이지 행을 가리키는 값이 아니라 여기 들어가지 않는다.)
const SCOPE_KEYS: &[&str] = &["yearId", "schoolId", "homeroomClassId", "subjectClassId"];

/// 지우는 학년도가 **지금 보고 있는 학년도이면** 범위 열쇠를 지운다.
///
/// 대신 볼 학년도를 여기서 선택하지 않는다. 이유는 둘이다.
///   · 그 선택은 화면이 이미 한다 — 열쇠가 없거나 목록에 없는 번호면 화면이 목록의
///     첫 학년도로 되돌린다. 여기서도 선택하면 같은 규칙이 두 곳에 생기고, 한쪽을
///     고칠 때 다른 쪽이 남아 조용히 어긋난다.
///   · 열쇠가 없다는 것은 "아직 선택하지 않았다"는 뜻이고, 지운 직후가 정확히 그 상태다.
///     없는 행의 번호를 남겨 두는 것보다 비어 있는 편이 사실에 가깝다.
///
/// 학교 · 학급 열쇠도 함께 지운다. 그 행들은 CASCADE로 이미 사라졌는데 학년도 열쇠만
/// 비우면, 다음 실행에서 없는 학교 · 학급 번호가 복원되어 다른 학년도의 화면에 얹힌다.
///
/// **다른 학년도를 지울 때는 손대지 않는다.** 그때의 열쇠들은 남아 있는 학년도의
/// 자리를 가리키고 있고, 지우면 교사가 보던 화면이 이유 없이 처음으로 돌아간다.
fn forget_scope_if_current(conn: &Connection, year_id: i64) -> Result<(), String> {
    let current = get_config_impl(conn, "yearId")?.and_then(|v| v.trim().parse::<i64>().ok());
    if current != Some(year_id) {
        return Ok(());
    }
    for key in SCOPE_KEYS {
        conn.execute(
            "DELETE FROM app_config WHERE config_key = ?1",
            rusqlite::params![key],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 학년도를 **실제로 지운다.** 되돌릴 수 없다.
///
/// `school` → `teaching_class` → `class_member` · `absence_span` · `subject_session`이
/// 전부 `ON DELETE CASCADE`라, 그 학년도의 기록이 통째로 사라진다. 학교를 마감하는 것
/// (`retire_school`)과 다른 동작이다 — 이쪽은 잘못 만든 학년도를 목록에서 없애는
/// 자리이고, Welcome에서 만든 것이 초안 없이 곧장 DB에 들어가기 때문에 필요하다.
/// 무엇이 함께 사라지는지는 `get_years`가 세어 화면에 싣는다.
///
/// **마지막 하나는 지우지 않는다.** 학년도가 0개가 되면 학교도 담당 학급 · 강좌도
/// 만들 수 없어, 교사가 아무것도 할 수 없는 화면에 갇힌다. 잘못 만든 학년도가 하나뿐일
/// 때의 옳은 순서는 맞는 학년도를 먼저 만들고 그다음에 지우는 것이라, 거절 문구가
/// 그것을 말한다.
pub fn delete_year_impl(conn: &Connection, year_id: i64) -> Result<(), String> {
    with_transaction(conn, || {
        // 없는 학년도를 먼저 거른다. 남은 것이 하나뿐일 때 엉뚱한 번호를 넘겨도
        // "마지막 학년도"라고 답하면, 교사는 지우려던 줄이 그대로 남은 이유를
        // 잘못 읽는다.
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM academic_year WHERE id = ?1",
                rusqlite::params![year_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists == 0 {
            return Err(format!("학년도를 찾을 수 없습니다: {year_id}"));
        }

        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM academic_year", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if total <= 1 {
            return Err(
                "마지막 학년도는 지울 수 없습니다. 학년도가 없으면 학교도 담당 학급 · 강좌도 \
                 만들 수 없습니다. 새 학년도를 먼저 만든 뒤에 지우세요."
                    .to_string(),
            );
        }

        conn.execute(
            "DELETE FROM academic_year WHERE id = ?1",
            rusqlite::params![year_id],
        )
        .map_err(|e| e.to_string())?;

        forget_scope_if_current(conn, year_id)
    })
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

#[tauri::command]
pub fn delete_year(db: State<DbState>, year_id: i64) -> Result<(), String> {
    with_conn(&db, |c| delete_year_impl(c, year_id))
}
