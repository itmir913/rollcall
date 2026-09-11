//! 출결 입력과 수정. 이 앱의 심장이다.
//!
//! **미완성 기록이 정상 상태다.** `reason_id`와 `type_id`는 각각 비어 있을 수 있고,
//! 비어 있으면 "아직 안 정했다"는 뜻이다. 학생이 안 왔는데 연락이 닿지 않으면 그날은
//! 날짜와 학생만 남는다. 그래서 저장 함수는 어느 축이 비었다고 거절하지 않는다.
//!
//! **프로그램은 판정하지 않는다.** 한도 초과도, 구간 겹침도 저장을 막지 않는다.
//! 겹침은 화면에 표시할 값(`overlapping`)으로만 돌려준다.
//!
//! **같은 조합을 다시 찍으면 취소다.** 구분 · 종류 · 기간이 모두 같은 건이 이미
//! 있으면 추가하지 않고 그 건을 지운다. 실수로 두 번 누른 것과 "방금 찍은 것을
//! 무르고 싶다"가 교사에게는 같은 동작이기 때문이다. 조합이 하나라도 다르면 쌓는다 —
//! 한 학생이 `1교시 지각`과 `5~7교시 조퇴`를 함께 가지는 것이 정상이다.
//!
//! **다만 그 구간에 태그 · 사유 · 서류 · NEIS 표시가 남아 있으면 지우지 않는다.**
//! 한 번 더 누르면 되돌아온다는 전제가 그때는 거짓이다 — 다시 찍어도 빈 구간만
//! 돌아오고 교사가 적어 둔 것은 사라진다. 그 구간은 그대로 두고 알리기만 한다.
//!
//! **범위는 담임 학급 하나(`classId`)다.** 구간이 그 학급을 직접 가리키므로 조건도
//! `s.class_id`다. 학생으로 거르지 않는 이유는, 같은 학생이 내 담임 반에도 내 교과
//! 강좌에도 있을 수 있어 학생으로 거르면 교과 화면에 담임 출결이 새기 때문이다.
//!
//! 트랜잭션 규칙: 여러 문장을 쓰는 경로는 `db::with_transaction`으로만 연다.
//! 조기 반환은 클로저 **안에서** `?`로 한다(db.rs 참고).

use crate::commands::class::{homeroom_scope, member_count_on, member_rows_on};
use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::{self, format_date, format_korean, parse_date};
use crate::slots::{self, CLOSING, HOMEROOM, UNKNOWN};
use crate::state::{constraint_err, DbState};
use crate::types::*;
use chrono::NaiveDate;
use rusqlite::{params, Connection, ToSql};
use std::collections::HashSet;
use tauri::State;

// ── 학교 설정 ─────────────────────────────────────────────────

/// 학교가 들고 있는 값. 앱 상수도 전역 설정도 아니다.
pub(crate) struct SchoolSettings {
    pub max_slot: i64,
    pub due_days: i64,
    pub due_skip_offdays: bool,
}

