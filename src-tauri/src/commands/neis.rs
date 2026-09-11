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

use super::attendance::{due_for, load_spans, school_settings, span_text};
use crate::commands::class::{homeroom_scope, member_by_number_on};
use crate::commands::with_conn;
use crate::db::with_transaction;
use crate::due::{format_date, format_korean, parse_date};
use crate::slots;
use crate::state::DbState;
use crate::types::{
    NeisDiffItem, NeisImportChoice, NeisImportPreview, NeisImportResult, NeisRowInput, SpanItem,
};
use rusqlite::{params, Connection, ToSql};
use std::collections::{HashMap, HashSet};
use tauri::State;

/// 학급과 기간으로 내 기록을 불러오는 조건. 별칭은 `load_spans`가 정한다.
///
/// **구간이 가리키는 학급으로 거른다.** 학적(학년 · 반)으로 거르면 반이 다른 전학생의
/// 기록이 빠지고, 같은 학생이 내 교과 강좌에도 있을 때 그쪽 기록이 섞인다.
const IMPORT_WHERE: &str = "WHERE s.class_id = ?1 AND s.date >= ?2 AND s.date <= ?3
                            ORDER BY s.date, st.number, s.id";

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
///
/// **`axis_by_label`과 같은 자로 날짜를 건다.** 코드는 마감 후 추가라, 마감한 코드에
/// 붙은 별칭이 그 뒤에도 계속 맞으면 9월 줄이 작년 코드의 두 축으로 들어간다.
/// 그 구간은 그날 살아 있던 코드와 이어지지 않아, 다 채워진 기록인데도 화면의
/// 코드 이름이 빈칸으로 나온다.
fn axis_by_alias(
    conn: &Connection,
    raw: &str,
    on_date: &str,
) -> Result<Option<(i64, i64)>, String> {
    conn.query_row(
        "SELECT c.reason_id, c.type_id
           FROM code_alias a JOIN attendance_code c ON c.id = a.code_id
          WHERE REPLACE(a.raw, ' ', '') = ?1
            AND c.valid_from <= ?2 AND (c.valid_to IS NULL OR ?2 < c.valid_to)
          ORDER BY c.id LIMIT 1",
        params![raw.replace(' ', ""), on_date],
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

/// 기간을 묻지 않는 종류(`none`, 결석)는 언제나 조회~종례다.
///
/// 앱은 결석을 조회~종례로 저장하는데(`ranges_for`), 나이스 파일에는 결시교시 칸이
/// 빈 줄이 온다. 그대로 두면 화면에 양쪽 다 "하루 종일"로 보이는 두 줄이 '다름'으로
/// 남고, 교체해 NULL이 들어가면 `stamp_span_impl`의 `find_exact`가 그 건을 찾지 못해
/// 같은 조합 재클릭(무르기)이 멎는다. 그래서 **비교도 저장도 같은 자로 맞춘다.**
fn day_slots<'a>(
    slot_prompt: Option<&str>,
    start: Option<&'a str>,
    end: Option<&'a str>,
) -> (Option<&'a str>, Option<&'a str>) {
    if slot_prompt == Some("none") {
        return (Some(slots::HOMEROOM), Some(slots::CLOSING));
    }
    (start, end)
}

/// 파일의 줄을 DB 식별자로 옮긴다. 못 옮긴 줄은 **조용히 넘기지 않고** 그대로 돌려준다.
fn resolve(
    conn: &Connection,
    class_id: i64,
    rows: &[NeisRowInput],
) -> Result<(Vec<Resolved>, Vec<NeisDiffItem>), String> {
    let scope = homeroom_scope(conn, class_id)?;
    let max_slot = school_settings(conn, scope.school_id)?.max_slot as usize;
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
        // 번호는 **내 명단에서** 찾는다. 학적(학년 · 반)으로 찾으면 반이 다른 전학생이
        // 그날 이 반에 없는 번호로 잡힌다.
        let Some((student_id, name)) = member_by_number_on(conn, scope.id, row.number, &date)?
        else {
            bad.push(unreadable(
                key,
                row,
                "그날 이 반에 없는 번호입니다. 명렬표를 먼저 맞춰주세요.",
            ));
            continue;
        };

        // 프런트가 구분한 것을 먼저 쓰고, 구분하지 못했으면 별칭표로 한 번 더 찾는다.
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
                if let Some((r, t)) = axis_by_alias(conn, raw, &date)? {
                    // **비어 있는 축만 채운다.** 라벨로 맞춘 축은 그날 목록에서 찾은
                    // 것이라 확실한데, 별칭이 두 축을 한꺼번에 덮으면 파일이 분명히
                    // 적어 둔 구분이 별칭표의 다른 구분으로 조용히 바뀐다.
                    reason_id = reason_id.or(Some(r));
                    type_id = type_id.or(Some(t));
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

        // 파일이 적어 둔 축을 하나라도 못 옮겼으면 **읽지 못한 줄이다.**
        //
        // 한쪽만 맞은 줄을 "질병 미정"으로 넣으면 파일이 분명히 말한 절반을 앱이 버린
        // 것이 되고, 교사는 무엇이 빠졌는지 화면에서 알 방법이 없다. 빈 축이 "아직 안
        // 정했다"는 뜻인 것은 교사가 직접 찍을 때이고, 파일에서 온 줄은 그 뜻이 아니다.
        // 그래서 어느 쪽을 못 찾았는지 이름을 적어 미리보기로 돌려준다.
        let missed = [
            (reason_id, "구분", row.reason_label.as_deref()),
            (type_id, "종류", row.type_label.as_deref()),
        ]
        .into_iter()
        .find_map(|(id, axis, label)| match (id, label) {
            (None, Some(label)) => Some(format!("그날 목록에 없는 {axis}입니다: {label}")),
            _ => None,
        });
        if let Some(why) = missed {
            bad.push(unreadable(key, row, &why));
            continue;
        }

        // **저장 경로는 전부 `validate_span`을 지난다.** 가져오기만 예외로 두면
        // max_slot 밖의 교시가 그대로 들어와, 그 줄은 겹침 오탐이 뜨고 이후 수정이
        // 영영 거부된다. 여기서 걸러 교사에게 이유를 보여준다.
        if let Err(why) = slots::validate_span(
            row.start_slot.as_deref(),
            row.end_slot.as_deref(),
            max_slot,
        ) {
            bad.push(unreadable(key, row, &why));
            continue;
        }

        let reason_label = label_of(conn, "attendance_reason", reason_id)?;
        let type_label = label_of(conn, "attendance_type", type_id)?;
        let prompt = slot_prompt_of(conn, type_id)?;
        let (start_slot, end_slot) = day_slots(
            prompt.as_deref(),
            row.start_slot.as_deref(),
            row.end_slot.as_deref(),
        );
        ok.push(Resolved {
            key,
            student_id,
            number: row.number,
            name,
            axis: axis_text(reason_label.as_deref(), type_label.as_deref()),
            span: span_text(prompt.as_deref(), start_slot, end_slot),
            date,
            reason_id,
            type_id,
            start_slot: start_slot.map(str::to_string),
            end_slot: end_slot.map(str::to_string),
            detail: row.detail.clone(),
        });
    }
    Ok((ok, bad))
}

fn same_span(a: &Resolved, b: &SpanItem) -> bool {
    // 파일 쪽은 `resolve`에서 이미 맞춰 두었고, 앱 쪽도 같은 자로 잰다.
    let (start, end) = day_slots(
        b.slot_prompt.as_deref(),
        b.start_slot.as_deref(),
        b.end_slot.as_deref(),
    );
    a.reason_id == b.reason_id
        && a.type_id == b.type_id
        && a.start_slot.as_deref() == start
        && a.end_slot.as_deref() == end
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
type Diff = (NeisImportPreview, HashMap<usize, Resolved>);

fn diff(
    conn: &Connection,
    class_id: i64,
    rows: &[NeisRowInput],
    today: &str,
) -> Result<Diff, String> {
    let (resolved, mut items) = resolve(conn, class_id, rows)?;

    // **자리를 채운 ISO로 잰다.** 파일이 `2026-9-1`로 주면 원본 문자열 비교에서
    // `'2026-09-01' >= '2026-9-1'`이 거짓이라 내 기록을 하나도 못 불러오고,
    // 이미 있는 기록이 전부 "앱에 없음"으로 잡혀 중복이 들어간다.
    let dates: Vec<String> = rows
        .iter()
        .filter_map(|r| parse_date(&r.date).ok().map(format_date))
        .collect();
    let from = dates.iter().min().cloned().unwrap_or_default();
    let to = dates.iter().max().cloned().unwrap_or_default();

    let mine = if from.is_empty() {
        Vec::new()
    } else {
        load_spans(
            conn,
            IMPORT_WHERE,
            &[&class_id as &dyn ToSql, &from, &to],
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

        // 남은 것끼리 짝을 짓되 **같은 종류 · 구분을 먼저 맞춘다.**
        //
        // 순서대로만 엮으면 하루에 지각과 조퇴가 하나씩 있을 때 짝이 서로 바뀐다.
        // 교사가 둘 다 "나이스 것으로"를 고르면 메모 · 태그 · 마감이 엉뚱한 기록으로
        // 넘어가는데, 그 교환은 화면에 보이지 않는다. 앱이 못 보는 곳에서 짝을
        // 정하는 것이 곧 판정이므로, 적어도 눈에 보이는 근거(종류 · 구분)를 먼저 쓴다.
        let mut spare: Vec<&SpanItem> =
            ours.iter().filter(|m| !taken.contains(&m.id)).copied().collect();
        for row in leftovers {
            let at = spare
                .iter()
                .position(|m| m.type_id == row.type_id && m.reason_id == row.reason_id)
                .or_else(|| spare.iter().position(|m| m.type_id == row.type_id))
                .or_else(|| spare.iter().position(|m| m.reason_id == row.reason_id))
                .or(if spare.is_empty() { None } else { Some(0) });
            match at.map(|at| spare.remove(at)) {
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
    let by_key: HashMap<usize, Resolved> = resolved.into_iter().map(|r| (r.key, r)).collect();
    Ok((NeisImportPreview {
        only_mine: mine.iter().filter(|m| !paired.contains(&m.id)).count() as i64,
        same: count("same"),
        add: count("add"),
        differ: count("differ"),
        unreadable: count("unreadable"),
        items,
        from,
        to,
    }, by_key))
}

// ── 적용 ──────────────────────────────────────────────────────

/// 짝지어진 내 기록의 id. `item_of`가 same · differ에는 반드시 담는다.
///
/// 그래도 꺼내는 자리에서 확인하는 이유는, 비어 있다면 차분이 어긋났다는 뜻이라
/// **조용히 넘길 일이 아니기** 때문이다. 한 건을 건너뛰면 교사는 고른 것이 왜
/// 들어가지 않았는지 알 방법이 없다.
fn paired_span(item: &NeisDiffItem) -> Result<i64, String> {
    item.span_id.ok_or_else(|| {
        format!(
            "고칠 대상을 찾지 못했습니다: {} {}번",
            item.date_label, item.number
        )
    })
}

/// 교사가 고른 것만 적용한다. **한 트랜잭션이다** — 절반만 들어간 가져오기는 없다.
///
/// 메모는 비어 있을 때만 채운다. 나이스의 사유로 교사가 쓴 문장을 덮으면, 파일을
/// 다시 가져올 때마다 손으로 적은 말이 사라진다.
pub fn apply_neis_import_impl(
    conn: &Connection,
    class_id: i64,
    rows: &[NeisRowInput],
    choice: &NeisImportChoice,
    today: &str,
) -> Result<NeisImportResult, String> {
    let scope = homeroom_scope(conn, class_id)?;
    let (preview, resolved) = diff(conn, scope.id, rows, today)?;
    let settings = school_settings(conn, scope.school_id)?;
    let off_days = super::attendance::off_days_of(conn, scope.school_id)?;
    let picked_add: HashSet<usize> = choice.add.iter().copied().collect();
    let picked_replace: HashSet<usize> = choice.replace.iter().copied().collect();

    with_transaction(conn, || {
        let mut added = 0;
        let mut replaced = 0;
        let mut marked = 0;
        let mut done: HashSet<usize> = HashSet::new();

        for item in &preview.items {
            // 못 읽은 줄은 여기 없다. **건너뛰는 것이지 실패가 아니다** — 모르는 표기
            // 한 줄 때문에 교사가 고른 나머지가 통째로 막히면 안 된다. 그 줄은 이미
            // 미리보기에서 이유와 함께 보여 주었다.
            let Some(hit) = resolved.get(&item.key) else {
                continue;
            };

            match item.verdict.as_str() {
                "add" if picked_add.contains(&item.key) => {
                    // 마감 계산은 `attendance.rs`의 것을 그대로 쓴다. 같은 규칙을 두
                    // 벌 두면 한쪽만 고쳐져, 찍어 넣은 건과 가져온 건의 마감이 갈라진다.
                    let due = due_for(parse_date(&hit.date)?, &settings, &off_days);
                    conn.execute(
                        "INSERT INTO absence_span
                           (class_id, student_id, date, reason_id, type_id, start_slot, end_slot,
                            memo, doc_due, neis_done, neis_done_on)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10)",
                        params![
                            scope.id,
                            hit.student_id,
                            hit.date,
                            hit.reason_id,
                            hit.type_id,
                            hit.start_slot,
                            hit.end_slot,
                            hit.detail.clone().unwrap_or_default(),
                            due,
                            today
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                    done.insert(item.key);
                    added += 1;
                }
                "differ" if picked_replace.contains(&item.key) => {
                    let span_id = paired_span(item)?;
                    // 마감은 다시 계산하지 않는다. 이미 박아 둔 값이고, 소급 변경되면
                    // 교사가 학생에게 말한 날짜와 화면이 달라진다. 태그와 서류도
                    // 건드리지 않는다 — 나이스가 모르는 값이다.
                    let changed = conn
                        .execute(
                            "UPDATE absence_span
                                SET reason_id = ?1, type_id = ?2, start_slot = ?3, end_slot = ?4,
                                    memo = CASE WHEN memo = '' THEN ?5 ELSE memo END,
                                    neis_done = 1, neis_done_on = ?6
                              WHERE id = ?7",
                            params![
                                hit.reason_id,
                                hit.type_id,
                                hit.start_slot,
                                hit.end_slot,
                                hit.detail.clone().unwrap_or_default(),
                                today,
                                span_id
                            ],
                        )
                        .map_err(|e| e.to_string())?;
                    if changed == 0 {
                        return Err(format!("출결 기록을 찾을 수 없습니다: {span_id}"));
                    }
                    done.insert(item.key);
                    replaced += 1;
                }
                "same" if choice.mark_neis && !item.neis_done => {
                    let span_id = paired_span(item)?;
                    let changed = conn
                        .execute(
                            "UPDATE absence_span
                                SET neis_done = 1, neis_done_on = ?1
                              WHERE id = ?2",
                            params![today, span_id],
                        )
                        .map_err(|e| e.to_string())?;
                    if changed == 0 {
                        return Err(format!("출결 기록을 찾을 수 없습니다: {span_id}"));
                    }
                    marked += 1;
                }
                _ => {}
            }
        }
        // 교사가 골랐는데 적용되지 않은 것. 미리보기 이후에 다른 화면에서 그 기록이
        // 바뀌어 판정이 달라진 경우다. **조용히 넘기지 않고 세어서 돌려준다.**
        let skipped = picked_add
            .union(&picked_replace)
            .filter(|key| !done.contains(key))
            .count() as i64;
        Ok(NeisImportResult {
            added,
            replaced,
            marked,
            skipped,
        })
    })
}

pub fn preview_neis_import_impl(
    conn: &Connection,
    class_id: i64,
    rows: &[NeisRowInput],
    today: &str,
) -> Result<NeisImportPreview, String> {
    Ok(diff(conn, class_id, rows, today)?.0)
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn preview_neis_import(
    db: State<DbState>,
    class_id: i64,
    rows: Vec<NeisRowInput>,
    today: String,
) -> Result<NeisImportPreview, String> {
    with_conn(&db, |conn| {
        preview_neis_import_impl(conn, class_id, &rows, &today)
    })
}

#[tauri::command]
pub fn apply_neis_import(
    db: State<DbState>,
    class_id: i64,
    rows: Vec<NeisRowInput>,
    choice: NeisImportChoice,
    today: String,
) -> Result<NeisImportResult, String> {
    with_conn(&db, |conn| {
        apply_neis_import_impl(conn, class_id, &rows, &choice, &today)
    })
}
