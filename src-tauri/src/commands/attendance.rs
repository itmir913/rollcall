//! 일일 출결 입력의 핵심. 구간(absence_span)과 사유(daily_reason)를 다룬다.
//!
//! **미완성 기록이 정상 상태다.** `reason_id`와 `type_id`는 각각 비어 있을 수 있고,
//! 비어 있으면 "아직 안 정했다"는 뜻이다. 학생이 안 왔는데 연락이 닿지 않으면 그날은
//! 날짜와 학생만 남는다. 웹앱이 결석 번호만 넘겨주는 경로도 같은 모양으로 들어온다.
//! 그래서 저장 함수는 어느 축이 비었다고 거절하지 않는다.
//!
//! 트랜잭션 규칙: `*_inner` 함수는 트랜잭션을 열지 않는다. 여러 건을 한 묶음으로
//! 저장하는 경로(도장 찍기, 기간 일괄 입력)가 이들을 반복 호출하기 때문이다.
//! 트랜잭션은 커맨드가 부르는 `*_impl` 한 겹에서만 연다. 안쪽에서 또 열면 BEGIN이
//! 실패하고, 그 실패가 조용히 무시되면 트랜잭션이 열린 채 세션에 남는다(db.rs 참고).

use crate::commands::axis::find_code_impl;
use crate::commands::check::ensure_daily_checks_inner;
use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::{format_date, format_korean, parse_date, weekdays_between};
use crate::phrase;
use crate::slots::{format_span, validate_span};
use crate::state::DbState;
use crate::types::*;
use rusqlite::Connection;
use tauri::State;
use uuid::Uuid;

// ── 조회 ──────────────────────────────────────────────────────

/// 코드는 두 축이 다 채워졌을 때만 붙는다. LEFT JOIN인 이유다.
const SPAN_SELECT: &str = "SELECT s.id, s.student_id, s.date, s.reason_id, s.type_id,
                                  r.label, t.label, c.label,
                                  s.start_slot, s.end_slot, s.symptom, s.group_id
                           FROM absence_span s
                           LEFT JOIN attendance_reason r ON r.id = s.reason_id
                           LEFT JOIN attendance_type t ON t.id = s.type_id
                           LEFT JOIN attendance_code c
                                  ON c.reason_id = s.reason_id
                                 AND c.type_id = s.type_id
                                 AND c.valid_from <= s.date
                                 AND (c.valid_to IS NULL OR s.date < c.valid_to)";

