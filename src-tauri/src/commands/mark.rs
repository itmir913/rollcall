//! 서류와 나이스 표시, 그리고 두 미제출 목록.
//!
//! 출결 한 건에 대해 앱이 아는 것은 **증빙을 받았는가 · 나이스에 넣었는가 · 메모** 셋뿐이다.
//! 서류 '종류'는 기록하지 않는다 — 학부모확인서냐 의사소견서냐는 학교 규정이고 해마다
//! 바뀐다. 무엇을 받기로 했는지는 메모에 적고, 앱은 그 문장을 해석하지 않는다.
//!
//! 두 표시는 성격이 다르고, 그래서 목록의 모양도 다르다.
//!   · 서류는 **건별**이다. 학생마다 받아 오는 시점이 다르므로 마감이 지난 것부터 본다.
//!   · 나이스는 **하루씩** 입력한다. 그래서 날짜로 묶고 오래된 날부터 위에서 아래로 옮겨 적는다.
//!
//! 구간 목록은 `attendance::load_spans`가 만든다. 같은 `SpanItem`을 두 번 만들면
//! 겹침 표시나 마감 계산이 화면마다 갈라진다. 조건과 별칭은 아래 상수 한 묶음에
//! 모아 두었다 — 그쪽 질의가 별칭을 바꾸면 고칠 자리가 거기뿐이다.
//!
//! `today`는 화면이 넘긴다. 커맨드 안에서 서버 시계를 읽지 않는다 — 자정 언저리나
//! 지난 날짜를 정리하는 중에 화면이 보는 날짜와 저장되는 날짜가 어긋나기 때문이다.

use super::attendance::load_spans;
use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::{days_overdue, format_date, format_korean, parse_date};
use crate::state::DbState;
use crate::types::{DayGroup, SpanItem};
use chrono::NaiveDate;
use rusqlite::{Connection, ToSql};
use std::collections::BTreeMap;
use tauri::State;

// ── 조건 조각 ─────────────────────────────────────────────────
//
// 별칭은 `load_spans`의 질의가 정한다 — 구간이 `s`, 학생이 `st`다.
// 매개변수 자리는 ?1 학교 · ?2 학년도 · ?3 학년 · ?4 반 · ?5 월 패턴으로 고정한다.

/// 두 목록이 공통으로 쓰는 학급 조건.
const CLASS_CLAUSES: [&str; 4] = [
    "st.school_id = ?1",
    "st.year_id = ?2",
    "st.grade = ?3",
    "st.class_no = ?4",
];

/// 아직 받지 않은 서류 · 아직 등재하지 않은 구간.
const DOC_UNDONE: &str = "s.doc_done = 0";
const NEIS_UNDONE: &str = "s.neis_done = 0";

/// 월 필터. 날짜가 ISO라 앞자리 비교로 그 달이 구분된다.
const MONTH_LIKE: &str = "s.date LIKE ?5";

/// 목록의 기본 순서. 서류는 이 뒤에 마감 순으로 다시 세운다.
const SPAN_ORDER: &str = "ORDER BY s.date, st.number, s.id";

/// 월 필터를 날짜 앞자리 패턴으로 바꾼다. 연도만 주면 그 해 전체다.
///
/// 달만 넘어오면 거절한다. 어느 해의 그 달인지 앱이 고르면, 학년도가 걸친 1·2월에
/// 교사가 고른 것과 다른 달이 나온다.
fn period_like(year: Option<i64>, month: Option<i64>) -> Result<Option<String>, String> {
    match (year, month) {
        (Some(y), Some(m)) => {
            if !(1..=12).contains(&m) {
                return Err(format!("달은 1에서 12 사이여야 합니다: {m}"));
            }
            Ok(Some(format!("{y:04}-{m:02}%")))
        }
        (Some(y), None) => Ok(Some(format!("{y:04}%"))),
        (None, Some(_)) => Err("달로 거르려면 연도가 함께 필요합니다.".to_string()),
        (None, None) => Ok(None),
    }
}

/// 날짜를 ISO로 맞춘다.
///
/// 파싱은 `2026-9-11`처럼 자리를 채우지 않은 표기도 받아 준다. 그대로 저장하면
/// 날짜 비교가 문자열 비교인 자리에서 `2026-09-01`보다 앞선 것으로 읽힌다.
fn iso(date: &str) -> Result<String, String> {
    Ok(format_date(parse_date(date)?))
}

/// 마감 경과일. 마감일이 없으면 None이다.
///
/// `SpanItem.days_overdue`는 이미 받은 건에서 비어 있다. 정렬에는 그 값을 쓰지 않고
/// `doc_due`로 다시 센다 — 교사가 체크한 줄이 그 자리에 남아야 하기 때문이다.
/// 체크하는 순간 줄이 목록 맨 아래로 뛰면 다음에 누를 줄을 다시 찾아야 한다.
fn overdue_of(span: &SpanItem, today: NaiveDate) -> Option<i64> {
    span.doc_due
        .as_deref()
        .and_then(|d| parse_date(d).ok())
        .map(|d| days_overdue(d, today))
}

