//! 학교 설정 — 최대 교시 · 서류 제출 기한 · 휴업일 · 태그 · 한도 규정.
//!
//! **학교 단위 값은 `app_config`에 넣지 않는다.** `app_config`는 학교가 하나뿐이라는
//! 가정이 들어간 전역 키-값이다. 최대 교시와 제출 기한은 학교마다 다르고, 순회 교사가
//! 학교를 둘 이상 등록하는 날이 와도 자리를 옮기지 않으려면 지금부터 `school` 행에 둔다.
//!
//! **이 모듈은 판정하지 않는다.** 한도 규정을 저장하고 조회할 뿐, 한도를 넘었다는
//! 이유로 출결 입력을 거부하지 않는다. 세는 일은 통계 화면이 열릴 때 따로 한다.
//!
//! **수정은 마감 후 추가다.** 태그와 한도 규정을 UPDATE로 고치면 그것을 가리키는 과거
//! 기록의 의미가 소급 변경된다. `valid_to`를 채워 마감하고 새 행을 넣는다.
//! 학교 행과 휴업일은 예외다 — 둘 다 과거 기록이 가리키는 대상이 아니고,
//! 계산된 마감일은 만들 때 `absence_span.doc_due`에 박아 두므로 설정을 바꿔도
//! 과거 기록의 마감이 함께 움직이지 않는다.

use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::parse_date;
use crate::state::{constraint_err, DbState};
use crate::types::{OffDayItem, QuotaRuleItem, SchoolItem, TagItem};
use rusqlite::Connection;
use tauri::State;

/// 시작일을 명시하지 않은 행의 `valid_from`.
///
/// 오늘 날짜를 넣으면 그 이전 날짜의 출결을 입력할 때 목록이 통째로 비어 버린다.
/// 최초 집합에 시작일이 없는 것은 `seed.sql`과 같은 이유다.
const NO_START_DATE: &str = "1900-01-01";

/// 하루의 마지막 교시가 가질 수 있는 범위.
/// 조회와 종례는 설정 대상이 아니라 언제나 하루의 양 끝이므로 여기 들어가지 않는다.
const MAX_SLOT_MIN: i64 = 1;
const MAX_SLOT_MAX: i64 = 9;

/// 한도 규정이 세는 기간과 단위. 값 자체는 DB의 CHECK에도 있고, 여기서는 저장 전에
/// 알 수 없는 값이 들어오는 것을 한국어 문장으로 막는다.
const PERIODS: &[&str] = &["year", "month"];
const UNITS: &[&str] = &["day", "count"];

/// 시작일이 비어 있으면 처음부터 유효한 것으로 본다.
fn valid_from_or_epoch(given: &str) -> Result<String, String> {
    let given = given.trim();
    if given.is_empty() {
        return Ok(NO_START_DATE.to_string());
    }
    parse_date(given)?;
    Ok(given.to_string())
}

// ── 학교 ──────────────────────────────────────────────────────

const SCHOOL_SELECT: &str = "SELECT id, year_id, name, max_slot, due_days, due_skip_offdays,
                                    sort_order, active
                             FROM school";

fn map_school(row: &rusqlite::Row) -> rusqlite::Result<SchoolItem> {
    Ok(SchoolItem {
        id: row.get(0)?,
        year_id: row.get(1)?,
        name: row.get(2)?,
        max_slot: row.get(3)?,
        due_days: row.get(4)?,
        due_skip_offdays: row.get(5)?,
        sort_order: row.get(6)?,
        active: row.get(7)?,
    })
}

/// 그 학년도의 학교들. **목록에서 내린 학교는 빼고 돌려준다.**
///
/// 학년도를 받는 것이 요점이다. 학교를 전역으로 두면 해가 바뀌어도 지난해 학교가
/// 목록에 남고, 그 학교의 최대 교시 · 제출 기한을 고치면 지난해 화면까지 소급해 바뀐다.
pub fn get_schools_impl(conn: &Connection, year_id: i64) -> Result<Vec<SchoolItem>, String> {
    let sql = format!("{SCHOOL_SELECT} WHERE year_id = ?1 AND active = 1 ORDER BY sort_order, id");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![year_id], map_school)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

