//! CSV 내보내기.
//!
//! 세 가지를 CSV **문자열**로 만든다. 파일로 쓰는 일은 하지 않는다 —
//! 저장 위치를 고르는 것은 화면의 일이고, 프런트의 `write_bytes_file`이 그 바이트를
//! 디스크에 저장한다. Rust가 경로까지 정하면 다이얼로그가 두 번 뜬다.
//!
//! **앞 세 열은 언제나 `학년,반,번호`다.** 문자 일괄 발송 시스템이 그 셋으로 수신자를
//! 찾으므로, 열 순서를 바꾸면 명단이 통째로 쓸모없어진다.
//!
//! 엑셀은 BOM 없는 UTF-8을 CP949로 읽어 한글을 전부 깨뜨린다. 그래서 맨 앞에 BOM을 붙인다.
//!
//! 값은 RFC 4180으로 인용한다. 이름에 쉼표가 들어가는 경우가 실제로 있고, 메모는
//! 자유 문장이라 따옴표와 줄바꿈이 섞여 들어온다.
//!
//! 목록을 고르고 정렬하는 규칙은 여기에 두지 않는다. 미제출 명단은 `mark`의 두
//! 함수를, 구간 조회는 `attendance::load_spans`를, 한도 집계는 `stats`를 그대로 부른다 —
//! 같은 규칙을 두 곳에 두면 화면에서 본 순서와 내보낸 파일의 순서가 갈라진다.

use crate::commands::attendance::load_spans;
use crate::commands::class::{homeroom_scope, ClassScope};
use crate::commands::mark::{get_doc_pending_impl, get_neis_pending_impl};
use crate::commands::stats::get_quota_reports_impl;
use crate::commands::with_conn;
use crate::due::{days_overdue, format_date, format_korean, parse_date};
use crate::state::DbState;
use crate::types::{QuotaReport, SpanItem};
use rusqlite::{Connection, ToSql};
use std::collections::HashMap;
use tauri::State;

/// 엑셀이 UTF-8을 알아보게 하는 BOM. 없으면 한글이 깨져 열린다.
pub const BOM: &str = "\u{feff}";

// ── CSV 조립 ──────────────────────────────────────────────────