fn map_span(row: &rusqlite::Row) -> rusqlite::Result<SpanItem> {
    let reason_id: Option<i64> = row.get(3)?;
    let type_id: Option<i64> = row.get(4)?;
    let start_slot: Option<String> = row.get(8)?;
    let end_slot: Option<String> = row.get(9)?;
    Ok(SpanItem {
        id: row.get(0)?,
        student_id: row.get(1)?,
        date: row.get(2)?,
        reason_id,
        type_id,
        reason_label: row.get(5)?,
        type_label: row.get(6)?,
        code_label: row.get(7)?,
        span_text: format_span(start_slot.as_deref(), end_slot.as_deref()),
        start_slot,
        end_slot,
        symptom: row.get(10)?,
        group_id: row.get(11)?,
        complete: reason_id.is_some() && type_id.is_some(),
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
/// 빈 행은 출석을 뜻하며 저장되지 않는다.
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
            spans.iter().filter(|sp| sp.student_id == s.id).cloned().collect();

        let reason = conn
            .query_row(
                "SELECT reason_id, type_id, reason FROM daily_reason
                 WHERE student_id = ?1 AND date = ?2",
                rusqlite::params![s.id, date],
                |r| {
                    Ok(DailyReasonItem {
                        reason_id: r.get(0)?,
                        type_id: r.get(1)?,
                        reason: r.get(2)?,
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

// ── 문구 ──────────────────────────────────────────────────────

/// 문구 초안. 두 축이 다 정해져야 만들 수 있다 — 아니면 빈 문자열이다.
///
/// 프론트엔드가 아니라 여기서 만든다. 같은 규칙을 양쪽에 두면 미리보기와 저장값이
/// 갈라진다.
pub fn render_phrase_impl(
    conn: &Connection,
    reason_id: Option<i64>,
    type_id: Option<i64>,
    symptom: Option<&str>,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
    on_date: Option<&str>,
) -> Result<String, String> {
    let Some(code) = find_code_impl(conn, reason_id, type_id, on_date)? else {
        return Ok(String::new());
    };
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
///
/// 두 축이 비어 있어도 저장한다. 그것이 "안 왔는데 연락이 안 됨" 상태다.
#[allow(clippy::too_many_arguments)]
pub fn add_span_inner(
    conn: &Connection,
    student_id: i64,
    date: &str,
    reason_id: Option<i64>,
    type_id: Option<i64>,
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
           (student_id, date, reason_id, type_id, start_slot, end_slot, symptom, group_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            student_id, date, reason_id, type_id, start_slot, end_slot, symptom, group_id
        ],
    )
    .map_err(|e| crate::state::constraint_err(&e, "구간을 저장하지 못했습니다."))?;
    let span_id = conn.last_insert_rowid();

    sync_reason_draft_inner(conn, student_id, date, reason_id, type_id, symptom, start_slot, end_slot)?;
    ensure_daily_checks_inner(conn, student_id, date, due_base)?;
    Ok(span_id)
}

/// 사유 초안을 손본다.
///
/// 이미 사유 행이 있으면 **건드리지 않는다.** 교사가 고친 문구를 두 번째 구간
/// 입력이나 축 채우기가 덮어쓰면 안 되기 때문이다. 두 축이 아직 안 정해져
/// 문구를 만들 수 없으면 행을 만들지 않는다 — 빈 사유 행은 나이스에 낼 것이
/// 있는 것처럼 보이게 만든다.
#[allow(clippy::too_many_arguments)]
fn sync_reason_draft_inner(
    conn: &Connection,
    student_id: i64,
    date: &str,
    reason_id: Option<i64>,
    type_id: Option<i64>,
    symptom: Option<&str>,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
) -> Result<(), String> {
    let has_reason: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daily_reason WHERE student_id = ?1 AND date = ?2",
            rusqlite::params![student_id, date],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if has_reason > 0 {
        return Ok(());
    }

    let draft = render_phrase_impl(
        conn, reason_id, type_id, symptom, start_slot, end_slot, Some(date),
    )?;
    if draft.is_empty() {
        return Ok(());
    }

    conn.execute(
        "INSERT INTO daily_reason (student_id, date, reason_id, type_id, reason)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![student_id, date, reason_id, type_id, draft],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 여러 학생에게 같은 출결을 한 번에 찍는다.
///
/// 화면의 입력 방식이 이렇다 — 구분과 종류를 고른 뒤 학생을 눌러 나간다.
/// 학생 한 명을 누를 때마다 목록 하나짜리로 불러도 되고, 웹앱이 넘겨준 결석
/// 번호 묶음을 두 축 없이(`None`, `None`) 한꺼번에 넣을 때도 같은 경로다.
#[allow(clippy::too_many_arguments)]
pub fn add_spans_impl(
    conn: &Connection,
    student_ids: &[i64],
    date: &str,
    reason_id: Option<i64>,
    type_id: Option<i64>,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
    symptom: Option<&str>,
) -> Result<i64, String> {
    if student_ids.is_empty() {
        return Err("학생을 선택해주세요.".to_string());
    }
    with_transaction(conn, || {
        let mut n = 0;
        for sid in student_ids {
            add_span_inner(
                conn, *sid, date, reason_id, type_id, start_slot, end_slot, symptom, None, date,
            )?;
            n += 1;
        }
        Ok(n)
    })
}

#[allow(clippy::too_many_arguments)]
pub fn update_span_impl(
    conn: &Connection,
    id: i64,
    reason_id: Option<i64>,
    type_id: Option<i64>,
    start_slot: Option<&str>,
    end_slot: Option<&str>,
    symptom: Option<&str>,
) -> Result<(), String> {
    validate_span(start_slot, end_slot)?;
    with_transaction(conn, || {
        conn.execute(
            "UPDATE absence_span
             SET reason_id = ?1, type_id = ?2, start_slot = ?3, end_slot = ?4, symptom = ?5
             WHERE id = ?6",
            rusqlite::params![reason_id, type_id, start_slot, end_slot, symptom, id],
        )
        .map_err(|e| e.to_string())?;

        // 미완성이던 기록의 축을 이제 채웠다면 사유 초안을 만들어 준다.
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
        if let Some((student_id, date)) = target {
            sync_reason_draft_inner(
                conn, student_id, &date, reason_id, type_id, symptom, start_slot, end_slot,
            )?;
        }
        Ok(())
    })
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
    reason_id: Option<i64>,
    type_id: Option<i64>,
    reason: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO daily_reason (student_id, date, reason_id, type_id, reason)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(student_id, date) DO UPDATE
           SET reason_id = excluded.reason_id,
               type_id = excluded.type_id,
               reason = excluded.reason",
        rusqlite::params![student_id, date, reason_id, type_id, reason],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 아직 두 축이 다 채워지지 않은 구간들. 채워 넣어야 할 목록이다.
pub fn get_incomplete_impl(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
) -> Result<Vec<SpanItem>, String> {
    let sql = format!(
        "{SPAN_SELECT}
         JOIN student st ON st.id = s.student_id
         WHERE (s.reason_id IS NULL OR s.type_id IS NULL)
           AND st.year_id = ?1 AND st.grade = ?2 AND st.class_no = ?3
         ORDER BY s.date DESC, st.number"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![year_id, grade, class_no], map_span)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

// ── 어제 것 그대로 ────────────────────────────────────────────

/// 그 학생의 직전 기록일을 찾아 구간과 사유를 그대로 복사한다.
///
/// 연속 결석 학생은 이것 하나로 끝난다. 7월 실데이터에서 한 학생이 3주 넘게 거의
/// 매일 같은 사유로 기록되어 있었다.
pub fn copy_previous_impl(conn: &Connection, student_id: i64, date: &str) -> Result<i64, String> {
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
                sp.reason_id,
                sp.type_id,
                sp.start_slot.as_deref(),
                sp.end_slot.as_deref(),
                sp.symptom.as_deref(),
                None,
                date,
            )?;
            copied += 1;
        }

        // 사유도 그대로 가져온다. 교사가 고쳐 둔 문구가 있다면 그것이 정답이다.
        let prev_reason: Option<(Option<i64>, Option<i64>, String)> = conn
            .query_row(
                "SELECT reason_id, type_id, reason FROM daily_reason
                 WHERE student_id = ?1 AND date = ?2",
                rusqlite::params![student_id, prev],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other.to_string()),
            })?;
        if let Some((reason_id, type_id, reason)) = prev_reason {
            set_daily_reason_impl(conn, student_id, date, reason_id, type_id, &reason)?;
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
            label: format_korean(d),
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
    reason_id: Option<i64>,
    type_id: Option<i64>,
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
                    reason_id,
                    type_id,
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
    reason_id: Option<i64>,
    type_id: Option<i64>,
    symptom: Option<String>,
    start_slot: Option<String>,
    end_slot: Option<String>,
    on_date: Option<String>,
) -> Result<String, String> {
    with_conn(&db, |c| {
        render_phrase_impl(
            c,
            reason_id,
            type_id,
            symptom.as_deref(),
            start_slot.as_deref(),
            end_slot.as_deref(),
            on_date.as_deref(),
        )
    })
}

#[tauri::command]
pub fn add_spans(
    db: State<DbState>,
    student_ids: Vec<i64>,
    date: String,
    reason_id: Option<i64>,
    type_id: Option<i64>,
    start_slot: Option<String>,
    end_slot: Option<String>,
    symptom: Option<String>,
) -> Result<i64, String> {
    with_conn(&db, |c| {
        add_spans_impl(
            c,
            &student_ids,
            &date,
            reason_id,
            type_id,
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
    reason_id: Option<i64>,
    type_id: Option<i64>,
    start_slot: Option<String>,
    end_slot: Option<String>,
    symptom: Option<String>,
) -> Result<(), String> {
    with_conn(&db, |c| {
        update_span_impl(
            c,
            id,
            reason_id,
            type_id,
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
    reason_id: Option<i64>,
    type_id: Option<i64>,
    reason: String,
) -> Result<(), String> {
    with_conn(&db, |c| {
        set_daily_reason_impl(c, student_id, &date, reason_id, type_id, &reason)
    })
}

#[tauri::command]
pub fn get_incomplete(
    db: State<DbState>,
    year_id: i64,
    grade: i64,
    class_no: i64,
) -> Result<Vec<SpanItem>, String> {
    with_conn(&db, |c| get_incomplete_impl(c, year_id, grade, class_no))
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
    reason_id: Option<i64>,
    type_id: Option<i64>,
    start_slot: Option<String>,
    end_slot: Option<String>,
    symptom: Option<String>,
) -> Result<BulkApplyResult, String> {
    with_conn(&db, |c| {
        bulk_apply_impl(
            c,
            &student_ids,
            &dates,
            reason_id,
            type_id,
            start_slot.as_deref(),
            end_slot.as_deref(),
            symptom.as_deref(),
        )
    })
}