pub(crate) fn school_settings(conn: &Connection, school_id: i64) -> Result<SchoolSettings, String> {
    conn.query_row(
        "SELECT max_slot, due_days, due_skip_offdays FROM school WHERE id = ?1",
        params![school_id],
        |r| {
            Ok(SchoolSettings {
                max_slot: r.get(0)?,
                due_days: r.get(1)?,
                due_skip_offdays: r.get::<_, i64>(2)? != 0,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => format!("학교를 찾을 수 없습니다: {school_id}"),
        other => other.to_string(),
    })
}

/// 하루의 마지막 교시. 조회와 종례는 설정 대상이 아니라 언제나 양 끝이다.
pub(crate) fn max_slot_of(conn: &Connection, school_id: i64) -> Result<i64, String> {
    Ok(school_settings(conn, school_id)?.max_slot)
}

/// 마감을 셀 때 건너뛸 날. 학사일정이 아니라 그 목록일 뿐이다.
pub(crate) fn off_days_of(conn: &Connection, school_id: i64) -> Result<HashSet<NaiveDate>, String> {
    let mut stmt = conn
        .prepare("SELECT date FROM off_day WHERE school_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![school_id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    // 형식이 깨진 행은 마감 계산에서 제외한다. 그것 때문에 저장이 막히면 안 된다.
    Ok(rows.iter().filter_map(|s| parse_date(s).ok()).collect())
}

/// 그 학생이 이 학급 명단인지 확인한다. **날짜로 좁히지 않는다** — 지난 날짜를 열어
/// 정리하는 동안 지금 명단에서 빠진 학생의 그날 기록을 고칠 수 있어야 한다.
///
/// 확인하는 이유는 판정이 아니라 메시지다. 그냥 넣으면 외래키 위반이 "참조하는 항목이
/// 없습니다"로 올라와, 교사는 무엇이 잘못됐는지 알 수 없다.
fn ensure_member(conn: &Connection, class_id: i64, student_id: i64) -> Result<(), String> {
    let found: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM class_member WHERE class_id = ?1 AND student_id = ?2",
            params![class_id, student_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if found == 0 {
        return Err(format!("이 학급 명단에 없는 학생입니다: {student_id}"));
    }
    Ok(())
}

/// 그 구간이 어느 학급의 것인가. **학생이 아니라 학급에서 학교를 얻는다** —
/// 같은 학생이 학교를 옮겨 다니지는 않지만, 범위를 말하는 것은 언제나 학급이다.
fn span_class(conn: &Connection, span_id: i64) -> Result<i64, String> {
    conn.query_row(
        "SELECT class_id FROM absence_span WHERE id = ?1",
        params![span_id],
        |r| r.get(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => format!("출결 기록을 찾을 수 없습니다: {span_id}"),
        other => other.to_string(),
    })
}

/// 종류가 물어야 하는 기간이 어느 쪽인지. **DB에서 읽는다** —
/// 라벨 '결석'으로 비교하면 교사가 종류를 추가하는 순간 틀린다.
fn slot_prompt_of(conn: &Connection, type_id: Option<i64>) -> Result<Option<String>, String> {
    let Some(id) = type_id else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT slot_prompt FROM attendance_type WHERE id = ?1",
        params![id],
        |r| r.get::<_, String>(0),
    )
    .map(Some)
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => format!("출결 종류를 찾을 수 없습니다: {id}"),
        other => other.to_string(),
    })
}

// ── 구간 문구 (순수 함수) ─────────────────────────────────────

/// 순서값을 슬롯 토큰으로 되돌린다. 0은 조회, 마지막+1은 종례다.
fn token_of(ordinal: usize, max_slot: usize) -> String {
    if ordinal == 0 {
        HOMEROOM.to_string()
    } else if ordinal > max_slot {
        CLOSING.to_string()
    } else {
        ordinal.to_string()
    }
}

/// 사람이 읽는 구간 표기. 열린 쪽은 `?`다.
///
/// 종류가 하루 전체(`none`)면 기간을 묻지 않으므로 "하루 종일"이고,
/// 양쪽이 다 비면 "기간 미정"이다. 그 밖에는 실제로 저장된 두 끝을 그대로 적는다 —
/// 화면이 보여주는 문장과 DB 값이 어긋나면 교사가 무엇을 고쳐야 할지 알 수 없다.
pub(crate) fn span_text(
    slot_prompt: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
) -> String {
    if slot_prompt == Some("none") {
        return "하루 종일".to_string();
    }
    if start.is_none() && end.is_none() {
        return "기간 미정".to_string();
    }
    let disp = |v: &str| slots::display(v);
    // 두 끝이 같으면 슬롯 하나다. "조회부터 조회까지"로 적지 않는다 —
    // 나이스 실파일에 결시교시가 `조회,` 하나뿐인 지각이 있었다.
    if let (Some(a), Some(b)) = (start, end) {
        if a == b {
            return disp(a);
        }
    }
    match slot_prompt {
        // 지각 — 온 때 하나를 묻는다. 시작은 조회다.
        Some("end") => format!(
            "{}부터 {}까지",
            disp(start.unwrap_or(HOMEROOM)),
            disp(end.unwrap_or(UNKNOWN))
        ),
        // 조퇴 — 나간 때 하나를 묻는다. 끝은 종례다.
        Some("start") => format!(
            "{}부터 {}까지",
            disp(start.unwrap_or(UNKNOWN)),
            disp(end.unwrap_or(CLOSING))
        ),
        _ => format!(
            "{}부터 {}까지",
            disp(start.unwrap_or(UNKNOWN)),
            disp(end.unwrap_or(UNKNOWN))
        ),
    }
}

/// 고른 교시를 저장할 구간들로 바꾼다. 순수 함수다.
///
/// · 종류가 `none`이면 조회~종례 한 건이다(결석은 교시를 묻지 않는다).
/// · 고른 교시가 없으면 양쪽이 열린 한 건이다(기간 미정).
/// · `end`는 조회~고른 값, `start`는 고른 값~종례.
/// · 그 밖(`multi`, 종류 미정)은 **연속한 것끼리 묶어** 여러 건으로 나눈다.
///   1,2,3은 한 건이고 1,3,5는 세 건이다 — 연속하지 않은 것을 한 구간으로 저장하면
///   2교시가 조용히 포함된다.
pub(crate) fn ranges_for(
    slot_prompt: Option<&str>,
    picked: &[String],
    max_slot: usize,
) -> Result<Vec<(Option<String>, Option<String>)>, String> {
    if slot_prompt == Some("none") {
        return Ok(vec![(
            Some(HOMEROOM.to_string()),
            Some(CLOSING.to_string()),
        )]);
    }
    if picked.is_empty() {
        return Ok(vec![(None, None)]);
    }

    // `?`는 **묻고 있는 쪽이 열려 있다**는 뜻이다(`slots.rs`: "저장은 NULL이다").
    // 지각이면 조회부터 언제까지인지 모르는 것이고, 조퇴면 언제부터인지 모르는 채
    // 종례까지다. 이 길이 없으면 화면의 `?` 버튼이 눌러도 저장되지 않는다.
    if picked.iter().any(|s| s == UNKNOWN) {
        return match slot_prompt {
            Some("end") => Ok(vec![(Some(HOMEROOM.to_string()), None)]),
            Some("start") => Ok(vec![(None, Some(CLOSING.to_string()))]),
            _ => Err(format!("여기서는 {UNKNOWN}를 고를 수 없습니다.")),
        };
    }

    let mut ordinals = Vec::with_capacity(picked.len());
    for slot in picked {
        let o = slots::ordinal(slot, max_slot)
            .ok_or_else(|| format!("알 수 없는 교시입니다: {slot}"))?;
        ordinals.push(o);
    }
    ordinals.sort_unstable();
    ordinals.dedup();

    let ranges = match slot_prompt {
        Some("end") => vec![(
            Some(HOMEROOM.to_string()),
            Some(token_of(*ordinals.last().unwrap(), max_slot)),
        )],
        Some("start") => vec![(
            Some(token_of(ordinals[0], max_slot)),
            Some(CLOSING.to_string()),
        )],
        _ => slots::group_runs(ordinals)
            .into_iter()
            .map(|(a, b)| {
                (
                    Some(token_of(a, max_slot)),
                    Some(token_of(b, max_slot)),
                )
            })
            .collect(),
    };

    for (s, e) in &ranges {
        slots::validate_span(s.as_deref(), e.as_deref(), max_slot)?;
    }
    Ok(ranges)
}

// ── 구간 읽기 ─────────────────────────────────────────────────

/// 코드는 두 축이 다 채워졌을 때만 붙는다. LEFT JOIN인 이유다.
///
/// 별칭은 구간이 `s`, 학생이 `st`, 학교가 `sc`다. 조건을 적는 쪽이 이 이름을 쓴다.
/// **별칭은 하나다** — 같은 행을 두 이름으로 부를 수 있게 두면 부르는 쪽마다 다른 이름을
/// 쓰게 되고, 어느 쪽이 진짜인지 조건을 읽는 사람이 매번 되물어야 한다.
///
/// 학교를 함께 조인하는 이유는 최대 교시가 **학교가 들고 있는 값**이기 때문이다 —
/// 순회 교사가 학교를 둘 이상 등록해도 각 행이 제 학교의 값으로 겹침을 센다.
const SPAN_SELECT: &str = "SELECT s.id, s.student_id, st.number, st.name, s.date,
                                  s.reason_id, s.type_id, r.label, t.label, c.label,
                                  s.start_slot, s.end_slot, t.slot_prompt,
                                  s.tag_id, g.name, s.memo,
                                  s.doc_done, s.doc_due, s.doc_done_on,
                                  s.neis_done, s.neis_done_on, s.group_id,
                                  sc.max_slot,
                                  st.grade, st.class_no
                           FROM absence_span s
                           JOIN student st ON st.id = s.student_id
                           JOIN school sc ON sc.id = st.school_id
                           LEFT JOIN attendance_reason r ON r.id = s.reason_id
                           LEFT JOIN attendance_type t ON t.id = s.type_id
                           LEFT JOIN attendance_code c
                                  ON c.reason_id = s.reason_id
                                 AND c.type_id = s.type_id
                                 AND c.valid_from <= s.date
                                 AND (c.valid_to IS NULL OR s.date < c.valid_to)
                           LEFT JOIN span_tag g ON g.id = s.tag_id ";

fn map_span(row: &rusqlite::Row, today: NaiveDate) -> rusqlite::Result<SpanItem> {
    let date: String = row.get(4)?;
    let reason_id: Option<i64> = row.get(5)?;
    let type_id: Option<i64> = row.get(6)?;
    let start_slot: Option<String> = row.get(10)?;
    let end_slot: Option<String> = row.get(11)?;
    let slot_prompt: Option<String> = row.get(12)?;
    let doc_done: bool = row.get::<_, i64>(16)? != 0;
    let doc_due: Option<String> = row.get(17)?;

    // 이미 받은 서류는 마감을 세지 않는다.
    let days_overdue = match (doc_done, doc_due.as_deref()) {
        (false, Some(d)) => parse_date(d).ok().map(|due| due::days_overdue(due, today)),
        _ => None,
    };
    let date_label = parse_date(&date)
        .map(format_korean)
        .unwrap_or_else(|_| date.clone());

    Ok(SpanItem {
        id: row.get(0)?,
        student_id: row.get(1)?,
        // 학적은 **맨 뒤에 붙어 있다.** 가운데 끼우면 아래 스무 남짓한 자리가 전부
        // 한 칸씩 밀리고, 그 밀림은 형이 같은 열끼리 조용히 어긋난다.
        grade: row.get(23)?,
        class_no: row.get(24)?,
        number: row.get(2)?,
        name: row.get(3)?,
        date,
        date_label,
        reason_id,
        type_id,
        reason_label: row.get(7)?,
        type_label: row.get(8)?,
        code_label: row.get(9)?,
        span_text: span_text(
            slot_prompt.as_deref(),
            start_slot.as_deref(),
            end_slot.as_deref(),
        ),
        start_slot,
        end_slot,
        slot_prompt,
        tag_id: row.get(13)?,
        tag_name: row.get(14)?,
        memo: row.get(15)?,
        doc_done,
        doc_due,
        doc_done_on: row.get(18)?,
        days_overdue,
        neis_done: row.get::<_, i64>(19)? != 0,
        neis_done_on: row.get(20)?,
        group_id: row.get(21)?,
        complete: reason_id.is_some() && type_id.is_some(),
        overlapping: false,
    })
}

/// 같은 날 같은 학생의 다른 구간과 시간이 겹치는지 표시한다.
///
/// **막지 않는다.** 3교시 결과와 3교시 조퇴가 함께 있는 것은 실수일 수도 의도일 수도
/// 있고, 판정은 교사 몫이다. 겹침은 **돌려준 목록 안에서** 센다 — 화면이 들고 있는
/// 줄들끼리 비교한 결과라는 뜻이다. 최대 교시는 그 학생의 학교에서 읽는다.
fn mark_overlaps(items: &mut [SpanItem], max_slots: &[i64]) {
    let mut flags = vec![false; items.len()];
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            if items[i].student_id != items[j].student_id || items[i].date != items[j].date {
                continue;
            }
            let a = (items[i].start_slot.as_deref(), items[i].end_slot.as_deref());
            let b = (items[j].start_slot.as_deref(), items[j].end_slot.as_deref());
            if slots::overlaps(a, b, max_slots[i] as usize) {
                flags[i] = true;
                flags[j] = true;
            }
        }
    }
    for (item, flag) in items.iter_mut().zip(flags) {
        item.overlapping = flag;
    }
}