pub fn get_school_impl(conn: &Connection, school_id: i64) -> Result<SchoolItem, String> {
    let sql = format!("{SCHOOL_SELECT} WHERE id = ?1");
    conn.query_row(&sql, rusqlite::params![school_id], map_school)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                format!("학교를 찾을 수 없습니다: {school_id}")
            }
            other => other.to_string(),
        })
}

/// 저장 전에 학교 설정을 확인한다. 만들 때와 고칠 때가 같은 규칙이어야 하므로
/// 한 곳에 둔다 — 나누면 새로 만드는 길로만 최대 교시 10이 들어온다.
fn validate_school(name: &str, max_slot: i64, due_days: i64) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("학교 이름이 비어 있습니다.".to_string());
    }
    if !(MAX_SLOT_MIN..=MAX_SLOT_MAX).contains(&max_slot) {
        return Err(format!(
            "최대 교시는 {MAX_SLOT_MIN}에서 {MAX_SLOT_MAX} 사이여야 합니다: {max_slot}"
        ));
    }
    if due_days < 0 {
        return Err(format!("서류 제출 기한은 0일 이상이어야 합니다: {due_days}"));
    }
    Ok(name)
}

fn next_school_order(conn: &Connection, year_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) + 10
           FROM school WHERE year_id = ?1 AND active = 1",
        rusqlite::params![year_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

/// 학교를 만들 때 함께 넣는 출결 태그. **세는 대상의 기본값이다.**
///
/// 학교마다 부르는 이름이 다르므로(`체험학습` / `학교장허가 체험학습`) 설정에서 고친다.
const DEFAULT_TAGS: &[(&str, i64)] = &[("체험학습", 10), ("생리통", 20)];

/// 함께 넣는 한도 규정. (규정 이름, 세는 태그, 기간, 한도, 단위, 순서)
///
/// 흔한 두 가지를 미리 넣는다. 숫자는 학교마다 다르므로 설정에서 고친다.
/// **이 규정은 입력을 막지 않는다** — 통계 화면에서 세어 알려줄 뿐이다.
const DEFAULT_RULES: &[(&str, &str, &str, i64, &str, i64)] = &[
    ("체험학습 연 20일", "체험학습", "year", 20, "day", 10),
    ("생리통 월 1회", "생리통", "month", 1, "count", 20),
];

/// 학교를 하나 만든다. 순회 교사는 학교를 둘 이상 맡는다.
///
/// **기본 태그와 한도 규정을 함께 넣는다.** 예전에는 `seed.sql`이 넣었는데, 시드는 한 번만
/// 도는 데 반해 학교는 여럿 만들어진다 — 첫 학교만 태그를 받고 둘째 학교는 빈 목록으로
/// 시작했다. 학교 단위 기본값은 학교를 만드는 자리에서 넣는 것이 맞다.
pub fn create_school_impl(
    conn: &Connection,
    year_id: i64,
    name: &str,
    max_slot: i64,
    due_days: i64,
    due_skip_offdays: bool,
) -> Result<i64, String> {
    let name = validate_school(name, max_slot, due_days)?;
    let sort_order = next_school_order(conn, year_id)?;

    with_transaction(conn, || {
        conn.execute(
            "INSERT INTO school (year_id, name, max_slot, due_days, due_skip_offdays, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![year_id, name, max_slot, due_days, due_skip_offdays, sort_order],
        )
        .map_err(|e| constraint_err(&e, &format!("이미 있는 학교입니다: {name}")))?;
        let school_id = conn.last_insert_rowid();
        seed_school_defaults(conn, school_id)?;
        Ok(school_id)
    })
}

/// 새 학교의 기본 태그와 한도 규정. 이름으로 방금 넣은 태그를 다시 찾는다 —
/// 규정이 태그를 가리키므로 순서가 강제된다.
fn seed_school_defaults(conn: &Connection, school_id: i64) -> Result<(), String> {
    for (name, order) in DEFAULT_TAGS {
        insert_tag(conn, school_id, name, *order, NO_START_DATE)?;
    }
    for (name, tag, period, limit_n, unit, order) in DEFAULT_RULES {
        let tag_id: i64 = conn
            .query_row(
                "SELECT id FROM span_tag WHERE school_id = ?1 AND name = ?2 AND valid_to IS NULL",
                rusqlite::params![school_id, tag],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO quota_rule
               (school_id, name, tag_id, period, limit_n, unit, sort_order, valid_from)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                school_id,
                name,
                tag_id,
                period,
                limit_n,
                unit,
                order,
                NO_START_DATE
            ],
        )
        .map_err(|e| constraint_err(&e, "이미 있는 한도 규정입니다."))?;
    }
    Ok(())
}

