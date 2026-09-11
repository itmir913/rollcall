//! 담임 학급과 교과 강좌, 그리고 그 명단.
//!
//! **범위는 `classId` 하나다.** 예전에는 학교 · 학년도 · 학년 · 반 네 값으로 "우리 반"을
//! 걸러냈다. 그 방식으로는 두 화면을 구별할 수 없다 — 같은 학생이 내 담임 반에도
//! 내 교과 강좌에도 있을 수 있어서, 학적으로 거르면 교과 화면에 담임 출결이 유출된다.
//! 그래서 담당 학급 · 강좌가 행(`teaching_class`)이 되고, 기록과 명단이 그 행을 가리킨다.
//!
//! 학교 단위 설정(최대 교시 · 제출 기한 · 휴업일)이 필요한 곳은 여기 있는
//! `homeroom_scope`로 학교를 얻는다. **같은 질의를 커맨드마다 따로 두지 않으려는 것이다** —
//! 나누어 두면 담임과 교과를 구별하는 조건이 파일마다 조금씩 달라진다.
//!
//! 명단도 여기가 답한다. 학년 · 반 · 번호는 그 학생의 **학적**이지 소속이 아니므로
//! 반으로 거르지 않는다. 반으로 거르면 반이 다른 전학생이 내 명단에서 조용히 빠지고,
//! 교과 강좌는 여러 반에서 모이므로 아예 담을 수 없다.

use crate::commands::with_conn;
use crate::due::parse_date;
use crate::state::{constraint_err, DbState};
use crate::types::{ClassTagItem, StudentItem, TeachingClassItem};
use rusqlite::Connection;
use tauri::State;

// ── 범위 ──────────────────────────────────────────────────────

/// 학급 하나가 가리키는 자리. 학교 · 학년도 · 역할이 전부 여기서 나온다.
///
/// **학년도는 학교를 거쳐 읽는다.** `teaching_class`가 학년도를 따로 들면 2027학년도
/// 학교에 속한 2026학년도 학급 같은 어긋난 행이 만들어질 수 있다.
pub(crate) struct ClassScope {
    pub id: i64,
    pub school_id: i64,
    pub year_id: i64,
    /// homeroom | subject. **명렬표의 열쇠가 이 값으로 구분된다** —
    /// 담임은 번호 하나, 교과는 학적 자리(학년 · 반 · 번호) 전체다.
    pub role: String,
    pub name: String,
    pub grade: Option<i64>,
    pub class_no: Option<i64>,
}

/// 역할을 가리지 않는 범위. **명단은 두 모드가 함께 쓰는 개념이다.**
///
/// `class_member`에는 역할이 없다 — 담임 학급이든 교과 강좌든 "누가 내 명단에 있는가"는
/// 같은 물음이고, 교과 강좌도 명렬표를 받아야 한다. 구분되는 것은 **기록**이지 명단이 아니다.
pub(crate) fn class_scope(conn: &Connection, class_id: i64) -> Result<ClassScope, String> {
    scope_of(conn, class_id, None)
}

/// 담임 커맨드가 쓰는 범위. **교과 강좌가 오면 거절한다.**
///
/// 스키마의 `trg_span_homeroom_only`가 같은 것을 막지만, 트리거는 쓰기에만 걸린다.
/// 읽기까지 막지 않으면 교과 `classId`로 담임 목록을 불러 빈 화면을 보여주게 되고,
/// 교사는 기록이 사라진 것인지 화면을 잘못 연 것인지 알 방법이 없다.
pub(crate) fn homeroom_scope(conn: &Connection, class_id: i64) -> Result<ClassScope, String> {
    scope_of(conn, class_id, Some("homeroom"))
}

/// 교과 커맨드가 쓰는 범위. **담임 학급이 오면 거절한다.** 위 담임 쪽과 짝이다.
///
/// 스키마의 `trg_session_subject_only`가 같은 것을 막지만 트리거는 쓰기에만 걸린다.
/// 읽기까지 막지 않으면 담임 `classId`로 차시 목록을 불러 빈 화면을 보여주게 된다.
pub(crate) fn subject_scope(conn: &Connection, class_id: i64) -> Result<ClassScope, String> {
    scope_of(conn, class_id, Some("subject"))
}