// ── 표시 ──────────────────────────────────────────────────────

/// 증빙 서류를 받았는지 표시한다. 받은 날짜는 화면이 넘긴 `today`로 적는다.
pub fn set_doc_done_impl(
    conn: &Connection,
    span_id: i64,
    done: bool,
    today: &str,
) -> Result<(), String> {
    let today = iso(today)?;
    let done_on = if done { Some(today) } else { None };
    let changed = conn
        .execute(
            "UPDATE absence_span SET doc_done = ?1, doc_done_on = ?2 WHERE id = ?3",
            rusqlite::params![done as i64, done_on, span_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("출결 기록을 찾을 수 없습니다: {span_id}"));
    }
    Ok(())
}

/// 나이스에 등재했는지 표시한다.
pub fn set_neis_done_impl(
    conn: &Connection,
    span_id: i64,
    done: bool,
    today: &str,
) -> Result<(), String> {
    let today = iso(today)?;
    let done_on = if done { Some(today) } else { None };
    let changed = conn
        .execute(
            "UPDATE absence_span SET neis_done = ?1, neis_done_on = ?2 WHERE id = ?3",
            rusqlite::params![done as i64, done_on, span_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("출결 기록을 찾을 수 없습니다: {span_id}"));
    }
    Ok(())
}

/// 그날 아직 등재하지 않은 구간을 한 번에 표시하고 건수를 돌려준다.
///
/// 나이스는 하루치를 한 화면에서 입력하므로, 옮겨 적고 나면 그날 전체가 끝난다.
/// 이미 표시된 건은 건드리지 않는다 — 등재한 날짜(`neis_done_on`)를 오늘로 덮으면
/// 언제 넣었는지가 사라진다.
///
/// **미완성 기록도 함께 표시한다.** 프로그램은 판정하지 않는다. 어느 건을 빼야
/// 하는지는 교사가 목록을 보고 개별로 해제한다.
pub fn mark_day_neis_impl(
    conn: &Connection,
    school_id: i64,
    year_id: i64,
    grade: i64,
    class_no: i64,
    date: &str,
    today: &str,
) -> Result<i64, String> {
    let date = iso(date)?;
    let today = iso(today)?;
    with_transaction(conn, || {
        let changed = conn
            .execute(
                "UPDATE absence_span SET neis_done = 1, neis_done_on = ?1
                 WHERE neis_done = 0 AND date = ?2
                   AND student_id IN (SELECT id FROM student
                                       WHERE school_id = ?3 AND year_id = ?4
                                         AND grade = ?5 AND class_no = ?6)",
                rusqlite::params![today, date, school_id, year_id, grade, class_no],
            )
            .map_err(|e| e.to_string())?;
        Ok(changed as i64)
    })
}

// ── 미제출 목록 ───────────────────────────────────────────────

/// 서류 미제출 목록. 마감이 지난 것부터 보여준다.
///
/// `include_done`이 true면 이미 받은 건도 함께 돌려준다. 화면에서 체크한 줄이
/// 곧바로 사라지면 잘못 눌렀을 때 되돌릴 자리가 없기 때문이다.
#[allow(clippy::too_many_arguments)]
pub fn get_doc_pending_impl(
    conn: &Connection,
    school_id: i64,
    year_id: i64,
    grade: i64,
    class_no: i64,
    year: Option<i64>,
    month: Option<i64>,
    include_done: bool,
    today: &str,
) -> Result<Vec<SpanItem>, String> {
    let today_date = parse_date(today)?;
    let today = format_date(today_date);
    let like = period_like(year, month)?;

    let mut clauses: Vec<&str> = CLASS_CLAUSES.to_vec();
    let mut params: Vec<&dyn ToSql> = vec![&school_id, &year_id, &grade, &class_no];
    if !include_done {
        clauses.push(DOC_UNDONE);
    }
    if let Some(pattern) = &like {
        clauses.push(MONTH_LIKE);
        params.push(pattern);
    }
    let where_sql = format!("WHERE {} {SPAN_ORDER}", clauses.join(" AND "));

    let mut spans = load_spans(conn, &where_sql, &params, &today)?;

    // 마감이 지난 것이 위다. 마감이 없는 건은 맨 아래로 보낸다 — 재촉할 근거가 없다.
    spans.sort_by(|a, b| {
        let left = overdue_of(a, today_date).unwrap_or(i64::MIN);
        let right = overdue_of(b, today_date).unwrap_or(i64::MIN);
        right
            .cmp(&left)
            .then(a.number.cmp(&b.number))
            .then(a.date.cmp(&b.date))
            .then(a.id.cmp(&b.id))
    });
    Ok(spans)
}