/// 화면에 뿌릴 구간 목록. **다른 모듈(서류 · 개요 · 통계 · 내보내기)이 함께 쓴다.**
///
/// `where_sql`은 `SPAN_SELECT` 뒤에 그대로 붙는다 — `WHERE ...`부터 `ORDER BY ...`까지
/// 부르는 쪽이 적고, 매개변수는 `?1`부터 쓴다. 별칭은 구간이 `s`, 학생이 `st`, 학교가 `sc`다.
///
/// `today`는 화면이 넘긴 기준일(ISO)이다. 마감 경과일을 이 날짜로 센다 —
/// 여기서 시계를 읽으면 지난 날짜를 열어 둔 채 정리하는 동안 화면이 보는 날과 어긋난다.
pub(crate) fn load_spans(
    conn: &Connection,
    where_sql: &str,
    params: &[&dyn ToSql],
    today: &str,
) -> Result<Vec<SpanItem>, String> {
    load_spans_on(conn, where_sql, params, parse_date(today)?)
}

/// 조건 문자열에 `WHERE`가 이미 붙어 있으면 그대로, 아니면 붙여서 쓴다.
/// 부르는 쪽마다 적는 방식이 달라 어느 쪽이 와도 같은 질의가 나오게 한다.
fn where_clause(where_sql: &str) -> String {
    let trimmed = where_sql.trim();
    if trimmed.is_empty() {
        return "WHERE 1 = 1".to_string();
    }
    let already = trimmed
        .get(..5)
        .is_some_and(|head| head.eq_ignore_ascii_case("WHERE"));
    if already {
        trimmed.to_string()
    } else {
        format!("WHERE {trimmed}")
    }
}