fn scope_of(
    conn: &Connection,
    class_id: i64,
    want: Option<&str>,
) -> Result<ClassScope, String> {
    let (school_id, year_id, role, name, grade, class_no) = conn
        .query_row(
            "SELECT c.school_id, sc.year_id, c.role, c.name, c.grade, c.class_no
               FROM teaching_class c
                        JOIN school sc ON sc.id = c.school_id
              WHERE c.id = ?1",
            rusqlite::params![class_id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("학급을 찾을 수 없습니다: {class_id}"),
            other => other.to_string(),
        })?;

    if let Some(want) = want {
        if role != want {
            return Err(match want {
                "homeroom" => format!("담임 학급이 아닙니다: {name}"),
                _ => format!("교과 강좌가 아닙니다: {name}"),
            });
        }
    }
    Ok(ClassScope {
        id: class_id,
        school_id,
        year_id,
        role,
        name,
        grade,
        class_no,
    })
}

/// 담임 명렬표가 학생을 배치할 학적 자리. **필드가 비공개인 것이 이 타입의 전부다.**
///
/// `UPDATE student SET enrolled_to`(자리 넘겨받기)는 되돌릴 수 없는 쓰기다. 교과 파일의
/// 반 오타 하나가 남의 반 학생을 전출시키면 안 되므로, 그 쓰기를 하는 함수는 이 값을
/// 받게 해 두었다. `if role == "subject"` 한 줄로 막으면 다음 사람이 그 줄을 지운다 —
/// **만들 수 없는 타입**으로 막는다.
pub(crate) struct HomeroomSeat {
    grade: i64,
    class_no: i64,
}

impl HomeroomSeat {
    pub(crate) fn at(&self) -> (i64, i64) {
        (self.grade, self.class_no)
    }
}

/// 담임 학급의 학적 자리. **`HomeroomSeat`을 만드는 유일한 곳이다.**
///
/// 교과 경로에서는 이 값을 얻을 수 없고, 그래서 이 값을 받는 함수를 부를 수도 없다.
/// 교과 강좌는 반이 섞여 학년 · 반이 비어 있는 것이 정상이고, 그쪽 명렬표는 파일이
/// 학년 · 반을 줄마다 들고 온다. 담임 학급이 비어 있는 것은 만들 때 빠뜨린 것이다.
pub(crate) fn homeroom_seat(scope: &ClassScope) -> Result<HomeroomSeat, String> {
    if scope.role != "homeroom" {
        return Err(format!("담임 학급이 아닙니다: {}", scope.name));
    }
    match (scope.grade, scope.class_no) {
        (Some(grade), Some(class_no)) => Ok(HomeroomSeat { grade, class_no }),
        _ => Err(format!(
            "{}의 학년 · 반이 설정되어 있지 않습니다. 설정에서 먼저 채워주세요.",
            scope.name
        )),
    }
}

// ── 명단 ──────────────────────────────────────────────────────

/// 그날 명단에 있던 학생. **두 기간을 함께 본다.**
///
/// 소속 기간(`class_member`)은 내 명단에 들어오고 나간 때이고, 재학 기간(`student`)은
/// 학교에 있고 없던 때다. 둘은 다른 사건이라 각각 닫힌다 — 내 반에서 빠졌다고
/// 학교를 떠난 것이 아니고, 그 반대도 마찬가지다.
///
/// 경계일은 닫는다(`?2 < left_on`). 같은 날 번호를 물려받는 경우 그 번호가 하루 동안
/// 두 학생으로 보이지 않게 하려는 것이고, 이 저장소의 `valid_to`와 같은 규칙이다.
const MEMBER_ON: &str = "FROM class_member m
                                  JOIN student st ON st.id = m.student_id
                         WHERE m.class_id = ?1
                           AND m.joined_on <= ?2 AND (m.left_on IS NULL OR ?2 < m.left_on)
                           AND st.enrolled_from <= ?2
                           AND (st.enrolled_to IS NULL OR ?2 < st.enrolled_to)";