/// 나이스 미등재 목록. 날짜로 묶고 **오래된 날부터** 돌려준다.
///
/// 나이스는 하루씩 입력하므로 위에서 아래로 그대로 옮겨 적는 순서가 된다.
/// 서류와 달리 `include_done`이 없다 — 등재는 하루 단위로 끝내는 일이라,
/// 끝난 날은 목록에서 빠지는 편이 남은 날을 세기 쉽다.
#[allow(clippy::too_many_arguments)]
pub fn get_neis_pending_impl(
    conn: &Connection,
    school_id: i64,
    year_id: i64,
    grade: i64,
    class_no: i64,
    year: Option<i64>,
    month: Option<i64>,
    today: &str,
) -> Result<Vec<DayGroup>, String> {
    let today = iso(today)?;
    let like = period_like(year, month)?;

    let mut clauses: Vec<&str> = CLASS_CLAUSES.to_vec();
    clauses.push(NEIS_UNDONE);
    let mut params: Vec<&dyn ToSql> = vec![&school_id, &year_id, &grade, &class_no];
    if let Some(pattern) = &like {
        clauses.push(MONTH_LIKE);
        params.push(pattern);
    }
    let where_sql = format!("WHERE {} {SPAN_ORDER}", clauses.join(" AND "));

    let spans = load_spans(conn, &where_sql, &params, &today)?;

    // BTreeMap이 날짜 오름차순을 보장한다. 날짜가 ISO라 문자열 순서가 곧 날짜 순서다.
    let mut by_date: BTreeMap<String, Vec<SpanItem>> = BTreeMap::new();
    for span in spans {
        by_date.entry(span.date.clone()).or_default().push(span);
    }

    let mut groups = Vec::with_capacity(by_date.len());
    for (date, day_spans) in by_date {
        groups.push(DayGroup {
            date_label: format_korean(parse_date(&date)?),
            enrolled: enrolled_on(conn, school_id, year_id, grade, class_no, &date)?,
            date,
            spans: day_spans,
        });
    }
    Ok(groups)
}

/// 그날 재학 중이던 학생 수. 전출한 학생의 기록은 목록에 남지만 머릿수에서는 빠진다.
///
/// 조건은 `attendance.rs`의 `enrolled_count`와 같다. 그쪽이 비공개라 한 벌 더 두었다 —
/// 재학 판정이 화면마다 갈라지면 같은 날 머릿수가 화면마다 달라진다.
fn enrolled_on(
    conn: &Connection,
    school_id: i64,
    year_id: i64,
    grade: i64,
    class_no: i64,
    date: &str,
) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM student
         WHERE school_id = ?1 AND year_id = ?2 AND grade = ?3 AND class_no = ?4
           AND enrolled_from <= ?5 AND (enrolled_to IS NULL OR enrolled_to > ?5)",
        rusqlite::params![school_id, year_id, grade, class_no, date],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn set_doc_done(
    db: State<DbState>,
    span_id: i64,
    done: bool,
    today: String,
) -> Result<(), String> {
    with_conn(&db, |c| set_doc_done_impl(c, span_id, done, &today))
}

#[tauri::command]
pub fn set_neis_done(
    db: State<DbState>,
    span_id: i64,
    done: bool,
    today: String,
) -> Result<(), String> {
    with_conn(&db, |c| set_neis_done_impl(c, span_id, done, &today))
}

#[tauri::command]
pub fn mark_day_neis(
    db: State<DbState>,
    school_id: i64,
    year_id: i64,
    grade: i64,
    class_no: i64,
    date: String,
    today: String,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        mark_day_neis_impl(c, school_id, year_id, grade, class_no, &date, &today)
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn get_doc_pending(
    db: State<DbState>,
    school_id: i64,
    year_id: i64,
    grade: i64,
    class_no: i64,
    year: Option<i64>,
    month: Option<i64>,
    include_done: bool,
    today: String,
) -> Result<Vec<SpanItem>, String> {
    with_conn(&db, |c| {
        get_doc_pending_impl(
            c,
            school_id,
            year_id,
            grade,
            class_no,
            year,
            month,
            include_done,
            &today,
        )
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn get_neis_pending(
    db: State<DbState>,
    school_id: i64,
    year_id: i64,
    grade: i64,
    class_no: i64,
    year: Option<i64>,
    month: Option<i64>,
    today: String,
) -> Result<Vec<DayGroup>, String> {
    with_conn(&db, |c| {
        get_neis_pending_impl(c, school_id, year_id, grade, class_no, year, month, &today)
    })
}
