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
//! 집중 등재의 저장도 여기 있다. 한 명분이 축 · 기간 · 사유 · 태그 · 등재 표시이고,
//! **그것을 한 트랜잭션으로 묶는 것이 이 화면의 전제**다 — 모달은 [저장]을 누르기
//! 전까지 아무것도 쓰지 않는다고 약속한다.
//!
//! `today`는 화면이 넘긴다. 커맨드 안에서 서버 시계를 읽지 않는다 — 자정 언저리나
//! 지난 날짜를 정리하는 중에 화면이 보는 날짜와 저장되는 날짜가 어긋나기 때문이다.

use super::attendance::{
    load_spans, max_slot_of, ranges_for, set_span_memo_impl, set_span_tag_impl,
};
use crate::commands::class::{homeroom_scope, member_count_on};
use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::{days_overdue, format_date, format_korean, parse_date};
use crate::state::{constraint_err, DbState};
use crate::types::{DayGroup, FocusEntry, SpanEdit, SpanItem};
use chrono::NaiveDate;
use rusqlite::{Connection, ToSql};
use std::collections::BTreeMap;
use tauri::State;

// ── 조건 조각 ─────────────────────────────────────────────────
//
// 별칭은 `load_spans`의 질의가 정한다 — 구간이 `s`, 학생이 `st`다.
// 매개변수 자리는 ?1 학급 · ?2 월 패턴으로 고정한다.

/// 두 목록이 공통으로 쓰는 학급 조건. **구간이 학급을 직접 가리킨다** —
/// 학적(학년 · 반)으로 거르면 교과 강좌 화면에 담임 출결이 샌다.
const CLASS_CLAUSE: &str = "s.class_id = ?1";

/// 아직 받지 않은 서류 · 아직 등재하지 않은 구간.
const DOC_UNDONE: &str = "s.doc_done = 0";
const NEIS_UNDONE: &str = "s.neis_done = 0";

/// 월 필터. 날짜가 ISO라 앞자리 비교로 그 달이 구분된다.
const MONTH_LIKE: &str = "s.date LIKE ?2";

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
    class_id: i64,
    date: &str,
    today: &str,
) -> Result<i64, String> {
    let date = iso(date)?;
    let today = iso(today)?;
    let scope = homeroom_scope(conn, class_id)?;
    with_transaction(conn, || {
        let changed = conn
            .execute(
                "UPDATE absence_span SET neis_done = 1, neis_done_on = ?1
                 WHERE neis_done = 0 AND date = ?2 AND class_id = ?3",
                rusqlite::params![today, date, scope.id],
            )
            .map_err(|e| e.to_string())?;
        Ok(changed as i64)
    })
}

// ── 집중 등재 저장 ────────────────────────────────────────────

/// 그 구간이 속한 학교와, 고른 종류가 물어야 하는 기간이 어느 쪽인지.
///
/// 종류는 **DB에서 읽는다** — 라벨 '결석'으로 비교하면 교사가 종류를 추가하는 순간 틀린다.
fn span_context(
    conn: &Connection,
    span_id: i64,
    type_id: Option<i64>,
) -> Result<(i64, Option<String>), String> {
    let class_id = conn
        .query_row(
            "SELECT class_id FROM absence_span WHERE id = ?1",
            rusqlite::params![span_id],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                format!("출결 기록을 찾을 수 없습니다: {span_id}")
            }
            other => other.to_string(),
        })?;
    let school_id = homeroom_scope(conn, class_id)?.school_id;
    let Some(id) = type_id else {
        return Ok((school_id, None));
    };
    let prompt = conn
        .query_row(
            "SELECT slot_prompt FROM attendance_type WHERE id = ?1",
            rusqlite::params![id],
            |r| r.get::<_, String>(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => format!("출결 종류를 찾을 수 없습니다: {id}"),
            other => other.to_string(),
        })?;
    Ok((school_id, Some(prompt)))
}

/// 두 축과 기간을 고친다. **트랜잭션을 열지 않는다.**
///
/// `attendance::edit_span_impl`과 같은 일을 하지만 그쪽은 스스로 `with_transaction`을
/// 연다. SQLite는 트랜잭션을 겹쳐 열 수 없어, 여러 건을 한 트랜잭션으로 묶는 이 경로에서는
/// 그대로 부를 수 없다. **`edit_span_impl`에서 트랜잭션을 분리하면 이 함수는 지운다.**
///
/// 고른 교시를 구간으로 바꾸는 판단은 그쪽과 **같은 `ranges_for`**를 쓴다. 그 규칙까지
/// 한 벌 더 두면 수정 모달로 고친 것과 이 화면으로 고친 것이 조용히 갈라진다.
///
/// 마감은 다시 계산하지 않는다 — `edit_span_impl`과 같다. 이미 교사가 학부모에게
/// 말해 둔 날짜라, 축을 고쳤다고 소급해 움직이면 그 약속이 달라진다.
fn edit_axis_in_tx(conn: &Connection, edit: &SpanEdit) -> Result<(), String> {
    let (school_id, prompt) = span_context(conn, edit.span_id, edit.type_id)?;
    let max_slot = max_slot_of(conn, school_id)?;
    let ranges = ranges_for(prompt.as_deref(), &edit.slots, max_slot as usize)?;

    // 한 행은 한 구간이다. 1,3,5처럼 이어지지 않은 교시는 담을 자리가 없다.
    if ranges.len() > 1 {
        return Err(
            "이어지지 않은 교시는 한 구간으로 고칠 수 없습니다. 지운 뒤 다시 찍어주세요."
                .to_string(),
        );
    }
    let (start, end) = ranges.into_iter().next().unwrap();

    let changed = conn
        .execute(
            "UPDATE absence_span
             SET reason_id = ?1, type_id = ?2, start_slot = ?3, end_slot = ?4
             WHERE id = ?5",
            rusqlite::params![
                edit.reason_id,
                edit.type_id,
                start.as_deref(),
                end.as_deref(),
                edit.span_id
            ],
        )
        .map_err(|e| constraint_err(&e, "이미 같은 구간이 있습니다."))?;
    if changed == 0 {
        return Err(format!("출결 기록을 찾을 수 없습니다: {}", edit.span_id));
    }
    Ok(())
}