/// 그날 명단의 (학생 id, 번호, 이름). 하루 격자가 이 목록으로 그려진다.
pub(crate) fn member_rows_on(
    conn: &Connection,
    class_id: i64,
    date: &str,
) -> Result<Vec<(i64, i64, String)>, String> {
    let sql = format!("SELECT st.id, st.number, st.name {MEMBER_ON} ORDER BY st.number, st.id");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![class_id, date], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 그날 명단의 (학생 id, 학년, 반, 번호, 이름). **학적 자리까지 준다.**
///
/// 교과 강좌는 여러 반에서 모이므로 번호만으로는 줄을 구별할 수 없다 — 3학년 1반 4번과
/// 3학년 6반 4번이 같은 강좌에 있다. 그래서 차시 명단은 자리 순으로 정렬한다.
pub(crate) fn member_seats_on(
    conn: &Connection,
    class_id: i64,
    date: &str,
) -> Result<Vec<(i64, i64, i64, i64, String)>, String> {
    let sql = format!(
        "SELECT st.id, st.grade, st.class_no, st.number, st.name {MEMBER_ON}
         ORDER BY st.grade, st.class_no, st.number, st.id"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![class_id, date], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 그날 명단의 인원. 개요 · 기록 · 미등재 화면이 말하는 "서른 명 중"이 이 수다.
///
/// **격자에 뜨는 줄 수와 반드시 같아야 한다.** 다르면 교사는 둘 중 어느 쪽이 맞는지
/// 알 방법이 없어, 조건을 위 상수 하나로 묶어 둔다.
pub(crate) fn member_count_on(conn: &Connection, class_id: i64, date: &str) -> Result<i64, String> {
    let sql = format!("SELECT COUNT(*) {MEMBER_ON}");
    conn.query_row(&sql, rusqlite::params![class_id, date], |r| r.get(0))
        .map_err(|e| e.to_string())
}

/// 그날 명단에서 번호로 학생을 찾는다. 나이스 파일 열기가 파일의 번호를 옮길 때 쓴다.
pub(crate) fn member_by_number_on(
    conn: &Connection,
    class_id: i64,
    number: i64,
    date: &str,
) -> Result<Option<(i64, String)>, String> {
    let sql = format!("SELECT st.id, st.name {MEMBER_ON} AND st.number = ?3 ORDER BY st.id LIMIT 1");
    conn.query_row(&sql, rusqlite::params![class_id, date, number], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other.to_string()),
    })
}

/// 지금 명단 전체. 날짜를 묻지 않는 화면(설정의 명단 편집)이 쓴다.
pub(crate) fn members_of(conn: &Connection, class_id: i64) -> Result<Vec<StudentItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT st.id, st.school_id, st.grade, st.class_no,
                    st.number, st.name, st.enrolled_from, st.enrolled_to
               FROM class_member m
                        JOIN student st ON st.id = m.student_id
              WHERE m.class_id = ?1 AND m.left_on IS NULL AND st.enrolled_to IS NULL
              ORDER BY st.number, st.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![class_id], |r| {
            Ok(StudentItem {
                id: r.get(0)?,
                school_id: r.get(1)?,
                grade: r.get(2)?,
                class_no: r.get(3)?,
                number: r.get(4)?,
                name: r.get(5)?,
                enrolled_from: r.get(6)?,
                enrolled_to: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

// ── 담당 학급 · 강좌 목록 ─────────────────────────────────────

fn map_class(row: &rusqlite::Row) -> rusqlite::Result<TeachingClassItem> {
    Ok(TeachingClassItem {
        id: row.get(0)?,
        school_id: row.get(1)?,
        role: row.get(2)?,
        name: row.get(3)?,
        grade: row.get(4)?,
        class_no: row.get(5)?,
        group_tag_id: row.get(6)?,
        group_tag_name: row.get(7)?,
        sort_order: row.get(8)?,
        valid_from: row.get(9)?,
        valid_to: row.get(10)?,
        member_count: row.get(11)?,
    })
}

const CLASS_SELECT: &str = "SELECT c.id, c.school_id, c.role, c.name, c.grade, c.class_no,
                                   c.group_tag_id, g.name,
                                   c.sort_order, c.valid_from, c.valid_to,
                                   (SELECT COUNT(*) FROM class_member m
                                     WHERE m.class_id = c.id
                                       AND m.left_on IS NULL)
                            FROM teaching_class c
                                     LEFT JOIN class_tag g ON g.id = c.group_tag_id";

/// 그 학교의 담당 학급 · 강좌. `role`을 주면 담임만 · 교과만 골라 온다.
///
/// **범위가 학교다.** 학년도는 학교가 들고 있으므로 학년도로 다시 거르지 않는다 —
/// 두 곳에서 거르면 조건이 어긋났을 때 어느 쪽이 맞는지 알 수 없다.
///
/// 마감된 줄(`valid_to`)은 빼고 돌려준다. 3월이 되면 지난해 줄을 마감하고 새로 넣으므로,
/// 마감된 것까지 목록에 나오면 학급 고르개에 작년 반이 함께 뜬다.
pub fn get_teaching_classes_impl(
    conn: &Connection,
    school_id: i64,
    role: Option<&str>,
) -> Result<Vec<TeachingClassItem>, String> {
    let mut sql = format!("{CLASS_SELECT} WHERE c.school_id = ?1 AND c.valid_to IS NULL");
    if role.is_some() {
        sql.push_str(" AND c.role = ?2");
    }
    sql.push_str(" ORDER BY c.sort_order, c.id");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = match role {
        Some(role) => stmt.query_map(rusqlite::params![school_id, role], map_class),
        None => stmt.query_map(rusqlite::params![school_id], map_class),
    }
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn next_class_order(conn: &Connection, school_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) + 10
           FROM teaching_class WHERE school_id = ?1 AND valid_to IS NULL",
        rusqlite::params![school_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

/// 묶음 이름표가 **그 학교의 것인가.** 없어도 된다(묶을 것이 없는 과목이 더 많다).
///
/// FK는 `class_tag`의 존재만 볼 뿐 어느 학교의 것인지는 보지 않는다. 남의 학교
/// 이름표를 단 강좌는 목록에서 이름 없는 묶음으로 보이고, 그 상태는 어떤 입력의
/// 결과도 아니다.
fn check_group_tag(
    conn: &Connection,
    school_id: i64,
    group_tag_id: Option<i64>,
) -> Result<(), String> {
    let Some(tag_id) = group_tag_id else {
        return Ok(());
    };
    let found: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM class_tag WHERE id = ?1 AND school_id = ?2",
            rusqlite::params![tag_id, school_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if found == 0 {
        return Err(format!("이 학교의 강좌 묶음이 아닙니다: {tag_id}"));
    }
    Ok(())
}

/// 역할이 요구하는 학적 자리를 확인한다.
///
/// 만들 때와 고칠 때가 같은 규칙이어야 하므로 한 곳에 둔다. 나누면 한쪽에만 조건이
/// 붙어, 설정 화면에서 고치는 길로만 학년 · 반이 빈 담임 학급이 생긴다.
fn check_role_seat(role: &str, grade: Option<i64>, class_no: Option<i64>) -> Result<(), String> {
    match role {
        "homeroom" => {
            if grade.is_none() || class_no.is_none() {
                return Err("담임 학급은 학년과 반이 필요합니다.".to_string());
            }
        }
        "subject" => {
            if grade.is_some() || class_no.is_some() {
                return Err("교과 강좌는 반이 섞이므로 학년 · 반을 두지 않습니다.".to_string());
            }
        }
        other => return Err(format!("알 수 없는 역할입니다: {other}")),
    }
    Ok(())
}

/// 이름 칸을 다듬고 비어 있으면 거절한다. 화면에 적을 이름이 없으면 고를 수도 없다.
fn check_class_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("학급 이름이 비어 있습니다.".to_string());
    }
    Ok(name)
}

/// 아직 마감하지 않은 학급의 학교와 역할. 마감된 줄은 목록에 나오지 않으므로 고칠 수도 없다.
fn live_class_seat(conn: &Connection, class_id: i64) -> Result<(i64, String), String> {
    conn.query_row(
        "SELECT school_id, role FROM teaching_class WHERE id = ?1 AND valid_to IS NULL",
        rusqlite::params![class_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            format!("유효한 학급을 찾을 수 없습니다: {class_id}")
        }
        other => other.to_string(),
    })
}

/// 담당 학급 · 강좌 하나를 만든다.
///
/// 담임 학급은 학년 · 반을 함께 받는다. 그 둘이 명렬표가 학생을 배치할 학적 자리이고,
/// 나중에 채우게 두면 명렬표를 가져오는 자리에서야 빠진 것을 알게 된다.
/// 교과 강좌는 반이 섞이므로 비워 둔다 — 채워 오면 거절한다.
///
/// **학년도를 받지 않는다.** 학교가 이미 안다.
#[allow(clippy::too_many_arguments)]
pub fn create_teaching_class_impl(
    conn: &Connection,
    school_id: i64,
    role: &str,
    name: &str,
    grade: Option<i64>,
    class_no: Option<i64>,
    group_tag_id: Option<i64>,
    valid_from: &str,
) -> Result<i64, String> {
    let name = check_class_name(name)?;
    check_role_seat(role, grade, class_no)?;
    check_group_tag(conn, school_id, group_tag_id)?;
    parse_date(valid_from)?;

    let sort_order = next_class_order(conn, school_id)?;
    conn.execute(
        "INSERT INTO teaching_class
           (school_id, role, name, grade, class_no, group_tag_id, sort_order, valid_from)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            school_id,
            role,
            name,
            grade,
            class_no,
            group_tag_id,
            sort_order,
            valid_from
        ],
    )
    .map_err(|e| constraint_err(&e, "이미 같은 학급이 있습니다."))?;
    Ok(conn.last_insert_rowid())
}

/// 담당 학급 · 강좌의 이름 · 학년 · 반을 고친다. **UPDATE이고, 마감 후 추가가 아니다.**
///
/// "수정은 마감 후 추가"는 코드 · 태그처럼 과거 기록이 **뜻으로** 가리키는 것에 붙는
/// 규칙이다. 학급 이름은 화면에 적히는 이름표일 뿐이고 기록은 `class_id`로 가리키므로,
/// 이름을 고쳐도 지난 출결이 가리키는 대상이 달라지지 않는다. 오히려 여기서 마감 후
/// 추가를 하면 지난 기록이 마감된 학급에 남아 화면에서 통째로 사라진다.
///
/// 역할은 고치지 않는다. 담임과 교과는 기록하는 것 자체가 달라, 역할을 바꾸는 것은
/// 이름표를 고치는 일이 아니라 다른 것을 맡는 일이다 — 새로 만들고 옛것을 마감한다.
pub fn update_teaching_class_impl(
    conn: &Connection,
    class_id: i64,
    name: &str,
    grade: Option<i64>,
    class_no: Option<i64>,
    group_tag_id: Option<i64>,
) -> Result<(), String> {
    let name = check_class_name(name)?;
    let (school_id, role) = live_class_seat(conn, class_id)?;
    check_role_seat(&role, grade, class_no)?;
    check_group_tag(conn, school_id, group_tag_id)?;

    conn.execute(
        "UPDATE teaching_class
            SET name = ?1, grade = ?2, class_no = ?3, group_tag_id = ?4
          WHERE id = ?5",
        rusqlite::params![name, grade, class_no, group_tag_id, class_id],
    )
    .map_err(|e| constraint_err(&e, "이미 같은 학급이 있습니다."))?;
    Ok(())
}

/// 담당 학급 · 강좌를 마감한다. **지우지 않는다.**
///
/// 지난 출결이 이 학급을 가리키고 있고 `absence_span.class_id`가 `ON DELETE CASCADE`라,
/// 행을 지우면 그 학급의 기록이 함께 사라진다. 3월에 지난해 학급을 정리하는 동작이
/// 지난해 출결을 지우는 동작이어서는 안 된다. 마감한 학급은 `get_teaching_classes`가
/// 빼고 돌려주므로 학급 고르개에도 나오지 않는다.
pub fn retire_teaching_class_impl(
    conn: &Connection,
    class_id: i64,
    valid_to: &str,
) -> Result<(), String> {
    parse_date(valid_to)?;
    let changed = conn
        .execute(
            "UPDATE teaching_class SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
            rusqlite::params![valid_to, class_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("유효한 학급을 찾을 수 없습니다: {class_id}"));
    }
    Ok(())
}

// ── 강좌 묶음 이름표 ──────────────────────────────────────────
//
// `프로그래밍A` · `프로그래밍B` · `프로그래밍C`를 묶는 이름이다. **교과 - 분반 2단
// 구조가 아니다** — 강좌는 저마다 `teaching_class` 한 행으로 독립해 있고, 이 이름표는
// 화면에서 함께 보이게 할 뿐이다. 2단으로 짜면 분반이 하나뿐인 과목에도 껍데기 행이
// 하나 더 생기고, 명단 · 차시 질의가 전부 한 단계를 더 거쳐야 한다.
//
// **`span_tag`와 다른 표다.** 그쪽은 한도를 세는 출결 태그이고 `quota_rule`이 가리킨다.
// 섞으면 `프로그래밍`이 통계 화면의 한도 규정 후보로 뜬다.

fn check_class_tag_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("강좌 묶음 이름이 비어 있습니다.".to_string());
    }
    Ok(name)
}

pub fn get_class_tags_impl(conn: &Connection, school_id: i64) -> Result<Vec<ClassTagItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.name, t.sort_order,
                    (SELECT COUNT(*) FROM teaching_class c
                      WHERE c.group_tag_id = t.id AND c.valid_to IS NULL)
               FROM class_tag t
              WHERE t.school_id = ?1
              ORDER BY t.sort_order, t.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![school_id], |r| {
            Ok(ClassTagItem {
                id: r.get(0)?,
                name: r.get(1)?,
                sort_order: r.get(2)?,
                class_count: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

pub fn create_class_tag_impl(
    conn: &Connection,
    school_id: i64,
    name: &str,
) -> Result<i64, String> {
    let name = check_class_tag_name(name)?;
    let sort_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 10 FROM class_tag WHERE school_id = ?1",
            rusqlite::params![school_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO class_tag (school_id, name, sort_order) VALUES (?1, ?2, ?3)",
        rusqlite::params![school_id, name, sort_order],
    )
    .map_err(|e| constraint_err(&e, "이미 있는 강좌 묶음입니다."))?;
    Ok(conn.last_insert_rowid())
}

/// 이름을 고친다. **UPDATE이고, 마감 후 추가가 아니다.**
///
/// "수정은 마감 후 추가"는 과거 기록이 **뜻으로** 가리키는 것(출결 코드 · 출결 태그)에
/// 붙는 규칙이다. 강좌 묶음은 화면에서 함께 보이게 하는 이름표일 뿐이고 세는 데 쓰지
/// 않으므로, 이름을 고쳐도 지난 기록의 뜻이 달라지지 않는다. 학급 이름과 같은 이유다.
pub fn rename_class_tag_impl(conn: &Connection, tag_id: i64, name: &str) -> Result<(), String> {
    let name = check_class_tag_name(name)?;
    let changed = conn
        .execute(
            "UPDATE class_tag SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, tag_id],
        )
        .map_err(|e| constraint_err(&e, "이미 있는 강좌 묶음입니다."))?;
    if changed == 0 {
        return Err(format!("강좌 묶음을 찾을 수 없습니다: {tag_id}"));
    }
    Ok(())
}

/// 묶음을 지운다. **강좌는 함께 지워지지 않는다** —
/// `teaching_class.group_tag_id`가 `ON DELETE SET NULL`이라 이름표만 풀린다.
pub fn delete_class_tag_impl(conn: &Connection, tag_id: i64) -> Result<(), String> {
    let removed = conn
        .execute("DELETE FROM class_tag WHERE id = ?1", rusqlite::params![tag_id])
        .map_err(|e| e.to_string())?;
    if removed == 0 {
        return Err(format!("강좌 묶음을 찾을 수 없습니다: {tag_id}"));
    }
    Ok(())
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_teaching_classes(
    db: State<DbState>,
    school_id: i64,
    role: Option<String>,
) -> Result<Vec<TeachingClassItem>, String> {
    with_conn(&db, |c| {
        get_teaching_classes_impl(c, school_id, role.as_deref())
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_teaching_class(
    db: State<DbState>,
    school_id: i64,
    role: String,
    name: String,
    grade: Option<i64>,
    class_no: Option<i64>,
    group_tag_id: Option<i64>,
    valid_from: String,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        create_teaching_class_impl(
            c,
            school_id,
            &role,
            &name,
            grade,
            class_no,
            group_tag_id,
            &valid_from,
        )
    })
}

#[tauri::command]
pub fn update_teaching_class(
    db: State<DbState>,
    class_id: i64,
    name: String,
    grade: Option<i64>,
    class_no: Option<i64>,
    group_tag_id: Option<i64>,
) -> Result<(), String> {
    with_conn(&db, |c| {
        update_teaching_class_impl(c, class_id, &name, grade, class_no, group_tag_id)
    })
}

#[tauri::command]
pub fn get_class_tags(db: State<DbState>, school_id: i64) -> Result<Vec<ClassTagItem>, String> {
    with_conn(&db, |c| get_class_tags_impl(c, school_id))
}

#[tauri::command]
pub fn create_class_tag(
    db: State<DbState>,
    school_id: i64,
    name: String,
) -> Result<i64, String> {
    with_conn(&db, |c| create_class_tag_impl(c, school_id, &name))
}

#[tauri::command]
pub fn rename_class_tag(db: State<DbState>, id: i64, name: String) -> Result<(), String> {
    with_conn(&db, |c| rename_class_tag_impl(c, id, &name))
}

#[tauri::command]
pub fn delete_class_tag(db: State<DbState>, id: i64) -> Result<(), String> {
    with_conn(&db, |c| delete_class_tag_impl(c, id))
}

#[tauri::command]
pub fn retire_teaching_class(
    db: State<DbState>,
    class_id: i64,
    valid_to: String,
) -> Result<(), String> {
    with_conn(&db, |c| retire_teaching_class_impl(c, class_id, &valid_to))
}