pub fn update_school_impl(
    conn: &Connection,
    school_id: i64,
    name: &str,
    max_slot: i64,
    due_days: i64,
    due_skip_offdays: bool,
) -> Result<(), String> {
    let name = validate_school(name, max_slot, due_days)?;
    let changed = conn
        .execute(
            "UPDATE school
                SET name = ?1, max_slot = ?2, due_days = ?3, due_skip_offdays = ?4
              WHERE id = ?5",
            rusqlite::params![name, max_slot, due_days, due_skip_offdays, school_id],
        )
        .map_err(|e| constraint_err(&e, &format!("이미 있는 학교입니다: {name}")))?;
    if changed == 0 {
        return Err(format!("학교를 찾을 수 없습니다: {school_id}"));
    }
    Ok(())
}

/// 학교를 목록에서 내린다. **지우지 않는다.**
///
/// 학생 · 맡은 것 · 출결이 이 행을 가리키고 있고 전부 `ON DELETE CASCADE`라, 행을
/// 지우면 그 학교의 기록이 통째로 사라진다. 잘못 만든 학교를 정리하는 동작이
/// 한 해치 출결을 지우는 동작이어서는 안 된다.
pub fn retire_school_impl(conn: &Connection, school_id: i64) -> Result<(), String> {
    let changed = conn
        .execute(
            "UPDATE school SET active = 0 WHERE id = ?1 AND active = 1",
            rusqlite::params![school_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("유효한 학교를 찾을 수 없습니다: {school_id}"));
    }
    Ok(())
}

// ── 휴업일 ────────────────────────────────────────────────────
//
// 학사일정 테이블이 아니다. 제출 기한을 셀 때 건너뛸 날짜 목록일 뿐이고,
// 교사가 설정 화면에서 직접 넣는다.