/// 집중 등재에서 고친 것을 한꺼번에 저장한다. **한 트랜잭션이다.**
///
/// 모달은 [저장]을 누르기 전까지 아무것도 쓰지 않는다는 약속으로 만들어졌다.
/// 나이스 쪽 저장이 실패하는 날이 있어, 앱만 먼저 등재로 바뀌면 두 곳이 어긋난다.
/// 항목마다 커맨드를 따로 부르면 셋째에서 실패했을 때 앞의 둘은 이미 들어가고
/// 셋째만 반쯤 고쳐진 채 남아, 무엇이 저장됐는지 교사가 확인할 방법이 없다.
///
/// 어느 저장을 어떤 차례로 부르는지는 업무 규칙이므로 화면이 조립하지 않는다.
pub fn save_focus_entries_impl(
    conn: &Connection,
    entries: &[FocusEntry],
    today: &str,
) -> Result<(), String> {
    let today = iso(today)?;
    with_transaction(conn, || {
        for entry in entries {
            let span_id = entry.edit.span_id;
            edit_axis_in_tx(conn, &entry.edit)?;
            set_span_memo_impl(conn, span_id, &entry.memo)?;
            set_span_tag_impl(conn, span_id, entry.tag_id)?;
            set_neis_done_impl(conn, span_id, true, &today)?;
        }
        Ok(())
    })
}

// ── 미제출 목록 ───────────────────────────────────────────────

/// 서류 미제출 목록. 마감이 지난 것부터 보여준다.
///
/// `include_done`이 true면 이미 받은 건도 함께 돌려준다. 화면에서 체크한 줄이
/// 곧바로 사라지면 잘못 눌렀을 때 되돌릴 자리가 없기 때문이다.
pub fn get_doc_pending_impl(
    conn: &Connection,
    class_id: i64,
    year: Option<i64>,
    month: Option<i64>,
    include_done: bool,
    today: &str,
) -> Result<Vec<SpanItem>, String> {
    let today_date = parse_date(today)?;
    let today = format_date(today_date);
    let like = period_like(year, month)?;
    let scope = homeroom_scope(conn, class_id)?;

    let mut clauses: Vec<&str> = vec![CLASS_CLAUSE];
    let mut params: Vec<&dyn ToSql> = vec![&scope.id];
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
pub fn get_neis_pending_impl(
    conn: &Connection,
    class_id: i64,
    year: Option<i64>,
    month: Option<i64>,
    today: &str,
) -> Result<Vec<DayGroup>, String> {
    let today = iso(today)?;
    let like = period_like(year, month)?;
    let scope = homeroom_scope(conn, class_id)?;

    let mut clauses: Vec<&str> = vec![CLASS_CLAUSE, NEIS_UNDONE];
    let mut params: Vec<&dyn ToSql> = vec![&scope.id];
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
            enrolled: member_count_on(conn, scope.id, &date)?,
            date,
            spans: day_spans,
        });
    }
    Ok(groups)
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
    class_id: i64,
    date: String,
    today: String,
) -> Result<i64, String> {
    with_conn(&db, |c| mark_day_neis_impl(c, class_id, &date, &today))
}

#[tauri::command]
pub fn save_focus_entries(
    db: State<DbState>,
    entries: Vec<FocusEntry>,
    today: String,
) -> Result<(), String> {
    with_conn(&db, |c| save_focus_entries_impl(c, &entries, &today))
}

#[tauri::command]
pub fn get_doc_pending(
    db: State<DbState>,
    class_id: i64,
    year: Option<i64>,
    month: Option<i64>,
    include_done: bool,
    today: String,
) -> Result<Vec<SpanItem>, String> {
    with_conn(&db, |c| {
        get_doc_pending_impl(c, class_id, year, month, include_done, &today)
    })
}

#[tauri::command]
pub fn get_neis_pending(
    db: State<DbState>,
    class_id: i64,
    year: Option<i64>,
    month: Option<i64>,
    today: String,
) -> Result<Vec<DayGroup>, String> {
    with_conn(&db, |c| {
        get_neis_pending_impl(c, class_id, year, month, &today)
    })
}
