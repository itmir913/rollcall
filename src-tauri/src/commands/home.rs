//! 개요 화면이 한 번에 받아 가는 요약.
//!
//! 개요는 **밀린 일을 보는 곳**이다. 오늘 얼마나 입력했는지와, 아직 안 받은 서류·
//! 아직 나이스에 안 넣은 기록을 함께 보여준다. 화면이 커맨드 여러 개를 조합하면
//! 그 조합 규칙이 프런트로 유출되므로 커맨드 하나로 묶는다.
//!
//! 세는 기준 두 가지가 이 모듈의 요점이다.
//!
//! · `recorded`는 **학생 수**다. 구간 수가 아니다. 하루에 두 구간을 가진 학생이
//!   두 명으로 세지면 "서른 명 중 서른두 명 입력"이라는 문장이 나온다.
//! · `doc_pending`은 **안 받은 것 전부**다. 기한이 지난 것만 세지 않는다 —
//!   기한 뒤에 재촉하는 것은 이미 늦은 일이라, 여유가 있을 때 보이는 쪽이 서류를
//!   실제로 받게 한다. 급한 것은 `doc_overdue`가 따로 센다.
//!
//! 목록을 선택하고 정렬하는 규칙은 여기에 두지 않는다. `mark`의 두 함수를 그대로 부른다 —
//! 개요에 보이는 몇 줄은 미제출자 목록의 맨 위 몇 줄과 같아야 하고, 정렬을 따로
//! 구현하면 두 화면의 첫 줄이 서로 다른 학생이 된다.

use crate::commands::attendance::load_spans;
use crate::commands::class::{homeroom_scope, member_count_on};
use crate::commands::mark::{get_doc_pending_impl, get_neis_pending_impl};
use crate::commands::with_conn;
use crate::due::{format_date, format_korean, parse_date};
use crate::state::DbState;
use crate::types::{HomeSummary, SpanItem};
use rusqlite::{Connection, ToSql};
use std::collections::HashSet;
use tauri::State;

/// 하루치 구간. **학급으로 거른다** — 학생으로 거르면 그 학생이 내 교과 강좌에도
/// 있을 때 두 화면이 서로의 기록을 보게 된다.
///
/// 별칭은 `attendance::load_spans`가 정한 것을 그대로 쓴다. 구간이 `s`, 학생이 `st`다.
const DAY_SPANS: &str = "WHERE s.class_id = ?1 AND s.date = ?2
                         ORDER BY st.number, s.id";

pub fn get_home_summary_impl(
    conn: &Connection,
    class_id: i64,
    date: &str,
    limit: i64,
) -> Result<HomeSummary, String> {
    let parsed = parse_date(date)?;
    let date = format_date(parsed);
    // 음수 limit은 거절하지 않고 0으로 본다. 몇 줄을 보여줄지는 화면의 취향이고,
    // 그것 때문에 개요 전체가 실패하면 밀린 일도 함께 안 보인다.
    let take = limit.max(0) as usize;
    let scope = homeroom_scope(conn, class_id)?;

    // 그날 명단 인원. 명단에서 제외되는 일과 전출은 각각 기간이 닫히므로, 지난 달을
    // 열면 그날의 인원이 그대로 나온다. 오늘 기준으로 세면 "그날 서른 명 중 몇 명"이
    // 틀린 문장이 된다. 세는 규칙은 하루 격자와 **같은 도우미**가 들고 있다 —
    // 개요가 말하는 인원과 격자에 뜨는 줄 수가 다르면 어느 쪽이 맞는지 알 수 없다.
    let enrolled = member_count_on(conn, scope.id, &date)?;

    // 오늘치 — 입력한 학생 수와 미완성 구간 수.
    // 기준일은 화면이 넘긴 날이다. 시스템 시계를 읽으면 지난 날짜를 열어 둔 채
    // 정리하는 동안 화면이 보는 날과 경과일의 기준이 어긋난다.
    let params: [&dyn ToSql; 2] = [&scope.id, &date];
    let today_spans = load_spans(conn, DAY_SPANS, &params, &date)?;
    let recorded = today_spans
        .iter()
        .map(|s| s.student_id)
        .collect::<HashSet<i64>>()
        .len() as i64;
    let incomplete = today_spans.iter().filter(|s| !s.complete).count() as i64;

    // 밀린 일 — 날짜로 자르지 않는다. 지난 달 것이 남아 있는 것이 바로 밀린 일이다.
    let mut doc_rows = get_doc_pending_impl(conn, scope.id, None, None, false, &date)?;
    let doc_pending = doc_rows.len() as i64;
    // 경과일은 `mark`가 같은 기준일로 이미 세어 둔 값이다. 여기서 다시 세면 두 화면의
    // "며칠 지났다"가 달라질 수 있다.
    let doc_overdue = doc_rows
        .iter()
        .filter(|s| s.days_overdue.is_some_and(|d| d > 0))
        .count() as i64;
    doc_rows.truncate(take);

    let neis_groups = get_neis_pending_impl(conn, scope.id, None, None, &date)?;
    let mut neis_rows: Vec<SpanItem> = neis_groups.into_iter().flat_map(|g| g.spans).collect();
    let neis_pending = neis_rows.len() as i64;
    neis_rows.truncate(take);

    Ok(HomeSummary {
        date_label: format_korean(parsed),
        date,
        enrolled,
        recorded,
        incomplete,
        doc_pending,
        doc_overdue,
        neis_pending,
        doc_rows,
        neis_rows,
    })
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_home_summary(
    db: State<DbState>,
    class_id: i64,
    date: String,
    limit: i64,
) -> Result<HomeSummary, String> {
    with_conn(&db, |c| get_home_summary_impl(c, class_id, &date, limit))
}