pub fn get_off_days_impl(
    conn: &Connection,
    school_id: i64,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<OffDayItem>, String> {
    let mut sql = String::from("SELECT id, date, label FROM off_day WHERE school_id = ?1");
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(school_id)];
    if let Some(from) = from {
        parse_date(from)?;
        params.push(Box::new(from.to_string()));
        sql.push_str(&format!(" AND date >= ?{}", params.len()));
    }
    if let Some(to) = to {
        parse_date(to)?;
        params.push(Box::new(to.to_string()));
        sql.push_str(&format!(" AND date <= ?{}", params.len()));
    }
    sql.push_str(" ORDER BY date, id");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
    let rows = stmt
        .query_map(refs.as_slice(), |r| {
            Ok(OffDayItem {
                id: r.get(0)?,
                date: r.get(1)?,
                label: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 같은 날짜를 다시 넣으면 이름만 갱신하고 기존 id를 돌려준다.
///
/// 교사가 같은 날을 두 번 등록하는 것은 실수가 아니라 이름을 고치는 동작이다.
/// 여기서 UNIQUE 위반으로 실패시키면 "이미 있습니다"를 보고 지웠다가 다시 넣어야 한다.
pub fn add_off_day_impl(
    conn: &Connection,
    school_id: i64,
    day: &OffDayItem,
) -> Result<i64, String> {
    parse_date(&day.date)?;
    with_transaction(conn, || {
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM off_day WHERE school_id = ?1 AND date = ?2",
                rusqlite::params![school_id, day.date],
                |r| r.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.to_string()),
            })?;

        if let Some(id) = existing {
            conn.execute(
                "UPDATE off_day SET label = ?1 WHERE id = ?2",
                rusqlite::params![day.label, id],
            )
            .map_err(|e| e.to_string())?;
            return Ok(id);
        }

        conn.execute(
            "INSERT INTO off_day (school_id, date, label) VALUES (?1, ?2, ?3)",
            rusqlite::params![school_id, day.date, day.label],
        )
        .map_err(|e| constraint_err(&e, "이미 등록된 휴업일입니다."))?;
        Ok(conn.last_insert_rowid())
    })
}

pub fn remove_off_day_impl(conn: &Connection, off_day_id: i64) -> Result<(), String> {
    let removed = conn
        .execute(
            "DELETE FROM off_day WHERE id = ?1",
            rusqlite::params![off_day_id],
        )
        .map_err(|e| e.to_string())?;
    if removed == 0 {
        return Err(format!("휴업일을 찾을 수 없습니다: {off_day_id}"));
    }
    Ok(())
}

// ── 태그 ──────────────────────────────────────────────────────
//
// 세는 대상이다. 한도 규정이 이 이름을 가리키므로 규정과 같은 곳(학교)에 둔다.

fn map_tag(row: &rusqlite::Row) -> rusqlite::Result<TagItem> {
    Ok(TagItem {
        id: row.get(0)?,
        name: row.get(1)?,
        sort_order: row.get(2)?,
        valid_from: row.get(3)?,
        valid_to: row.get(4)?,
    })
}

pub fn get_tags_impl(conn: &Connection, school_id: i64) -> Result<Vec<TagItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, sort_order, valid_from, valid_to
               FROM span_tag
              WHERE school_id = ?1 AND valid_to IS NULL
              ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![school_id], map_tag)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn next_tag_order(conn: &Connection, school_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) + 10
           FROM span_tag WHERE school_id = ?1 AND valid_to IS NULL",
        rusqlite::params![school_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

fn insert_tag(
    conn: &Connection,
    school_id: i64,
    name: &str,
    sort_order: i64,
    valid_from: &str,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO span_tag (school_id, name, sort_order, valid_from) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![school_id, name, sort_order, valid_from],
    )
    .map_err(|e| constraint_err(&e, "이미 있는 태그입니다."))?;
    Ok(conn.last_insert_rowid())
}

fn close_tag(conn: &Connection, tag_id: i64, valid_to: &str) -> Result<(), String> {
    let changed = conn
        .execute(
            "UPDATE span_tag SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
            rusqlite::params![valid_to, tag_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("유효한 태그를 찾을 수 없습니다: {tag_id}"));
    }
    Ok(())
}

pub fn create_tag_impl(conn: &Connection, school_id: i64, name: &str) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("태그 이름이 비어 있습니다.".to_string());
    }
    let sort_order = next_tag_order(conn, school_id)?;
    insert_tag(conn, school_id, name, sort_order, NO_START_DATE)
}

/// 이름 변경 = 옛 행 마감 + 새 행 추가. **UPDATE가 아니다.**
///
/// UPDATE로 고치면 그 태그를 가진 과거 구간의 이름까지 소급 변경된다.
/// 그 태그를 가리키던 한도 규정도 새 태그를 가리키는 새 행으로 옮긴다 —
/// 남겨두면 목록에서 사라진 태그를 가리키는 규정이 계속 유효한 것으로 조회된다.
///
/// **이름이 그대로면 아무것도 하지 않는다.** 마감 후 추가는 과거 구간이 가리키는
/// 태그 id를 새 것과 분리한다. 이름이 달라졌을 때는 그것이 목적이지만, 교사가 이름
/// 칸을 열었다가 그대로 저장한 경우까지 분리하면 얻는 것 없이 계보만 끊긴다.
pub fn rename_tag_impl(
    conn: &Connection,
    tag_id: i64,
    name: &str,
    today: &str,
) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("태그 이름이 비어 있습니다.".to_string());
    }
    parse_date(today)?;
    with_transaction(conn, || {
        let (school_id, sort_order, current): (i64, i64, String) = conn
            .query_row(
                "SELECT school_id, sort_order, name
                   FROM span_tag WHERE id = ?1 AND valid_to IS NULL",
                rusqlite::params![tag_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    format!("유효한 태그를 찾을 수 없습니다: {tag_id}")
                }
                other => other.to_string(),
            })?;

        if current == name {
            return Ok(tag_id);
        }

        close_tag(conn, tag_id, today)?;
        let new_id = insert_tag(conn, school_id, name, sort_order, today)?;
        move_rules_to_tag(conn, tag_id, Some(new_id), today)?;
        Ok(new_id)
    })
}

