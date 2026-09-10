//! 나이스 가져오기.
//!
//! **교체가 아니라 차분이다.** 같은 것은 그대로 두고, 없는 것은 추가하고, 다른 것은
//! 어느 쪽을 남길지 교사가 고른다. 그리고 **앱에만 있는 것을 지우지 않는다** — 나이스에
//! 아직 안 넣은 기록이 곧 이 앱을 쓰는 이유이므로, 파일에 없다는 것이 삭제 근거가 될 수 없다.
//!
//! 경계는 명렬표 가져오기와 같다. 파일 서식은 프런트(`services/neisFile.js`)가 맡고,
//! 여기부터가 업무 규칙이다. 그래서 이 모듈은 파일이 아니라 **읽어 낸 줄**을 받는다.
//!
//! 미리보기와 적용은 **같은 차분을 두 번 계산한다.** 미리보기 결과를 서버에 담아 두지
//! 않는 이유는, 담아 두면 그 사이에 다른 화면에서 바뀐 기록을 못 본 채로 적용하기
//! 때문이다. 줄 순번(`key`)만 오가고 판단은 적용 시점의 DB로 다시 한다.

use super::attendance::{load_spans, school_settings, span_text};
use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::{self, format_date, format_korean, parse_date};
use crate::state::DbState;
use crate::types::{
    NeisDiffItem, NeisImportChoice, NeisImportPreview, NeisImportResult, NeisRowInput, SpanItem,
};
use rusqlite::{params, Connection, ToSql};
use std::collections::{HashMap, HashSet};
use tauri::State;

/// 학급과 기간으로 내 기록을 불러오는 조건. 별칭은 `load_spans`가 정한다.
const IMPORT_WHERE: &str = "WHERE st.school_id = ?1 AND st.year_id = ?2
                              AND st.grade = ?3 AND st.class_no = ?4
                              AND s.date >= ?5 AND s.date <= ?6
                            ORDER BY s.date, st.number, s.id";

/// 어느 학급의 기록과 맞출 것인가.
#[derive(Debug, Clone, Copy)]
pub struct Scope {
    pub school_id: i64,
    pub year_id: i64,
    pub grade: i64,
    pub class_no: i64,
}

/// 파일 한 줄을 DB의 식별자로 옮긴 것.
struct Resolved {
    key: usize,
    student_id: i64,
    number: i64,
    name: String,
    date: String,
    reason_id: Option<i64>,
    type_id: Option<i64>,
    start_slot: Option<String>,
    end_slot: Option<String>,
    axis: String,
    span: String,
    detail: Option<String>,
}

// ── 축 찾기 ───────────────────────────────────────────────────

/// 그날 살아 있던 구분 · 종류를 라벨로 찾는다.
///
/// **날짜로 거른다.** 코드는 마감 후 추가라, 작년에 쓰던 이름으로 적힌 파일을
/// 올해 이름으로 맞추면 과거 기록이 조용히 다른 코드가 된다.
fn axis_by_label(
    conn: &Connection,
    table: &str,
    label: &str,
    on_date: &str,
) -> Result<Option<i64>, String> {
    let sql = format!(
        "SELECT id FROM {table}
         WHERE REPLACE(label, ' ', '') = ?1
           AND valid_from <= ?2 AND (valid_to IS NULL OR ?2 < valid_to)
         ORDER BY id LIMIT 1"
    );
    conn.query_row(&sql, params![label.replace(' ', ""), on_date], |r| r.get(0))
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other.to_string()),
        })
}

/// 갈라지지 않은 표기를 `code_alias`로 한 번 더 찾아본다.
///
/// 학교마다 나이스 표기가 조금씩 다르다(`인정결석` / `출석인정결석`). 그 차이를
/// 파서에 적어 두면 학교가 늘 때마다 코드를 고쳐야 하므로 **데이터로** 흡수한다.
fn axis_by_alias(conn: &Connection, raw: &str) -> Result<Option<(i64, i64)>, String> {
    conn.query_row(
        "SELECT c.reason_id, c.type_id
           FROM code_alias a JOIN attendance_code c ON c.id = a.code_id
          WHERE REPLACE(a.raw, ' ', '') = ?1
          LIMIT 1",
        params![raw.replace(' ', "")],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other.to_string()),
    })
}

fn slot_prompt_of(conn: &Connection, type_id: Option<i64>) -> Result<Option<String>, String> {
    let Some(type_id) = type_id else {
        return Ok(None);
    };
    conn.query_row(
        "SELECT slot_prompt FROM attendance_type WHERE id = ?1",
        params![type_id],
        |r| r.get::<_, String>(0),
    )
    .map(Some)
    .map_err(|e| e.to_string())
}

