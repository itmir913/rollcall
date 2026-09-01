//! CSV 내보내기.
//!
//! 나이스에 import 기능이 없으므로 전체 내보내기는 순수 백업 용도다.
//! 실제로 매일 쓰이는 것은 문자 발송용 미제출자 명단 쪽이다.

use crate::commands::check::get_pending_impl;
use crate::commands::with_conn;
use crate::due::{format_korean, parse_date};
use crate::state::DbState;
use rusqlite::Connection;
use std::fs;
use tauri::State;

/// 엑셀이 UTF-8을 알아보게 하는 BOM. 없으면 한글이 깨져 열린다.
const BOM: &str = "\u{feff}";

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

/// 문자 일괄 발송용 미제출자 명단.
///
/// 외부 문자 시스템이 학년/반/번호로 수신자를 식별하므로 이 셋이 필수 컬럼이다.
/// `guardian_phone`은 입력되어 있을 때만 채운다 — 없다고 해서 행을 빼지 않는다.
pub fn build_pending_csv(rows: &[crate::types::PendingRow]) -> String {
    let mut out = String::from(BOM);
    out.push_str(&csv_line(&[
        "학년".into(),
        "반".into(),
        "번호".into(),
        "성명".into(),
        "결석일".into(),
        "미제출 항목".into(),
        "마감일".into(),
        "경과일".into(),
        "연락처 관계".into(),
        "연락처".into(),
    ]));
    out.push('\n');

    for r in rows {
        let date_label = parse_date(&r.date).map(format_korean).unwrap_or_else(|_| r.date.clone());
        let due_label = r
            .due_date
            .as_deref()
            .and_then(|d| parse_date(d).ok())
            .map(format_korean)
            .unwrap_or_default();
        let overdue = match r.days_overdue {
            Some(d) if d > 0 => format!("{d}일 경과"),
            Some(0) => "오늘 마감".to_string(),
            Some(d) => format!("{}일 남음", -d),
            None => String::new(),
        };
        out.push_str(&csv_line(&[
            r.grade.to_string(),
            r.class_no.to_string(),
            r.number.to_string(),
            r.name.clone(),
            date_label,
            r.item_name.clone(),
            due_label,
            overdue,
            r.contact_label.clone().unwrap_or_default(),
            r.contact_value.clone().unwrap_or_default(),
        ]));
        out.push('\n');
    }
    out
}

/// 전체 백업 CSV. 한 행이 한 구간이고, 사유와 체크 상태를 붙여 둔다.
pub fn build_backup_csv_impl(
    conn: &Connection,
    year_id: i64,
    grade: i64,
    class_no: i64,
) -> Result<String, String> {
    let mut stmt = conn
        .prepare(
            // 축은 LEFT JOIN이다. 아직 안 정한 기록도 백업에 들어가야 한다.
            "SELECT s.number, s.name, sp.date, r.label, t.label, sp.start_slot, sp.end_slot,
                    sp.symptom, dr.reason, sp.group_id
             FROM absence_span sp
             JOIN student s ON s.id = sp.student_id
             LEFT JOIN attendance_reason r ON r.id = sp.reason_id
             LEFT JOIN attendance_type t ON t.id = sp.type_id
             LEFT JOIN daily_reason dr
                    ON dr.student_id = sp.student_id AND dr.date = sp.date
             WHERE s.year_id = ?1 AND s.grade = ?2 AND s.class_no = ?3
             ORDER BY sp.date, s.number, sp.id",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![year_id, grade, class_no], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<String>>(8)?,
                r.get::<_, Option<String>>(9)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut out = String::from(BOM);
    out.push_str(&csv_line(&[
        "번호".into(),
        "성명".into(),
        "날짜".into(),
        "구분".into(),
        "종류".into(),
        "시작교시".into(),
        "끝교시".into(),
        "증상".into(),
        "사유".into(),
        "묶음".into(),
    ]));
    out.push('\n');

    for (number, name, date, reason_label, type_label, start, end, symptom, reason, group) in rows {
        let date_label = parse_date(&date).map(format_korean).unwrap_or(date);
        out.push_str(&csv_line(&[
            number.to_string(),
            name,
            date_label,
            reason_label.unwrap_or_else(|| "미정".into()),
            type_label.unwrap_or_else(|| "미정".into()),
            start.unwrap_or_else(|| "*".into()),
            end.unwrap_or_else(|| "*".into()),
            symptom.unwrap_or_default(),
            reason.unwrap_or_default(),
            group.unwrap_or_default(),
        ]));
        out.push('\n');
    }
    Ok(out)
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn export_pending_csv(
    db: State<DbState>,
    year_id: i64,
    grade: i64,
    class_no: i64,
    today: String,
    dest: String,
) -> Result<String, String> {
    let csv = with_conn(&db, |c| {
        let rows = get_pending_impl(c, year_id, grade, class_no, &today)?;
        Ok(build_pending_csv(&rows))
    })?;
    fs::write(&dest, csv).map_err(|e| format!("파일을 저장하지 못했습니다: {e}"))?;
    Ok(dest)
}

#[tauri::command]
pub fn export_backup_csv(
    db: State<DbState>,
    year_id: i64,
    grade: i64,
    class_no: i64,
    dest: String,
) -> Result<String, String> {
    let csv = with_conn(&db, |c| build_backup_csv_impl(c, year_id, grade, class_no))?;
    fs::write(&dest, csv).map_err(|e| format!("파일을 저장하지 못했습니다: {e}"))?;
    Ok(dest)
}