/// 마감한다. **삭제하지 않는다.** 그 태그를 가진 과거 구간이 그대로 남아야 한다.
pub fn retire_tag_impl(conn: &Connection, tag_id: i64, today: &str) -> Result<(), String> {
    parse_date(today)?;
    with_transaction(conn, || {
        close_tag(conn, tag_id, today)?;
        move_rules_to_tag(conn, tag_id, None, today)?;
        Ok(())
    })
}

// ── 한도 규정 ─────────────────────────────────────────────────
//
// "어떤 태그를 · 어느 기간에 · 몇 번까지"는 학교마다 해마다 바뀐다.
// 설정 값이 아니라 행이고, 고칠 때는 마감한 뒤 새 행을 넣는다.

const RULE_SELECT: &str = "SELECT q.id, q.name, q.tag_id, t.name, q.reason_id, q.type_id,
                                  q.period, q.limit_n, q.unit, q.sort_order,
                                  q.valid_from, q.valid_to
                             FROM quota_rule q
                             LEFT JOIN span_tag t ON t.id = q.tag_id";

fn map_rule(row: &rusqlite::Row) -> rusqlite::Result<QuotaRuleItem> {
    Ok(QuotaRuleItem {
        id: row.get(0)?,
        name: row.get(1)?,
        tag_id: row.get(2)?,
        tag_name: row.get(3)?,
        reason_id: row.get(4)?,
        type_id: row.get(5)?,
        period: row.get(6)?,
        limit_n: row.get(7)?,
        unit: row.get(8)?,
        sort_order: row.get(9)?,
        valid_from: row.get(10)?,
        valid_to: row.get(11)?,
    })
}

fn validate_rule(rule: &QuotaRuleItem) -> Result<(), String> {
    if rule.name.trim().is_empty() {
        return Err("한도 규정 이름이 비어 있습니다.".to_string());
    }
    if !PERIODS.contains(&rule.period.as_str()) {
        return Err(format!(
            "한도를 세는 기간이 올바르지 않습니다: {} (year / month)",
            rule.period
        ));
    }
    if !UNITS.contains(&rule.unit.as_str()) {
        return Err(format!(
            "한도를 세는 단위가 올바르지 않습니다: {} (day / count)",
            rule.unit
        ));
    }
    if rule.limit_n <= 0 {
        return Err(format!("한도는 1 이상이어야 합니다: {}", rule.limit_n));
    }
    Ok(())
}

/// 규정이 가리키는 태그가 **그 학교의 유효한 태그인가.**
///
/// `retire_tag`가 그 태그를 세던 규정을 함께 마감하는 것과 같은 이유다 — 목록에서
/// 사라진 태그를 가리키는 규정이 유효한 것으로 조회되면, 교사에게는 보이지 않는
/// 것을 세는 규정이 설정 화면에 남는다. 마감을 막아 놓고 생성·수정으로 다시 만들
/// 수 있으면 막은 것이 아니다.
///
/// 다른 학교의 태그도 같이 막는다. 태그와 규정은 같은 학교에 속해 있어야 하는데,
/// FK는 span_tag의 존재만 볼 뿐 어느 학교의 것인지는 보지 않는다.
///
/// 태그가 비어 있는 규정은 그대로 통과한다 — 두 축만으로 세는 규정이 있을 수 있다.
fn check_rule_tag(conn: &Connection, school_id: i64, tag_id: Option<i64>) -> Result<(), String> {
    let Some(tag_id) = tag_id else {
        return Ok(());
    };
    let live: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM span_tag
              WHERE id = ?1 AND school_id = ?2 AND valid_to IS NULL",
            rusqlite::params![tag_id, school_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if live == 0 {
        return Err(format!("이 학교의 유효한 태그가 아닙니다: {tag_id}"));
    }
    Ok(())
}

fn rule_school(conn: &Connection, rule_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT school_id FROM quota_rule WHERE id = ?1",
        rusqlite::params![rule_id],
        |r| r.get(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            format!("한도 규정을 찾을 수 없습니다: {rule_id}")
        }
        other => other.to_string(),
    })
}