fn label_of(conn: &Connection, table: &str, id: Option<i64>) -> Result<Option<String>, String> {
    let Some(id) = id else {
        return Ok(None);
    };
    let sql = format!("SELECT label FROM {table} WHERE id = ?1");
    conn.query_row(&sql, params![id], |r| r.get::<_, String>(0))
        .map(Some)
        .map_err(|e| e.to_string())
}

fn axis_text(reason: Option<&str>, kind: Option<&str>) -> String {
    format!("{} {}", reason.unwrap_or("미정"), kind.unwrap_or("미정"))
}

// ── 차분 ──────────────────────────────────────────────────────

/// 학생 번호 → 학생. 그날 재학 중인 학생만 본다(전출한 번호를 되살리지 않는다).
fn student_on(
    conn: &Connection,
    scope: Scope,
    number: i64,
    date: &str,
) -> Result<Option<(i64, String)>, String> {
    conn.query_row(
        "SELECT id, name FROM student
          WHERE school_id = ?1 AND year_id = ?2 AND grade = ?3 AND class_no = ?4
            AND number = ?5 AND enrolled_from <= ?6
            AND (enrolled_to IS NULL OR ?6 < enrolled_to)
          ORDER BY id LIMIT 1",
        params![
            scope.school_id,
            scope.year_id,
            scope.grade,
            scope.class_no,
            number,
            date
        ],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other.to_string()),
    })
}

fn unreadable(key: usize, row: &NeisRowInput, why: &str) -> NeisDiffItem {
    NeisDiffItem {
        key,
        verdict: "unreadable".to_string(),
        number: row.number,
        name: String::new(),
        date: row.date.clone(),
        date_label: row.date.clone(),
        their_axis: row.code_label.clone().unwrap_or_default(),
        their_span: String::new(),
        detail: row.detail.clone(),
        span_id: None,
        my_axis: None,
        my_span: None,
        neis_done: false,
        why: Some(why.to_string()),
    }
}

/// 파일의 줄을 DB 식별자로 옮긴다. 못 옮긴 줄은 **조용히 넘기지 않고** 그대로 돌려준다.
fn resolve(
    conn: &Connection,
    scope: Scope,
    rows: &[NeisRowInput],
) -> Result<(Vec<Resolved>, Vec<NeisDiffItem>), String> {
    let mut ok = Vec::new();
    let mut bad = Vec::new();

    for (key, row) in rows.iter().enumerate() {
        let date = match parse_date(&row.date) {
            Ok(d) => format_date(d),
            Err(_) => {
                bad.push(unreadable(key, row, "일자를 읽지 못했습니다."));
                continue;
            }
        };
        let Some((student_id, name)) = student_on(conn, scope, row.number, &date)? else {
            bad.push(unreadable(
                key,
                row,
                "그날 이 반에 없는 번호입니다. 명렬표를 먼저 맞춰주세요.",
            ));
            continue;
        };

        // 프런트가 가른 것을 먼저 쓰고, 못 갈랐으면 별칭표로 한 번 더 찾는다.
        let mut reason_id = match row.reason_label.as_deref() {
            Some(l) => axis_by_label(conn, "attendance_reason", l, &date)?,
            None => None,
        };
        let mut type_id = match row.type_label.as_deref() {
            Some(l) => axis_by_label(conn, "attendance_type", l, &date)?,
            None => None,
        };
        if reason_id.is_none() || type_id.is_none() {
            if let Some(raw) = row.code_label.as_deref() {
                if let Some((r, t)) = axis_by_alias(conn, raw)? {
                    reason_id = Some(r);
                    type_id = Some(t);
                }
            }
        }
        if reason_id.is_none() && type_id.is_none() && row.code_label.is_some() {
            bad.push(unreadable(
                key,
                row,
                &format!(
                    "모르는 출결 표기입니다: {}",
                    row.code_label.as_deref().unwrap_or("")
                ),
            ));
            continue;
        }

        let reason_label = label_of(conn, "attendance_reason", reason_id)?;
        let type_label = label_of(conn, "attendance_type", type_id)?;
        let prompt = slot_prompt_of(conn, type_id)?;
        ok.push(Resolved {
            key,
            student_id,
            number: row.number,
            name,
            axis: axis_text(reason_label.as_deref(), type_label.as_deref()),
            span: span_text(
                prompt.as_deref(),
                row.start_slot.as_deref(),
                row.end_slot.as_deref(),
            ),
            date,
            reason_id,
            type_id,
            start_slot: row.start_slot.clone(),
            end_slot: row.end_slot.clone(),
            detail: row.detail.clone(),
        });
    }
    Ok((ok, bad))
}

fn same_span(a: &Resolved, b: &SpanItem) -> bool {
    a.reason_id == b.reason_id
        && a.type_id == b.type_id
        && a.start_slot == b.start_slot
        && a.end_slot == b.end_slot
}

