//! 일일 출결 입력의 핵심. 구간(absence_span)과 사유(daily_reason)를 다룬다.
//!
//! 트랜잭션 규칙: `*_inner` 함수는 트랜잭션을 열지 않는다. 여러 건을 한 묶음으로
//! 저장하는 기간 일괄 입력이 이들을 반복 호출하기 때문이다. 트랜잭션은 커맨드가
//! 부르는 `*_impl` 한 겹에서만 연다. 안쪽에서 또 열면 BEGIN이 실패하고, 그
//! 실패가 조용히 무시되면 트랜잭션이 열린 채 세션에 남는다(db.rs 참고).

use crate::commands::check::ensure_daily_checks_inner;
use crate::commands::code::get_code_impl;
use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::{format_date, parse_date, weekdays_between};
use crate::phrase;
use crate::slots::{format_span, validate_span};
use crate::state::DbState;
use crate::types::*;
use rusqlite::Connection;
use tauri::State;
use uuid::Uuid;

// ── 조회 ──────────────────────────────────────────────────────

const SPAN_SELECT: &str = "SELECT s.id, s.student_id, s.date, s.code_id, c.label, c.type,
                                  s.start_slot, s.end_slot, s.symptom, s.group_id
                           FROM absence_span s
                           JOIN attendance_code c ON c.id = s.code_id";

fn map_span(row: &rusqlite::Row) -> rusqlite::Result<SpanItem> {
    let start_slot: Option<String> = row.get(6)?;
    let end_slot: Option<String> = row.get(7)?;
    let span_text = format_span(start_slot.as_deref(), end_slot.as_deref());
    Ok(SpanItem {
        id: row.get(0)?,
        student_id: row.get(1)?,
        date: row.get(2)?,
        code_id: row.get(3)?,
        code_label: row.get(4)?,
        code_type: row.get(5)?,
        start_slot,
        end_slot,
        symptom: row.get(8)?,
        group_id: row.get(9)?,
        span_text,
    })
}

pub fn get_spans_on_impl(conn: &Connection, date: &str) -> Result<Vec<SpanItem>, String> {
    let sql = format!("{SPAN_SELECT} WHERE s.date = ?1 ORDER BY s.student_id, s.id");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![date], map_span)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 하루치 격자. 재학 중인 학생 전원이 행으로 나온다.