/// RFC 4180 인용. 쉼표·따옴표·줄바꿈이 든 값만 감싼다.
pub fn csv_cell(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn csv_line(cells: &[String]) -> String {
    cells
        .iter()
        .map(|c| csv_cell(c))
        .collect::<Vec<_>>()
        .join(",")
}

/// 머리글 한 줄. 행이 하나도 없어도 이 줄은 나간다 — 빈 파일은 내보내기가 실패한
/// 것인지 대상이 없는 것인지 구분되지 않는다.
fn header(cells: &[&str]) -> String {
    let owned: Vec<String> = cells.iter().map(|c| c.to_string()).collect();
    let mut line = csv_line(&owned);
    line.push('\n');
    line
}

fn push_row(out: &mut String, cells: Vec<String>) {
    out.push_str(&csv_line(&cells));
    out.push('\n');
}

// ── 값 표기 ───────────────────────────────────────────────────

/// ISO 날짜를 화면 표기로. 형식이 깨진 값은 원문 그대로 내보낸다 —
/// 내보내기가 통째로 실패하는 것보다 교사가 그 값을 직접 보는 편이 낫다.
fn date_label(iso: &str) -> String {
    parse_date(iso)
        .map(format_korean)
        .unwrap_or_else(|_| iso.to_string())
}

fn opt_date_label(iso: Option<&str>) -> String {
    iso.map(date_label).unwrap_or_default()
}

/// 아직 안 정한 축. 빈칸으로 두면 "정하지 않았다"가 "값이 없다"와 섞인다.
fn axis_label(label: Option<&str>) -> String {
    label.unwrap_or("미정").to_string()
}

fn yes_no(flag: bool, yes: &str, no: &str) -> String {
    if flag {
        yes.to_string()
    } else {
        no.to_string()
    }
}

/// 마감까지 남은 날 · 지난 날을 사람이 읽는 문장으로.
fn overdue_text(doc_due: Option<&str>, today: &str) -> String {
    let (Some(due), Ok(today)) = (doc_due, parse_date(today)) else {
        return String::new();
    };
    let Ok(due) = parse_date(due) else {
        return String::new();
    };
    match days_overdue(due, today) {
        d if d > 0 => format!("{d}일 경과"),
        0 => "오늘 마감".to_string(),
        d => format!("{}일 남음", -d),
    }
}

fn period_label(period: &str) -> String {
    match period {
        "year" => "학년도",
        "semester" => "학기",
        "month" => "달",
        other => other,
    }
    .to_string()
}

fn unit_label(unit: &str) -> String {
    match unit {
        "day" => "일수",
        "count" => "건수",
        other => other,
    }
    .to_string()
}

fn state_label(state: &str) -> String {
    match state {
        "ok" => "여유",
        "near" => "임박",
        "over" => "초과",
        other => other,
    }
    .to_string()
}

// ── 표 만들기 ─────────────────────────────────────────────────

const SPAN_HEADER: &[&str] = &[
    "학년",
    "반",
    "번호",
    "성명",
    "날짜",
    "구분",
    "종류",
    "코드",
    "기간",
    "태그",
    "메모",
    "문구",
    "증빙",
    "증빙 마감",
    "증빙 받은 날",
    "나이스",
    "나이스 등재일",
    "겹침",
    "묶음",
];

/// 전체 기록 표. 한 행이 한 구간이다.
///
/// 미완성 기록도 빼지 않는다. 두 축이 비어 있다는 사실 자체가 교사가 봐야 할
/// 정보이고, 조용히 빼면 서른 건 중 스물여덟 건만 나온 것을 알 방법이 없다.
/// 구분 × 종류 쌍마다의 문구 패턴. 나이스 사유 칸에 붙여 넣을 초안을 만드는 데 쓴다.
///
/// 패턴은 `attendance_code`가 들고 있는 **데이터**다. 코드에 문장을 박아 두면
/// 학교마다 다른 표현을 고칠 수 없다.
pub(crate) fn phrase_patterns(
    conn: &Connection,
) -> Result<HashMap<(i64, i64), Option<String>>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT reason_id, type_id, phrase_pattern
             FROM attendance_code WHERE valid_to IS NULL",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                (r.get::<_, i64>(0)?, r.get::<_, i64>(1)?),
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 그 구간의 문구 초안. 두 축이 다 정해졌을 때만 만든다 —
/// 미완성 기록에서 억지로 문장을 만들면 나이스에 그대로 올라간다.
pub(crate) fn phrase_of(
    span: &SpanItem,
    patterns: &HashMap<(i64, i64), Option<String>>,
) -> String {
    let (Some(reason_id), Some(type_id)) = (span.reason_id, span.type_id) else {
        return String::new();
    };
    let Some(label) = span.code_label.as_deref() else {
        return String::new();
    };
    let pattern = patterns.get(&(reason_id, type_id)).and_then(|p| p.as_deref());
    crate::phrase::render(
        pattern,
        label,
        Some(span.memo.as_str()).filter(|m| !m.is_empty()),
        span.start_slot.as_deref(),
        span.end_slot.as_deref(),
    )
}

pub fn build_spans_csv(
    spans: &[SpanItem],
    patterns: &HashMap<(i64, i64), Option<String>>,
) -> String {
    let mut out = String::from(BOM);
    out.push_str(&header(SPAN_HEADER));
    for s in spans {
        push_row(
            &mut out,
            vec![
                // **학생의 학적이다.** 학급의 학년 · 반이 아니다 — 명단에 반이 다른
                // 학생이 들어올 수 있고, 문자 발송은 이 세 값으로 수신자를 찾는다.
                s.grade.to_string(),
                s.class_no.to_string(),
                s.number.to_string(),
                s.name.clone(),
                s.date_label.clone(),
                axis_label(s.reason_label.as_deref()),
                axis_label(s.type_label.as_deref()),
                s.code_label.clone().unwrap_or_default(),
                s.span_text.clone(),
                s.tag_name.clone().unwrap_or_default(),
                s.memo.clone(),
                phrase_of(s, patterns),
                yes_no(s.doc_done, "받음", "안 받음"),
                opt_date_label(s.doc_due.as_deref()),
                opt_date_label(s.doc_done_on.as_deref()),
                yes_no(s.neis_done, "등재", "미등재"),
                opt_date_label(s.neis_done_on.as_deref()),
                yes_no(s.overlapping, "겹침", ""),
                s.group_id.clone().unwrap_or_default(),
            ],
        );
    }
    out
}

const DOC_HEADER: &[&str] = &[
    "학년",
    "반",
    "번호",
    "성명",
    "날짜",
    "구분",
    "종류",
    "기간",
    "태그",
    "메모",
    "마감일",
    "경과일",
];

/// 증빙 서류 미제출 명단. 문자 발송에 그대로 넣는 표다.
///
/// **마감이 지난 것만 담지 않는다.** 마감 뒤에 재촉하는 것은 이미 늦은 일이라,
/// 여유가 있을 때 보이는 쪽이 서류를 실제로 받게 한다.
pub fn build_doc_pending_csv(
    today: &str,
    spans: &[SpanItem],
) -> String {
    let mut out = String::from(BOM);
    out.push_str(&header(DOC_HEADER));
    for s in spans {
        push_row(
            &mut out,
            vec![
                // **학생의 학적이다.** 학급의 학년 · 반이 아니다 — 명단에 반이 다른
                // 학생이 들어올 수 있고, 문자 발송은 이 세 값으로 수신자를 찾는다.
                s.grade.to_string(),
                s.class_no.to_string(),
                s.number.to_string(),
                s.name.clone(),
                s.date_label.clone(),
                axis_label(s.reason_label.as_deref()),
                axis_label(s.type_label.as_deref()),
                s.span_text.clone(),
                s.tag_name.clone().unwrap_or_default(),
                s.memo.clone(),
                opt_date_label(s.doc_due.as_deref()),
                overdue_text(s.doc_due.as_deref(), today),
            ],
        );
    }
    out
}

const NEIS_HEADER: &[&str] = &[
    "학년", "반", "번호", "성명", "날짜", "구분", "종류", "기간", "태그", "메모", "완성",
];

/// 나이스 미등재 명단. 마감이 없으므로 경과일 대신 완성 여부를 붙인다 —
/// 두 축이 비어 있는 건은 나이스에 넣을 수 없고, 그것이 미등재로 남은 이유인 경우가 많다.
pub fn build_neis_pending_csv(
    spans: &[SpanItem],
) -> String {
    let mut out = String::from(BOM);
    out.push_str(&header(NEIS_HEADER));
    for s in spans {
        push_row(
            &mut out,
            vec![
                // **학생의 학적이다.** 학급의 학년 · 반이 아니다 — 명단에 반이 다른
                // 학생이 들어올 수 있고, 문자 발송은 이 세 값으로 수신자를 찾는다.
                s.grade.to_string(),
                s.class_no.to_string(),
                s.number.to_string(),
                s.name.clone(),
                s.date_label.clone(),
                axis_label(s.reason_label.as_deref()),
                axis_label(s.type_label.as_deref()),
                s.span_text.clone(),
                s.tag_name.clone().unwrap_or_default(),
                s.memo.clone(),
                yes_no(s.complete, "완성", "미완성"),
            ],
        );
    }
    out
}

const QUOTA_HEADER: &[&str] = &[
    "학년", "반", "번호", "성명", "규정", "기간", "단위", "사용", "한도", "상태", "내역",
];

const UNTAGGED_HEADER: &[&str] = &[
    "학년", "반", "번호", "성명", "날짜", "구분", "종류", "기간", "메모",
];

/// 한도 표. 규정 하나에 대해 학생 한 명이 한 행이다.
///
/// 태그가 빠져 세지 못한 건은 조용히 넘기지 않는다. 표 아래에 빈 줄을 하나 두고
/// `태그 없는 기록` 표를 이어 붙인다 — 세지 않았다는 사실이 파일에도 남아야 한다.
pub fn build_quota_csv(
    report: &QuotaReport,
) -> String {
    let mut out = String::from(BOM);
    out.push_str(&header(QUOTA_HEADER));

    let period = period_label(&report.rule.period);
    let unit = unit_label(&report.rule.unit);
    for row in &report.rows {
        // 달 단위 규정은 달별 사용 수를, 그 밖에는 쓴 날짜를 내역에 적는다.
        let detail = if row.buckets.is_empty() {
            row.dates
                .iter()
                .map(|d| date_label(d))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            row.buckets
                .iter()
                .map(|b| format!("{} {}", b.label, b.count))
                .collect::<Vec<_>>()
                .join(", ")
        };
        push_row(
            &mut out,
            vec![
                row.grade.to_string(),
                row.class_no.to_string(),
                row.number.to_string(),
                row.name.clone(),
                report.rule.name.clone(),
                period.clone(),
                unit.clone(),
                row.used.to_string(),
                row.limit_n.to_string(),
                state_label(&row.state),
                detail,
            ],
        );
    }

    if !report.untagged.is_empty() {
        out.push('\n');
        out.push_str(&header(&["태그 없는 기록"]));
        out.push_str(&header(UNTAGGED_HEADER));
        for s in &report.untagged {
            push_row(
                &mut out,
                vec![
                    s.grade.to_string(),
                    s.class_no.to_string(),
                    s.number.to_string(),
                    s.name.clone(),
                    s.date_label.clone(),
                    axis_label(s.reason_label.as_deref()),
                    axis_label(s.type_label.as_deref()),
                    s.span_text.clone(),
                    s.memo.clone(),
                ],
            );
        }
    }
    out
}

// ── 조회 ──────────────────────────────────────────────────────

/// 기간 안의 한 학급. 명단에서 빠진 학생의 지난 기록도 그대로 나온다 —
/// 보관용 파일에서 그 학생만 빠지면 그 해의 기록이 통째로 어긋난다.
/// 그래서 구간이 가리키는 **학급**으로 거른다.
///
/// 별칭은 `attendance::load_spans`가 정한 것을 그대로 쓴다. 구간이 `s`, 학생이 `st`다.
const RANGE_SPANS: &str = "WHERE s.class_id = ?1 AND s.date >= ?2 AND s.date <= ?3
                           ORDER BY s.date, st.number, s.id";

// ── 구현 ──────────────────────────────────────────────────────

/// 기간 안의 모든 기록. 나이스에 가져오기 기능이 없으므로 이 파일은 보관용이다.
pub fn export_spans_csv_impl(
    conn: &Connection,
    class_id: i64,
    from: &str,
    to: &str,
) -> Result<String, String> {
    let start = parse_date(from)?;
    let end = parse_date(to)?;
    if start > end {
        return Err(format!("기간의 앞뒤가 바뀌었습니다: {from} ~ {to}"));
    }
    // 다시 찍어낸 ISO로 질의한다. chrono는 `2026-9-10`도 받아들이는데 저장된 값은
    // 언제나 자리를 채운 형식이라, 그대로 넣으면 아무것도 걸리지 않는다.
    let (from, to) = (format_date(start), format_date(end));
    let scope: ClassScope = homeroom_scope(conn, class_id)?;

    let params: [&dyn ToSql; 3] = [&scope.id, &from, &to];
    // 경과일을 쓰지 않는 표라 기준일은 기간의 끝으로 넘긴다.
    let spans = load_spans(conn, RANGE_SPANS, &params, &to)?;
    let patterns = phrase_patterns(conn)?;
    Ok(build_spans_csv(
        &spans,
        &patterns,
    ))
}

/// 미제출 명단. `kind`는 `doc`(증빙 서류) 또는 `neis`(나이스 등재)다.
///
/// 두 목록을 한 커맨드에 담은 이유는 화면이 같은 자리의 버튼 하나로 내보내기
/// 때문이다. 고르는 기준과 정렬은 각 화면이 쓰는 함수를 그대로 부른다 — 화면에서
/// 본 순서와 파일의 순서가 같아야 교사가 둘을 대조할 수 있다.
pub fn export_pending_csv_impl(
    conn: &Connection,
    class_id: i64,
    kind: &str,
    today: &str,
) -> Result<String, String> {
    // 자리를 채운 ISO로 맞춰 넘긴다. chrono는 `2026-9-10`도 받아들이는데, 날짜 비교가
    // 문자열 비교인 자리에서 그 값은 `2026-09-01`보다 앞선 것으로 읽힌다.
    let today = &format_date(parse_date(today)?);
    let scope = homeroom_scope(conn, class_id)?;
    match kind {
        "doc" => {
            let rows = get_doc_pending_impl(conn, scope.id, None, None, false, today)?;
            Ok(build_doc_pending_csv(
                today,
                &rows,
            ))
        }
        "neis" => {
            let groups = get_neis_pending_impl(conn, scope.id, None, None, today)?;
            let rows: Vec<SpanItem> = groups.into_iter().flat_map(|g| g.spans).collect();
            Ok(build_neis_pending_csv(&rows))
        }
        other => Err(format!("알 수 없는 명단 종류입니다: {other}")),
    }
}

/// 한도 규정 하나의 집계. 세는 일은 `stats`가 하고 여기서는 표로만 옮긴다.
pub fn export_quota_csv_impl(
    conn: &Connection,
    class_id: i64,
    rule_id: i64,
) -> Result<String, String> {
    let scope = homeroom_scope(conn, class_id)?;
    let reports = get_quota_reports_impl(conn, scope.id, Some(rule_id), None, None)?;
    let report = reports
        .into_iter()
        .find(|r| r.rule.id == rule_id)
        .ok_or_else(|| format!("한도 규정을 찾을 수 없습니다: {rule_id}"))?;
    Ok(build_quota_csv(&report))
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn export_spans_csv(
    db: State<DbState>,
    class_id: i64,
    from: String,
    to: String,
) -> Result<String, String> {
    with_conn(&db, |c| export_spans_csv_impl(c, class_id, &from, &to))
}

#[tauri::command]
pub fn export_pending_csv(
    db: State<DbState>,
    class_id: i64,
    kind: String,
    today: String,
) -> Result<String, String> {
    with_conn(&db, |c| export_pending_csv_impl(c, class_id, &kind, &today))
}

#[tauri::command]
pub fn export_quota_csv(
    db: State<DbState>,
    class_id: i64,
    rule_id: i64,
) -> Result<String, String> {
    with_conn(&db, |c| export_quota_csv_impl(c, class_id, rule_id))
}