fn item_of(row: &Resolved, verdict: &str, mine: Option<&SpanItem>) -> NeisDiffItem {
    NeisDiffItem {
        key: row.key,
        verdict: verdict.to_string(),
        number: row.number,
        name: row.name.clone(),
        date_label: parse_date(&row.date).map(format_korean).unwrap_or_default(),
        date: row.date.clone(),
        their_axis: row.axis.clone(),
        their_span: row.span.clone(),
        detail: row.detail.clone(),
        span_id: mine.map(|m| m.id),
        my_axis: mine.map(|m| axis_text(m.reason_label.as_deref(), m.type_label.as_deref())),
        my_span: mine.map(|m| m.span_text.clone()),
        neis_done: mine.map(|m| m.neis_done).unwrap_or(false),
        why: None,
    }
}

/// 미리보기와 적용이 함께 쓰는 차분. **판정하지 않고 갈래만 나눈다.**
///
/// 짝은 (학생, 날짜)로 맞춘다. 하루 2구간이 정상이므로 **여럿을 여럿과** 비교한다 —
/// 똑같은 것끼리 먼저 짝을 지우고, 남은 것끼리 다름으로 본다. 하나씩 비교하면
/// `1교시 지각`과 `5교시 조퇴`가 서로 다름으로 잡힌다.
fn diff(
    conn: &Connection,
    scope: Scope,
    rows: &[NeisRowInput],
    today: &str,
) -> Result<NeisImportPreview, String> {
    let (resolved, mut items) = resolve(conn, scope, rows)?;

    let dates: Vec<&str> = rows.iter().map(|r| r.date.as_str()).collect();
    let from = dates.iter().min().copied().unwrap_or("").to_string();
    let to = dates.iter().max().copied().unwrap_or("").to_string();

    let mine = if from.is_empty() {
        Vec::new()
    } else {
        load_spans(
            conn,
            IMPORT_WHERE,
            &[
                &scope.school_id as &dyn ToSql,
                &scope.year_id,
                &scope.grade,
                &scope.class_no,
                &from,
                &to,
            ],
            today,
        )?
    };

    let mut buckets: HashMap<(i64, String), Vec<&SpanItem>> = HashMap::new();
    for span in &mine {
        buckets
            .entry((span.student_id, span.date.clone()))
            .or_default()
            .push(span);
    }

    let mut groups: HashMap<(i64, String), Vec<&Resolved>> = HashMap::new();
    let mut order: Vec<(i64, String)> = Vec::new();
    for row in &resolved {
        let key = (row.student_id, row.date.clone());
        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        groups.entry(key).or_default().push(row);
    }

    let mut paired: HashSet<i64> = HashSet::new();
    for key in &order {
        let theirs = &groups[key];
        let empty = Vec::new();
        let ours = buckets.get(key).unwrap_or(&empty);
        let mut taken: HashSet<i64> = HashSet::new();
        let mut leftovers = Vec::new();

        for row in theirs {
            match ours
                .iter()
                .find(|m| !taken.contains(&m.id) && same_span(row, m))
            {
                Some(hit) => {
                    taken.insert(hit.id);
                    paired.insert(hit.id);
                    items.push(item_of(row, "same", Some(hit)));
                }
                None => leftovers.push(*row),
            }
        }

        // 남은 짝을 먼저 모은다. 짝을 지으면서 같은 목록을 다시 걸러 볼 수는 없다.
        let spare: Vec<&SpanItem> = ours.iter().filter(|m| !taken.contains(&m.id)).copied().collect();
        let mut spare = spare.into_iter();
        for row in leftovers {
            match spare.next() {
                Some(hit) => {
                    paired.insert(hit.id);
                    items.push(item_of(row, "differ", Some(hit)));
                }
                None => items.push(item_of(row, "add", None)),
            }
        }
    }

    items.sort_by_key(|i| i.key);
    let count = |v: &str| items.iter().filter(|i| i.verdict == v).count() as i64;
    Ok(NeisImportPreview {
        only_mine: mine.iter().filter(|m| !paired.contains(&m.id)).count() as i64,
        same: count("same"),
        add: count("add"),
        differ: count("differ"),
        unreadable: count("unreadable"),
        items,
        from,
        to,
    })
}

// ── 적용 ──────────────────────────────────────────────────────