pub(crate) fn load_spans_on(
    conn: &Connection,
    where_sql: &str,
    params: &[&dyn ToSql],
    today: NaiveDate,
) -> Result<Vec<SpanItem>, String> {
    let sql = format!("{SPAN_SELECT} {}", where_clause(where_sql));
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let loaded = stmt
        .query_map(params, |row| {
            Ok((map_span(row, today)?, row.get::<_, i64>(22)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let (mut items, max_slots): (Vec<SpanItem>, Vec<i64>) = loaded.into_iter().unzip();
    mark_overlaps(&mut items, &max_slots);
    Ok(items)
}

// ── 구간 쓰기 ─────────────────────────────────────────────────

/// 구분 · 종류 · 기간이 **완전히 같은** 구간을 찾는다. NULL끼리도 같은 것으로 본다.
///
/// 학급까지 함께 본다. 무르기는 지금 보고 있는 학급 안에서의 일이라,
/// 학생만으로 찾으면 다른 학급에 찍어 둔 같은 모양의 기록이 대신 지워진다.
#[allow(clippy::too_many_arguments)]
fn find_exact(
    conn: &Connection,
    class_id: i64,
    student_id: i64,
    date: &str,
    reason_id: Option<i64>,
    type_id: Option<i64>,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
) -> Result<Option<i64>, String> {
    conn.query_row(
        "SELECT id FROM absence_span
         WHERE class_id = ?1 AND student_id = ?2 AND date = ?3
           AND reason_id IS ?4 AND type_id IS ?5
           AND start_slot IS ?6 AND end_slot IS ?7
         ORDER BY id LIMIT 1",
        params![
            class_id, student_id, date, reason_id, type_id, start_slot, end_slot
        ],
        |r| r.get::<_, i64>(0),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other.to_string()),
    })
}

/// 그 구간에 교사가 적어 둔 것들. 무르기가 지워서는 안 되는 값의 이름이다.
///
/// **무르기의 전제는 그 구간에 아무것도 붙어 있지 않다는 것이다.** 한 번 더 누르면
/// 되돌아오므로 묻지 않는다고 했는데, 태그를 달고 사유를 적고 서류를 받았다고 표시한
/// 구간을 지우면 다시 찍어도 빈 구간만 돌아온다 — 교사가 적어 둔 것이 말없이 사라진다.
/// 그래서 하나라도 채워져 있으면 무르기가 지나가고, 지우는 일은 삭제가 맡는다.
fn span_notes(conn: &Connection, span_id: i64) -> Result<Vec<&'static str>, String> {
    let (tag_id, memo, doc_done, neis_done) = conn
        .query_row(
            "SELECT tag_id, memo, doc_done, neis_done FROM absence_span WHERE id = ?1",
            params![span_id],
            |r| {
                Ok((
                    r.get::<_, Option<i64>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                format!("출결 기록을 찾을 수 없습니다: {span_id}")
            }
            other => other.to_string(),
        })?;

    let mut notes = Vec::new();
    if tag_id.is_some() {
        notes.push("태그");
    }
    if !memo.trim().is_empty() {
        notes.push("사유");
    }
    if doc_done != 0 {
        notes.push("서류 표시");
    }
    if neis_done != 0 {
        notes.push("NEIS 표시");
    }
    Ok(notes)
}

#[allow(clippy::too_many_arguments)]
fn insert_span(
    conn: &Connection,
    class_id: i64,
    student_id: i64,
    date: &str,
    reason_id: Option<i64>,
    type_id: Option<i64>,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
    doc_due: Option<&str>,
    group_id: Option<&str>,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO absence_span
           (class_id, student_id, date, reason_id, type_id, start_slot, end_slot,
            doc_due, group_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            class_id, student_id, date, reason_id, type_id, start_slot, end_slot, doc_due,
            group_id
        ],
    )
    .map_err(|e| constraint_err(&e, "이미 같은 구간이 있습니다."))?;
    Ok(conn.last_insert_rowid())
}

/// 서류 제출 마감. **만들 때 계산해 박는다** — 설정을 바꿔도 과거 기록의 마감이
/// 소급 변경되지 않아야 하기 때문이다.
pub(crate) fn due_for(
    date: NaiveDate,
    settings: &SchoolSettings,
    off_days: &HashSet<NaiveDate>,
) -> String {
    format_date(due::due_date(
        date,
        settings.due_days,
        settings.due_skip_offdays,
        off_days,
    ))
}

/// 출결 한 건을 찍는다. **같은 조합을 다시 찍으면 취소다.**
///
/// 고른 교시가 여러 묶음이면 구간도 여러 건이 된다. 그 전부가 이미 있을 때만
/// 취소로 보고 지운다 — 일부만 있으면 나머지를 채우는 것이 교사의 의도다.
///
/// 그중 하나라도 교사가 적어 둔 것을 들고 있으면 `kept`로 돌려주고 아무것도 지우지
/// 않는다. 절반만 지우면 무엇이 사라졌는지 알 수 없기 때문이다.
pub fn stamp_span_impl(conn: &Connection, input: &StampInput) -> Result<StampResult, String> {
    let base = parse_date(&input.date)?;
    // **저장은 언제나 자리를 채운 ISO다.** chrono는 `2026-9-10`도 받아 주는데, 그대로
    // 넣으면 날짜로 거르는 모든 화면에서 그 행이 사라진다 — 날짜 비교가 문자열 비교라
    // `2026-9-10`은 `2026-09-30`보다 뒤로 읽히고, `LIKE '2026-09%'`에도 걸리지 않는다.
    let date = format_date(base);
    let scope = homeroom_scope(conn, input.class_id)?;
    ensure_member(conn, scope.id, input.student_id)?;
    let settings = school_settings(conn, scope.school_id)?;
    let prompt = slot_prompt_of(conn, input.type_id)?;
    let ranges = ranges_for(prompt.as_deref(), &input.slots, settings.max_slot as usize)?;

    with_transaction(conn, || {
        let mut found = Vec::with_capacity(ranges.len());
        for (s, e) in &ranges {
            found.push(find_exact(
                conn,
                scope.id,
                input.student_id,
                &date,
                input.reason_id,
                input.type_id,
                s.as_deref(),
                e.as_deref(),
            )?);
        }

        // 전부 이미 있다 = 무르기.
        if found.iter().all(|f| f.is_some()) {
            let ids: Vec<i64> = found.into_iter().flatten().collect();

            // **교사가 적어 둔 것이 하나라도 있으면 지우지 않는다.** 거절도 아니고
            // 새 구간을 만들지도 않는다 — 완전히 같은 두 건은 만들지 않기로 한 규칙이
            // 그대로 살아 있어야 한다. 그 구간을 그대로 두고 무엇이 남아 있는지 알린다.
            let mut notes: Vec<&'static str> = Vec::new();
            for id in &ids {
                for note in span_notes(conn, *id)? {
                    if !notes.contains(&note) {
                        notes.push(note);
                    }
                }
            }
            if !notes.is_empty() {
                return Ok(StampResult {
                    action: "kept".to_string(),
                    span_ids: ids,
                    message: Some(crate::phrase::apply_josa(&format!(
                        "{}(이)가 남아 있어 무르지 않았습니다. \
                         지우려면 그 줄의 휴지통 버튼을 쓰세요.",
                        notes.join(" · ")
                    ))),
                });
            }

            for id in &ids {
                conn.execute("DELETE FROM absence_span WHERE id = ?1", params![id])
                    .map_err(|e| e.to_string())?;
            }
            return Ok(StampResult {
                action: "cancelled".to_string(),
                span_ids: ids,
                message: None,
            });
        }

        let off_days = off_days_of(conn, scope.school_id)?;
        let doc_due = due_for(base, &settings, &off_days);
        let mut ids = Vec::new();
        for ((s, e), existing) in ranges.iter().zip(found) {
            if existing.is_some() {
                continue;
            }
            ids.push(insert_span(
                conn,
                scope.id,
                input.student_id,
                &date,
                input.reason_id,
                input.type_id,
                s.as_deref(),
                e.as_deref(),
                Some(&doc_due),
                None,
            )?);
        }
        Ok(StampResult {
            action: "added".to_string(),
            span_ids: ids,
            message: None,
        })
    })
}

/// 구간 하나의 두 축과 기간을 고친다.
///
/// **마감은 다시 계산하지 않는다.** 이미 교사가 학부모에게 말해 둔 날짜이고,
/// 축을 고쳤다고 소급해 움직이면 그 약속이 조용히 달라진다.
pub fn edit_span_impl(conn: &Connection, edit: &SpanEdit) -> Result<(), String> {
    let scope = homeroom_scope(conn, span_class(conn, edit.span_id)?)?;
    let max_slot = max_slot_of(conn, scope.school_id)?;
    let prompt = slot_prompt_of(conn, edit.type_id)?;
    let ranges = ranges_for(prompt.as_deref(), &edit.slots, max_slot as usize)?;

    // 한 행은 한 구간이다. 1,3,5처럼 연속하지 않은 교시는 담을 자리가 없다.
    if ranges.len() > 1 {
        return Err(
            "연속하지 않은 교시는 한 구간으로 고칠 수 없습니다. 지운 뒤 다시 찍어주세요."
                .to_string(),
        );
    }
    let (start, end) = ranges.into_iter().next().unwrap();

    with_transaction(conn, || {
        let changed = conn
            .execute(
                "UPDATE absence_span
                 SET reason_id = ?1, type_id = ?2, start_slot = ?3, end_slot = ?4
                 WHERE id = ?5",
                params![
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
    })
}

pub fn delete_span_impl(conn: &Connection, span_id: i64) -> Result<(), String> {
    let removed = conn
        .execute("DELETE FROM absence_span WHERE id = ?1", params![span_id])
        .map_err(|e| e.to_string())?;
    if removed == 0 {
        return Err(format!("출결 기록을 찾을 수 없습니다: {span_id}"));
    }
    Ok(())
}

/// 메모는 자유 문장이다. 앱은 해석하지 않고 세는 데도 쓰지 않는다.
pub fn set_span_memo_impl(conn: &Connection, span_id: i64, memo: &str) -> Result<(), String> {
    let changed = conn
        .execute(
            "UPDATE absence_span SET memo = ?1 WHERE id = ?2",
            params![memo, span_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("출결 기록을 찾을 수 없습니다: {span_id}"));
    }
    Ok(())
}

/// 그 태그가 **이 구간의 학교가 들고 있는 유효한 태그인가.**
///
/// `school.rs`의 `check_rule_tag`와 짝이다. 태그 목록은 학교 설정이고 한도 규정이
/// 그 이름을 가리키는데, FK는 `span_tag`의 존재만 볼 뿐 어느 학교의 것인지도
/// 마감됐는지도 보지 않는다. 한쪽에만 검사가 있으면 규정으로는 막힌 태그가
/// 출결 한 건에는 그대로 붙고, 그 건은 어느 규정으로도 세어지지 않는다.
fn ensure_school_tag(conn: &Connection, school_id: i64, tag_id: i64) -> Result<(), String> {
    let live: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM span_tag
              WHERE id = ?1 AND school_id = ?2 AND valid_to IS NULL",
            params![tag_id, school_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if live == 0 {
        return Err(format!("이 학교의 유효한 태그가 아닙니다: {tag_id}"));
    }
    Ok(())
}

/// 태그는 **한 건에 하나다.** 여러 개를 허용하면 같은 하루가 두 한도를 동시에
/// 깎는지 정해야 하고, 그 판단은 프로그램이 할 일이 아니다.
pub fn set_span_tag_impl(
    conn: &Connection,
    span_id: i64,
    tag_id: Option<i64>,
) -> Result<(), String> {
    if let Some(tag_id) = tag_id {
        let scope = homeroom_scope(conn, span_class(conn, span_id)?)?;
        ensure_school_tag(conn, scope.school_id, tag_id)?;
    }
    let changed = conn
        .execute(
            "UPDATE absence_span SET tag_id = ?1 WHERE id = ?2",
            params![tag_id, span_id],
        )
        .map_err(|e| constraint_err(&e, "이미 같은 태그가 있습니다."))?;
    if changed == 0 {
        return Err(format!("출결 기록을 찾을 수 없습니다: {span_id}"));
    }
    Ok(())
}

// ── 화면별 묶음 ───────────────────────────────────────────────

/// 하루치 격자. 그날 명단에 있던 학생 전원이 행으로 나온다.
///
/// 구간이 없어도 행은 나온다 — 빈 행이 곧 출석이고, 출석은 저장하지 않는다.
/// 명단에서 빠졌거나 전출한 학생은 그날 명단이 아니므로 빠진다.
pub fn get_day_grid_impl(
    conn: &Connection,
    class_id: i64,
    date: &str,
) -> Result<DayGrid, String> {
    let parsed = parse_date(date)?;
    // 저장된 날짜는 자리를 채운 ISO다. 화면이 `2026-9-10`을 넘겨도 같은 날을 찾도록 맞춘다.
    let date = format_date(parsed);
    let scope = homeroom_scope(conn, class_id)?;
    let max_slot = max_slot_of(conn, scope.school_id)?;
    let students = member_rows_on(conn, scope.id, &date)?;

    // 구간은 **학급으로 거른다.** 학생으로 거르면 그 학생이 내 교과 강좌에도 있을 때
    // 두 화면이 서로의 기록을 보게 된다.
    // 격자의 기준일은 교사가 보고 있는 날이다. 지난 날짜를 열어 정리하는 경우가 있다.
    let spans = load_spans_on(
        conn,
        "WHERE s.class_id = ?1 AND s.date = ?2
         ORDER BY st.number, s.id",
        &[&scope.id, &date],
        parsed,
    )?;

    // 명단에 있는 학생이 격자의 줄이다. 다만 **그날 기록이 있는 학생은 명단에서
    // 빠졌더라도 줄을 만든다.** 명렬표를 다시 가져와 번호가 빠지면 그 학생의 그날
    // 구간은 학급에 그대로 남는데, 줄이 없으면 화면에서 손댈 수 없는 기록이 된다.
    // 그날 그 학생은 이 반이었다 — 지우는 것은 교사가 보고 정할 일이다.
    let mut rows: Vec<DayRow> = students
        .into_iter()
        .map(|(id, number, name)| DayRow {
            student_id: id,
            number,
            name,
            spans: spans.iter().filter(|sp| sp.student_id == id).cloned().collect(),
        })
        .collect();

    let listed: HashSet<i64> = rows.iter().map(|r| r.student_id).collect();
    let mut extra: Vec<DayRow> = Vec::new();
    for span in &spans {
        if listed.contains(&span.student_id) || extra.iter().any(|r| r.student_id == span.student_id)
        {
            continue;
        }
        extra.push(DayRow {
            student_id: span.student_id,
            number: span.number,
            name: span.name.clone(),
            spans: spans
                .iter()
                .filter(|sp| sp.student_id == span.student_id)
                .cloned()
                .collect(),
        });
    }
    rows.extend(extra);
    rows.sort_by_key(|r| r.number);

    Ok(DayGrid {
        date,
        date_label: format_korean(parsed),
        max_slot,
        rows,
        spans,
    })
}

/// 두 날짜 사이의 기록. **달이 아니라 기간으로 묻는다.**
///
/// NEIS 검증이 이것을 쓴다. 나이스 파일의 기간은 달에 맞춰 떨어지지 않고(월별 파일도
/// 학기 중 임의 구간일 수 있다), 화면이 마지막에 보던 달로 대조하면 6월 파일을 9월
/// 기록과 맞춰 보고는 "전부 앱에 없음"이라고 말한다.
pub fn get_spans_between_impl(
    conn: &Connection,
    class_id: i64,
    from: &str,
    to: &str,
    today: &str,
) -> Result<Vec<SpanItem>, String> {
    // 자리를 채운 ISO로 맞춘다. 날짜 비교가 문자열 비교라 `2026-9-1`이 오면
    // 그 달의 기록이 통째로 빠진다.
    let from = format_date(parse_date(from)?);
    let to = format_date(parse_date(to)?);
    if to < from {
        return Err("끝 날짜가 시작 날짜보다 앞입니다.".to_string());
    }
    let scope = homeroom_scope(conn, class_id)?;
    load_spans(
        conn,
        "WHERE s.class_id = ?1 AND s.date >= ?2 AND s.date <= ?3
         ORDER BY s.date, st.number, s.id",
        &[&scope.id, &from, &to],
        today,
    )
}

/// 한 달치 기록을 날짜별로 묶는다. 최신 날짜가 먼저다.
///
/// 명단에서 빠진 학생의 지난 기록도 그대로 나온다 — 그 구간이 이 학급을 가리키고 있다.
/// `enrolled`는 그날 명단 인원이므로 지금 인원과 다를 수 있다.
pub fn get_month_log_impl(
    conn: &Connection,
    class_id: i64,
    year: i64,
    month: i64,
) -> Result<Vec<DayGroup>, String> {
    let scope = homeroom_scope(conn, class_id)?;
    let first = NaiveDate::from_ymd_opt(year as i32, month as u32, 1)
        .ok_or_else(|| format!("연월이 올바르지 않습니다: {year}-{month}"))?;
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year as i32 + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year as i32, month as u32 + 1, 1)
    }
    .ok_or_else(|| format!("연월이 올바르지 않습니다: {year}-{month}"))?;
    let last = next_month
        .pred_opt()
        .ok_or_else(|| format!("연월이 올바르지 않습니다: {year}-{month}"))?;

    let from = format_date(first);
    let to = format_date(last);

    // 경과일을 쓰지 않는 표라 기준일은 그 달의 끝으로 넘긴다.
    let spans = load_spans_on(
        conn,
        "WHERE s.class_id = ?1 AND s.date >= ?2 AND s.date <= ?3
         ORDER BY s.date DESC, st.number, s.id",
        &[&scope.id, &from, &to],
        last,
    )?;

    let mut groups: Vec<DayGroup> = Vec::new();
    for span in spans {
        if groups.last().map(|g| g.date != span.date).unwrap_or(true) {
            let enrolled = member_count_on(conn, scope.id, &span.date)?;
            groups.push(DayGroup {
                date: span.date.clone(),
                date_label: span.date_label.clone(),
                enrolled,
                spans: Vec::new(),
            });
        }
        groups.last_mut().unwrap().spans.push(span);
    }
    Ok(groups)
}

// ── 여러 날 일괄 입력 ─────────────────────────────────────────

/// 주말과 등록된 휴업일을 뺀 날 목록.
///
/// 등록되지 않은 휴업일은 교사가 미리보기에서 지운다 — 앱은 학사일정을 모른다.
///
/// `hasExisting`은 **그 학생에게** 그날 기록이 있는지다. 학급 서른 명 중 누구든
/// 하나 걸리면 되는 조건으로 세면 학기 중 거의 모든 날에 표시가 붙어, 정작 겹치는
/// 날이 눈에 들어오지 않는다. 학생을 넘기지 않으면 그 학급 전체로 센다.
pub fn preview_bulk_impl(
    conn: &Connection,
    class_id: i64,
    student_id: Option<i64>,
    from: &str,
    to: &str,
) -> Result<Vec<BulkPreviewDay>, String> {
    let from_d = parse_date(from)?;
    let to_d = parse_date(to)?;
    if to_d < from_d {
        return Err("끝 날짜가 시작 날짜보다 앞입니다.".to_string());
    }
    // 없는 학급이면 휴업일 목록이 빈 채로 넘어와 주말만 뺀 날짜가 나온다.
    // 조용히 틀린 미리보기를 내놓지 않도록 여기서 확인한다.
    let scope = homeroom_scope(conn, class_id)?;
    let off_days = off_days_of(conn, scope.school_id)?;

    let mut out = Vec::new();
    for d in due::open_days_between(from_d, to_d, &off_days) {
        let iso = format_date(d);
        let existing: i64 = match student_id {
            Some(id) => conn.query_row(
                "SELECT COUNT(*) FROM absence_span
                  WHERE class_id = ?1 AND student_id = ?2 AND date = ?3",
                params![scope.id, id, iso],
                |r| r.get(0),
            ),
            None => conn.query_row(
                "SELECT COUNT(*) FROM absence_span WHERE class_id = ?1 AND date = ?2",
                params![scope.id, iso],
                |r| r.get(0),
            ),
        }
        .map_err(|e| e.to_string())?;
        out.push(BulkPreviewDay {
            label: format_korean(d),
            date: iso,
            has_existing: existing > 0,
        });
    }
    Ok(out)
}

/// 묶음 식별자. **결정적 문자열이다** — 같은 입력이면 언제 눌러도 같은 값이 나온다.
/// 무작위 값이면 테스트가 값을 확인할 수 없고, 같은 묶음을 두 번 넣었을 때
/// 서로 다른 묶음으로 분리된다.
fn bulk_group_id(input: &StampInput, from: &str, to: &str) -> String {
    let axis = |v: Option<i64>| v.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string());
    let mut picked = input.slots.clone();
    picked.sort();
    format!(
        "bulk:{}:{}:{}~{}:{}:{}:{}",
        input.class_id,
        input.student_id,
        from,
        to,
        axis(input.reason_id),
        axis(input.type_id),
        picked.join(",")
    )
}

/// 여러 날에 같은 조합을 찍는다. 날짜는 `from`~`to`에서 주말·휴업일을 뺀 날이다.
///
/// 이미 완전히 같은 건이 있는 날은 **덮지도 지우지도 않고 그대로 둔다.** 무르기는
/// 하루짜리 입력의 규칙이고, 기간 입력에서까지 그러면 이미 넣어 둔 며칠이
/// 교사가 못 본 사이에 사라진다. `days`는 실제로 새로 넣은 날의 수다.
/// `input.date`는 쓰지 않는다 — 날짜는 기간이 정한다.
pub fn apply_bulk_impl(
    conn: &Connection,
    input: &StampInput,
    from: &str,
    to: &str,
) -> Result<BulkApplyResult, String> {
    let from_d = parse_date(from)?;
    let to_d = parse_date(to)?;
    if to_d < from_d {
        return Err("끝 날짜가 시작 날짜보다 앞입니다.".to_string());
    }
    // 묶음 이름에 들어가는 기간도 자리를 채운 ISO다. `2026-6-1`과 `2026-06-01`이
    // 서로 다른 묶음이 되면 "같은 입력이면 같은 묶음"이라는 성질이 깨진다.
    let (from, to) = (format_date(from_d), format_date(to_d));
    let scope = homeroom_scope(conn, input.class_id)?;
    ensure_member(conn, scope.id, input.student_id)?;
    let settings = school_settings(conn, scope.school_id)?;
    let prompt = slot_prompt_of(conn, input.type_id)?;
    let ranges = ranges_for(prompt.as_deref(), &input.slots, settings.max_slot as usize)?;
    let off_days = off_days_of(conn, scope.school_id)?;
    let days = due::open_days_between(from_d, to_d, &off_days);
    let group_id = bulk_group_id(input, &from, &to);

    with_transaction(conn, || {
        let mut applied = 0;
        for day in &days {
            let iso = format_date(*day);
            let doc_due = due_for(*day, &settings, &off_days);
            let mut added = false;
            for (s, e) in &ranges {
                let existing = find_exact(
                    conn,
                    scope.id,
                    input.student_id,
                    &iso,
                    input.reason_id,
                    input.type_id,
                    s.as_deref(),
                    e.as_deref(),
                )?;
                if existing.is_some() {
                    continue;
                }
                insert_span(
                    conn,
                    scope.id,
                    input.student_id,
                    &iso,
                    input.reason_id,
                    input.type_id,
                    s.as_deref(),
                    e.as_deref(),
                    Some(&doc_due),
                    Some(&group_id),
                )?;
                added = true;
            }
            if added {
                applied += 1;
            }
        }
        Ok(BulkApplyResult {
            group_id: group_id.clone(),
            days: applied,
        })
    })
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_day_grid(db: State<DbState>, class_id: i64, date: String) -> Result<DayGrid, String> {
    with_conn(&db, |c| get_day_grid_impl(c, class_id, &date))
}

#[tauri::command]
pub fn stamp_span(db: State<DbState>, input: StampInput) -> Result<StampResult, String> {
    with_conn(&db, |c| stamp_span_impl(c, &input))
}

#[tauri::command]
pub fn edit_span(db: State<DbState>, edit: SpanEdit) -> Result<(), String> {
    with_conn(&db, |c| edit_span_impl(c, &edit))
}

#[tauri::command]
pub fn delete_span(db: State<DbState>, span_id: i64) -> Result<(), String> {
    with_conn(&db, |c| delete_span_impl(c, span_id))
}

#[tauri::command]
pub fn set_span_memo(db: State<DbState>, span_id: i64, memo: String) -> Result<(), String> {
    with_conn(&db, |c| set_span_memo_impl(c, span_id, &memo))
}

#[tauri::command]
pub fn get_spans_between(
    db: State<DbState>,
    class_id: i64,
    from: String,
    to: String,
    today: String,
) -> Result<Vec<SpanItem>, String> {
    with_conn(&db, |c| {
        get_spans_between_impl(c, class_id, &from, &to, &today)
    })
}

#[tauri::command]
pub fn set_span_tag(
    db: State<DbState>,
    span_id: i64,
    tag_id: Option<i64>,
) -> Result<(), String> {
    with_conn(&db, |c| set_span_tag_impl(c, span_id, tag_id))
}

#[tauri::command]
pub fn get_month_log(
    db: State<DbState>,
    class_id: i64,
    year: i64,
    month: i64,
) -> Result<Vec<DayGroup>, String> {
    with_conn(&db, |c| get_month_log_impl(c, class_id, year, month))
}

#[tauri::command]
pub fn preview_bulk(
    db: State<DbState>,
    class_id: i64,
    student_id: Option<i64>,
    from: String,
    to: String,
) -> Result<Vec<BulkPreviewDay>, String> {
    with_conn(&db, |c| {
        preview_bulk_impl(c, class_id, student_id, &from, &to)
    })
}

#[tauri::command]
pub fn apply_bulk(
    db: State<DbState>,
    input: StampInput,
    from: String,
    to: String,
) -> Result<BulkApplyResult, String> {
    with_conn(&db, |c| apply_bulk_impl(c, &input, &from, &to))
}