fn insert_rule(
    conn: &Connection,
    school_id: i64,
    rule: &QuotaRuleItem,
    valid_from: &str,
) -> Result<i64, String> {
    validate_rule(rule)?;
    check_rule_tag(conn, school_id, rule.tag_id)?;
    conn.execute(
        "INSERT INTO quota_rule
           (school_id, name, tag_id, reason_id, type_id, period, limit_n, unit, sort_order, valid_from)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            school_id,
            rule.name.trim(),
            rule.tag_id,
            rule.reason_id,
            rule.type_id,
            rule.period,
            rule.limit_n,
            rule.unit,
            rule.sort_order,
            valid_from
        ],
    )
    .map_err(|e| constraint_err(&e, "이미 있는 한도 규정입니다."))?;
    Ok(conn.last_insert_rowid())
}

fn close_rule(conn: &Connection, rule_id: i64, valid_to: &str) -> Result<(), String> {
    let changed = conn
        .execute(
            "UPDATE quota_rule SET valid_to = ?1 WHERE id = ?2 AND valid_to IS NULL",
            rusqlite::params![valid_to, rule_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("유효한 한도 규정을 찾을 수 없습니다: {rule_id}"));
    }
    Ok(())
}

/// 태그가 이름을 바꾸거나 마감될 때 그 태그를 가리키던 규정을 함께 옮긴다.
///
/// `new_tag_id`가 없으면(마감) 규정도 함께 마감한다. 규정 역시 UPDATE로 고치지 않고
/// 옛 행을 마감한 뒤 새 행을 넣는다.
fn move_rules_to_tag(
    conn: &Connection,
    old_tag_id: i64,
    new_tag_id: Option<i64>,
    today: &str,
) -> Result<(), String> {
    let sql = format!(
        "{RULE_SELECT} WHERE q.tag_id = ?1 AND q.valid_to IS NULL ORDER BY q.sort_order, q.id"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rules = stmt
        .query_map(rusqlite::params![old_tag_id], map_rule)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    for rule in rules {
        let school_id = rule_school(conn, rule.id)?;
        close_rule(conn, rule.id, today)?;
        if let Some(new_tag_id) = new_tag_id {
            let mut moved = rule.clone();
            moved.tag_id = Some(new_tag_id);
            insert_rule(conn, school_id, &moved, today)?;
        }
    }
    Ok(())
}

pub fn get_quota_rules_impl(
    conn: &Connection,
    school_id: i64,
) -> Result<Vec<QuotaRuleItem>, String> {
    let sql = format!(
        "{RULE_SELECT} WHERE q.school_id = ?1 AND q.valid_to IS NULL ORDER BY q.sort_order, q.id"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![school_id], map_rule)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

pub fn create_quota_rule_impl(
    conn: &Connection,
    school_id: i64,
    rule: &QuotaRuleItem,
) -> Result<i64, String> {
    validate_rule(rule)?;
    let valid_from = valid_from_or_epoch(&rule.valid_from)?;
    let mut row = rule.clone();
    if row.sort_order == 0 {
        row.sort_order = next_rule_order(conn, school_id)?;
    }
    insert_rule(conn, school_id, &row, &valid_from)
}

fn next_rule_order(conn: &Connection, school_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(sort_order), 0) + 10
           FROM quota_rule WHERE school_id = ?1 AND valid_to IS NULL",
        rusqlite::params![school_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

/// 규정 "수정" = 옛 행 마감 + 새 행 추가. 지난 학년도의 규정이 그대로 남는다.
pub fn revise_quota_rule_impl(
    conn: &Connection,
    rule: &QuotaRuleItem,
    today: &str,
) -> Result<i64, String> {
    validate_rule(rule)?;
    parse_date(today)?;
    with_transaction(conn, || {
        let school_id = rule_school(conn, rule.id)?;
        close_rule(conn, rule.id, today)?;
        insert_rule(conn, school_id, rule, today)
    })
}

pub fn retire_quota_rule_impl(conn: &Connection, rule_id: i64, today: &str) -> Result<(), String> {
    parse_date(today)?;
    close_rule(conn, rule_id, today)
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_schools(db: State<DbState>, year_id: i64) -> Result<Vec<SchoolItem>, String> {
    with_conn(&db, |c| get_schools_impl(c, year_id))
}

#[tauri::command]
pub fn get_school(db: State<DbState>, school_id: i64) -> Result<SchoolItem, String> {
    with_conn(&db, |c| get_school_impl(c, school_id))
}

#[tauri::command]
pub fn create_school(
    db: State<DbState>,
    year_id: i64,
    name: String,
    max_slot: i64,
    due_days: i64,
    due_skip_offdays: bool,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        create_school_impl(c, year_id, &name, max_slot, due_days, due_skip_offdays)
    })
}

#[tauri::command]
pub fn update_school(
    db: State<DbState>,
    school_id: i64,
    name: String,
    max_slot: i64,
    due_days: i64,
    due_skip_offdays: bool,
) -> Result<(), String> {
    with_conn(&db, |c| {
        update_school_impl(c, school_id, &name, max_slot, due_days, due_skip_offdays)
    })
}

#[tauri::command]
pub fn retire_school(db: State<DbState>, school_id: i64) -> Result<(), String> {
    with_conn(&db, |c| retire_school_impl(c, school_id))
}

#[tauri::command]
pub fn get_off_days(
    db: State<DbState>,
    school_id: i64,
    from: Option<String>,
    to: Option<String>,
) -> Result<Vec<OffDayItem>, String> {
    with_conn(&db, |c| {
        get_off_days_impl(c, school_id, from.as_deref(), to.as_deref())
    })
}

#[tauri::command]
pub fn add_off_day(db: State<DbState>, school_id: i64, day: OffDayItem) -> Result<i64, String> {
    with_conn(&db, |c| add_off_day_impl(c, school_id, &day))
}

#[tauri::command]
pub fn remove_off_day(db: State<DbState>, off_day_id: i64) -> Result<(), String> {
    with_conn(&db, |c| remove_off_day_impl(c, off_day_id))
}

#[tauri::command]
pub fn get_tags(db: State<DbState>, school_id: i64) -> Result<Vec<TagItem>, String> {
    with_conn(&db, |c| get_tags_impl(c, school_id))
}

#[tauri::command]
pub fn create_tag(db: State<DbState>, school_id: i64, name: String) -> Result<i64, String> {
    with_conn(&db, |c| create_tag_impl(c, school_id, &name))
}

#[tauri::command]
pub fn rename_tag(
    db: State<DbState>,
    tag_id: i64,
    name: String,
    today: String,
) -> Result<i64, String> {
    with_conn(&db, |c| rename_tag_impl(c, tag_id, &name, &today))
}

#[tauri::command]
pub fn retire_tag(db: State<DbState>, tag_id: i64, today: String) -> Result<(), String> {
    with_conn(&db, |c| retire_tag_impl(c, tag_id, &today))
}

#[tauri::command]
pub fn get_quota_rules(
    db: State<DbState>,
    school_id: i64,
) -> Result<Vec<QuotaRuleItem>, String> {
    with_conn(&db, |c| get_quota_rules_impl(c, school_id))
}

#[tauri::command]
pub fn create_quota_rule(
    db: State<DbState>,
    school_id: i64,
    rule: QuotaRuleItem,
) -> Result<i64, String> {
    with_conn(&db, |c| create_quota_rule_impl(c, school_id, &rule))
}

#[tauri::command]
pub fn revise_quota_rule(
    db: State<DbState>,
    rule: QuotaRuleItem,
    today: String,
) -> Result<i64, String> {
    with_conn(&db, |c| revise_quota_rule_impl(c, &rule, &today))
}

#[tauri::command]
pub fn retire_quota_rule(
    db: State<DbState>,
    rule_id: i64,
    today: String,
) -> Result<(), String> {
    with_conn(&db, |c| retire_quota_rule_impl(c, rule_id, &today))
}