/// 교사가 고른 것만 적용한다. **한 트랜잭션이다** — 절반만 들어간 가져오기는 없다.
///
/// 메모는 비어 있을 때만 채운다. 나이스의 사유로 교사가 쓴 문장을 덮으면, 파일을
/// 다시 가져올 때마다 손으로 적은 말이 사라진다.
pub fn apply_neis_import_impl(
    conn: &Connection,
    scope: Scope,
    rows: &[NeisRowInput],
    choice: &NeisImportChoice,
    today: &str,
) -> Result<NeisImportResult, String> {
    let preview = diff(conn, scope, rows, today)?;
    let settings = school_settings(conn, scope.school_id)?;
    let off_days = super::attendance::off_days_of(conn, scope.school_id)?;
    let picked_add: HashSet<usize> = choice.add.iter().copied().collect();
    let picked_replace: HashSet<usize> = choice.replace.iter().copied().collect();

    with_transaction(conn, || {
        let mut added = 0;
        let mut replaced = 0;
        let mut marked = 0;

        for item in &preview.items {
            let row = rows
                .get(item.key)
                .ok_or_else(|| "미리보기와 파일이 어긋납니다.".to_string())?;
            let resolved = preview_row(conn, scope, row)?;

            match item.verdict.as_str() {
                "add" if picked_add.contains(&item.key) => {
                    let date = parse_date(&item.date)?;
                    let due = format_date(due::due_date(
                        date,
                        settings.due_days,
                        settings.due_skip_offdays,
                        &off_days,
                    ));
                    conn.execute(
                        "INSERT INTO absence_span
                           (student_id, date, reason_id, type_id, start_slot, end_slot,
                            memo, doc_due, neis_done, neis_done_on)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9)",
                        params![
                            resolved.0,
                            item.date,
                            resolved.1,
                            resolved.2,
                            row.start_slot,
                            row.end_slot,
                            row.detail.clone().unwrap_or_default(),
                            due,
                            today
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                    added += 1;
                }
                "differ" if picked_replace.contains(&item.key) => {
                    let Some(span_id) = item.span_id else { continue };
                    // 마감은 다시 계산하지 않는다. 이미 박아 둔 값이고, 소급 변경되면
                    // 교사가 학생에게 말한 날짜와 화면이 달라진다.
                    conn.execute(
                        "UPDATE absence_span
                            SET reason_id = ?1, type_id = ?2, start_slot = ?3, end_slot = ?4,
                                memo = CASE WHEN memo = '' THEN ?5 ELSE memo END,
                                neis_done = 1, neis_done_on = ?6
                          WHERE id = ?7",
                        params![
                            resolved.1,
                            resolved.2,
                            row.start_slot,
                            row.end_slot,
                            row.detail.clone().unwrap_or_default(),
                            today,
                            span_id
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                    replaced += 1;
                }
                "same" if choice.mark_neis && !item.neis_done => {
                    let Some(span_id) = item.span_id else { continue };
                    conn.execute(
                        "UPDATE absence_span SET neis_done = 1, neis_done_on = ?1 WHERE id = ?2",
                        params![today, span_id],
                    )
                    .map_err(|e| e.to_string())?;
                    marked += 1;
                }
                _ => {}
            }
        }
        Ok(NeisImportResult {
            added,
            replaced,
            marked,
        })
    })
}

/// 한 줄의 (학생 · 구분 · 종류). 적용 시점에 다시 찾는다 — 미리보기 이후에 명렬표가
/// 바뀌었을 수 있고, 그때는 넣지 않는 편이 조용히 다른 학생에게 붙는 것보다 낫다.
fn preview_row(
    conn: &Connection,
    scope: Scope,
    row: &NeisRowInput,
) -> Result<(i64, Option<i64>, Option<i64>), String> {
    let (mut ok, _) = resolve(conn, scope, std::slice::from_ref(row))?;
    let hit = ok
        .pop()
        .ok_or_else(|| format!("{}번 {} 줄을 다시 읽지 못했습니다.", row.number, row.date))?;
    Ok((hit.student_id, hit.reason_id, hit.type_id))
}

pub fn preview_neis_import_impl(
    conn: &Connection,
    scope: Scope,
    rows: &[NeisRowInput],
    today: &str,
) -> Result<NeisImportPreview, String> {
    diff(conn, scope, rows, today)
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn preview_neis_import(
    db: State<DbState>,
    school_id: i64,
    year_id: i64,
    grade: i64,
    class_no: i64,
    rows: Vec<NeisRowInput>,
    today: String,
) -> Result<NeisImportPreview, String> {
    let scope = Scope {
        school_id,
        year_id,
        grade,
        class_no,
    };
    with_conn(&db, |conn| {
        preview_neis_import_impl(conn, scope, &rows, &today)
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn apply_neis_import(
    db: State<DbState>,
    school_id: i64,
    year_id: i64,
    grade: i64,
    class_no: i64,
    rows: Vec<NeisRowInput>,
    choice: NeisImportChoice,
    today: String,
) -> Result<NeisImportResult, String> {
    let scope = Scope {
        school_id,
        year_id,
        grade,
        class_no,
    };
    with_conn(&db, |conn| {
        apply_neis_import_impl(conn, scope, &rows, &choice, &today)
    })
}