///
/// 30행을 항상 띄우고 빈 행은 출석을 뜻한다. "추가" 버튼이 없는 이유다.
pub fn get_day_grid_impl(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
    date: &str,
) -> Result<DayGrid, String> {
    let students =
        crate::commands::student::get_students_on_impl(conn, year_id, grade, class_no, date)?;
    let spans = get_spans_on_impl(conn, date)?;
    let items = crate::commands::check::get_check_items_impl(conn, true)?;

    let mut rows = Vec::with_capacity(students.len());
    for s in students {
        let student_spans: Vec<SpanItem> =
            spans.iter().filter(|sp| sp.student_id == s.id).map(clone_span).collect();

        let reason = conn
            .query_row(
                "SELECT code_id, reason FROM daily_reason WHERE student_id = ?1 AND date = ?2",
                rusqlite::params![s.id, date],
                |r| {
                    Ok(DailyReasonItem {
                        code_id: r.get(0)?,
                        reason: r.get(1)?,
                    })
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.to_string()),
            })?;

        let mut stmt = conn
            .prepare(
                "SELECT item_id, done, due_date, done_at FROM daily_check
                 WHERE student_id = ?1 AND date = ?2",
            )
            .map_err(|e| e.to_string())?;
        let checks = stmt
            .query_map(rusqlite::params![s.id, date], |r| {
                Ok(DailyCheckItem {
                    item_id: r.get(0)?,
                    done: r.get::<_, i64>(1)? != 0,
                    due_date: r.get(2)?,
                    done_at: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        rows.push(DayRow {
            student_id: s.id,
            number: s.number,
            name: s.name,
            spans: student_spans,
            reason,
            checks,
        });
    }

    Ok(DayGrid {
        date: date.to_string(),
        rows,
        items,
    })
}

fn clone_span(s: &SpanItem) -> SpanItem {
    SpanItem {
        id: s.id,
        student_id: s.student_id,
        date: s.date.clone(),
        code_id: s.code_id,
        code_label: s.code_label.clone(),
        code_type: s.code_type.clone(),
        start_slot: s.start_slot.clone(),
        end_slot: s.end_slot.clone(),
        symptom: s.symptom.clone(),
        group_id: s.group_id.clone(),
        span_text: s.span_text.clone(),
    }
}

// ── 문구 ──────────────────────────────────────────────────────

/// 문구 초안 생성. 프론트엔드가 아니라 여기서 만든다.
pub fn render_phrase_impl(
    conn: &Connection,
    code_id: i64,
    symptom: Option<&str>,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
) -> Result<String, String> {
    let code = get_code_impl(conn, code_id)?;
    Ok(phrase::render(
        code.phrase_pattern.as_deref(),
        &code.label,
        symptom,
        start_slot,
        end_slot,
    ))
}

// ── 구간 저장 ─────────────────────────────────────────────────

/// 트랜잭션을 열지 않는 저장 본체.
#[allow(clippy::too_many_arguments)]
pub fn add_span_inner(
    conn: &Connection,
    student_id: i64,
    date: &str,
    code_id: i64,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
    symptom: Option<&str>,
    group_id: Option<&str>,
    due_base: &str,
) -> Result<i64, String> {
    validate_span(start_slot, end_slot)?;
    parse_date(date)?;

    conn.execute(
        "INSERT INTO absence_span
           (student_id, date, code_id, start_slot, end_slot, symptom, group_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![student_id, date, code_id, start_slot, end_slot, symptom, group_id],
    )
    .map_err(|e| crate::state::constraint_err(&e, "같은 구간이 이미 있습니다."))?;
    let span_id = conn.last_insert_rowid();

    // 사유가 아직 없으면 초안을 만들어 둔다. 이미 있으면 건드리지 않는다 —
    // 교사가 고친 문구를 두 번째 구간 입력이 덮어쓰면 안 된다.
    let has_reason: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daily_reason WHERE student_id = ?1 AND date = ?2",
            rusqlite::params![student_id, date],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if has_reason == 0 {
        let draft = render_phrase_impl(conn, code_id, symptom, start_slot, end_slot)?;
        conn.execute(
            "INSERT INTO daily_reason (student_id, date, code_id, reason)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![student_id, date, code_id, draft],
        )
        .map_err(|e| e.to_string())?;
    }

    ensure_daily_checks_inner(conn, student_id, date, due_base)?;
    Ok(span_id)
}

#[allow(clippy::too_many_arguments)]
pub fn add_span_impl(
    conn: &Connection,
    student_id: i64,
    date: &str,
    code_id: i64,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
    symptom: Option<&str>,
) -> Result<i64, String> {
    with_transaction(conn, || {
        add_span_inner(
            conn, student_id, date, code_id, start_slot, end_slot, symptom, None, date,
        )
    })
}

#[allow(clippy::too_many_arguments)]
pub fn update_span_impl(
    conn: &Connection,
    id: i64,
    code_id: i64,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
    symptom: Option<&str>,
) -> Result<(), String> {
    validate_span(start_slot, end_slot)?;
    conn.execute(
        "UPDATE absence_span
         SET code_id = ?1, start_slot = ?2, end_slot = ?3, symptom = ?4
         WHERE id = ?5",
        rusqlite::params![code_id, start_slot, end_slot, symptom, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 구간 삭제. 그날 마지막 구간이었다면 사유와 체크도 함께 지운다.
///
/// **빈 행은 출석을 뜻한다.** 구간을 지웠는데 체크 행이 남으면 미제출자 목록에
/// 사라진 결석의 서류가 계속 뜬다.
pub fn delete_span_impl(conn: &Connection, id: i64) -> Result<(), String> {
    with_transaction(conn, || {
        let target: Option<(i64, String)> = conn
            .query_row(
                "SELECT student_id, date FROM absence_span WHERE id = ?1",
                rusqlite::params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.to_string()),
            })?;

        let Some((student_id, date)) = target else {
            return Ok(()); // 이미 없다. 조용히 성공으로 둔다.
        };

        conn.execute("DELETE FROM absence_span WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;

        let remaining: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM absence_span WHERE student_id = ?1 AND date = ?2",
                rusqlite::params![student_id, date],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        if remaining == 0 {
            conn.execute(
                "DELETE FROM daily_reason WHERE student_id = ?1 AND date = ?2",
                rusqlite::params![student_id, date],
            )
            .map_err(|e| e.to_string())?;
            conn.execute(
                "DELETE FROM daily_check WHERE student_id = ?1 AND date = ?2",
                rusqlite::params![student_id, date],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
}

/// 나이스에 낼 한 줄. 교사가 고치면 그대로 남는다.
pub fn set_daily_reason_impl(
    conn: &Connection,
    student_id: i64,
    date: &str,
    code_id: Option<i64>,
    reason: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO daily_reason (student_id, date, code_id, reason)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(student_id, date) DO UPDATE
           SET code_id = excluded.code_id, reason = excluded.reason",
        rusqlite::params![student_id, date, code_id, reason],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ── 어제 것 그대로 ────────────────────────────────────────────

/// 그 학생의 직전 기록일을 찾아 구간과 사유를 그대로 복사한다.
///
/// 연속 결석 학생은 이것 하나로 끝난다. 7월 실데이터에서 한 학생이 3주 넘게 거의
/// 매일 같은 사유로 기록되어 있었다. 그 경우 매일 코드·구간·증상을 다시 치는 것은
/// 5초 기준을 지킬 수 없다.
pub fn copy_previous_impl(
    conn: &Connection,
    student_id: i64,
    date: &str,
) -> Result<i64, String> {
    with_transaction(conn, || {
        let prev: Option<String> = conn
            .query_row(
                "SELECT MAX(date) FROM absence_span WHERE student_id = ?1 AND date < ?2",
                rusqlite::params![student_id, date],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;

        let Some(prev) = prev else {
            return Err("복사할 이전 기록이 없습니다.".to_string());
        };

        let existing: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM absence_span WHERE student_id = ?1 AND date = ?2",
                rusqlite::params![student_id, date],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if existing > 0 {
            return Err("이미 그날 기록이 있습니다. 지우고 다시 시도해주세요.".to_string());
        }

        let sql = format!("{SPAN_SELECT} WHERE s.student_id = ?1 AND s.date = ?2 ORDER BY s.id");
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let spans = stmt
            .query_map(rusqlite::params![student_id, prev], map_span)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut copied = 0;
        for sp in &spans {
            // group_id는 복사하지 않는다. 어제의 체험학습 묶음에 오늘이 끼면
            // 그 묶음의 서류 한 장이 오늘까지 덮는 것으로 보이게 된다.
            add_span_inner(
                conn,
                student_id,
                date,
                sp.code_id,
                sp.start_slot.as_deref(),
                sp.end_slot.as_deref(),
                sp.symptom.as_deref(),
                None,
                date,
            )?;
            copied += 1;
        }

        // 사유도 그대로 가져온다. 교사가 고쳐 둔 문구가 있다면 그것이 정답이다.
        let prev_reason: Option<(Option<i64>, String)> = conn
            .query_row(
                "SELECT code_id, reason FROM daily_reason WHERE student_id = ?1 AND date = ?2",
                rusqlite::params![student_id, prev],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.to_string()),
            })?;
        if let Some((code_id, reason)) = prev_reason {
            set_daily_reason_impl(conn, student_id, date, code_id, &reason)?;
        }

        Ok(copied)
    })
}

// ── 기간 일괄 입력 ────────────────────────────────────────────

/// 주말만 뺀 후보 날짜. 재량휴업일·공휴일은 교사가 미리보기에서 지운다.
pub fn bulk_preview_impl(
    conn: &Connection,
    student_ids: &[i64],
    from: &str,
    to: &str,
) -> Result<Vec<BulkPreviewDay>, String> {
    let from_d = parse_date(from)?;
    let to_d = parse_date(to)?;
    if to_d < from_d {
        return Err("끝 날짜가 시작 날짜보다 앞입니다.".to_string());
    }

    let mut out = Vec::new();
    for d in weekdays_between(from_d, to_d) {
        let iso = format_date(d);
        let mut has_existing = false;
        for sid in student_ids {
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM absence_span WHERE student_id = ?1 AND date = ?2",
                    rusqlite::params![sid, iso],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if n > 0 {
                has_existing = true;
                break;
            }
        }
        out.push(BulkPreviewDay {
            label: crate::due::format_korean(d),
            date: iso,
            has_existing,
        });
    }
    Ok(out)
}

/// 미리보기에서 교사가 확정한 날짜에만 저장한다.
///
/// 마감일은 **그룹의 마지막 날 기준으로 전 건에 동일하게** 넣는다. 서류 한 장이
/// 기간 전체를 덮는 실무와 맞고, 날짜마다 다른 마감일이 뜨면 목록이 시끄러워진다.
#[allow(clippy::too_many_arguments)]
pub fn bulk_apply_impl(
    conn: &Connection,
    student_ids: &[i64],
    dates: &[String],
    code_id: i64,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
    symptom: Option<&str>,
) -> Result<BulkApplyResult, String> {
    if student_ids.is_empty() {
        return Err("학생을 선택해주세요.".to_string());
    }
    if dates.is_empty() {
        return Err("날짜를 선택해주세요.".to_string());
    }
    let mut sorted = dates.to_vec();
    sorted.sort();
    let due_base = sorted.last().unwrap().clone();

    let group_id = Uuid::new_v4().to_string();

    with_transaction(conn, || {
        for sid in student_ids {
            for date in &sorted {
                add_span_inner(
                    conn,
                    *sid,
                    date,
                    code_id,
                    start_slot,
                    end_slot,
                    symptom,
                    Some(&group_id),
                    &due_base,
                )?;
            }
        }
        Ok(())
    })?;

    Ok(BulkApplyResult {
        group_id,
        days: sorted.len() as i64,
    })
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_day_grid(
    db: State<DbState>,
    year_id: i64,
    grade: i64,
    class_no: i64,
    date: String,
) -> Result<DayGrid, String> {
    with_conn(&db, |c| get_day_grid_impl(c, year_id, grade, class_no, &date))
}

#[tauri::command]
pub fn render_phrase(
    db: State<DbState>,
    code_id: i64,
    symptom: Option<String>,
    start_slot: Option<String>,
    end_slot: Option<String>,
) -> Result<String, String> {
    with_conn(&db, |c| {
        render_phrase_impl(
            c,
            code_id,
            symptom.as_deref(),
            start_slot.as_deref(),
            end_slot.as_deref(),
        )
    })
}

#[tauri::command]
pub fn add_span(
    db: State<DbState>,
    student_id: i64,
    date: String,
    code_id: i64,
    start_slot: Option<String>,
    end_slot: Option<String>,
    symptom: Option<String>,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        add_span_impl(
            c,
            student_id,
            &date,
            code_id,
            start_slot.as_deref(),
            end_slot.as_deref(),
            symptom.as_deref(),
        )
    })
}

#[tauri::command]
pub fn update_span(
    db: State<DbState>,
    id: i64,
    code_id: i64,
    start_slot: Option<String>,
    end_slot: Option<String>,
    symptom: Option<String>,
) -> Result<(), String> {
    with_conn(&db, |c| {
        update_span_impl(
            c,
            id,
            code_id,
            start_slot.as_deref(),
            end_slot.as_deref(),
            symptom.as_deref(),
        )
    })
}

#[tauri::command]
pub fn delete_span(db: State<DbState>, id: i64) -> Result<(), String> {
    with_conn(&db, |c| delete_span_impl(c, id))
}

#[tauri::command]
pub fn set_daily_reason(
    db: State<DbState>,
    student_id: i64,
    date: String,
    code_id: Option<i64>,
    reason: String,
) -> Result<(), String> {
    with_conn(&db, |c| {
        set_daily_reason_impl(c, student_id, &date, code_id, &reason)
    })
}

#[tauri::command]
pub fn copy_previous(db: State<DbState>, student_id: i64, date: String) -> Result<i64, String> {
    with_conn(&db, |c| copy_previous_impl(c, student_id, &date))
}

#[tauri::command]
pub fn bulk_preview(
    db: State<DbState>,
    student_ids: Vec<i64>,
    from: String,
    to: String,
) -> Result<Vec<BulkPreviewDay>, String> {
    with_conn(&db, |c| bulk_preview_impl(c, &student_ids, &from, &to))
}

#[tauri::command]
pub fn bulk_apply(
    db: State<DbState>,
    student_ids: Vec<i64>,
    dates: Vec<String>,
    code_id: i64,
    start_slot: Option<String>,
    end_slot: Option<String>,
    symptom: Option<String>,
) -> Result<BulkApplyResult, String> {
    with_conn(&db, |c| {
        bulk_apply_impl(
            c,
            &student_ids,
            &dates,
            code_id,
            start_slot.as_deref(),
            end_slot.as_deref(),
            symptom.as_deref(),
        )
    })
}
