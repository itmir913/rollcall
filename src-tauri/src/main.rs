// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod due;
mod phrase;
mod slots;
mod state;
mod types;
#[cfg(test)]
mod tests;

use commands::*;
use state::{DbPathState, DbState};
use std::sync::Mutex;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(DbState(Mutex::new(None)))
        .manage(DbPathState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            // 파일 · DB
            init_db,
            migrate_schema,
            export_backup,
            get_db_path,
            write_bytes_file,
            // 앱 설정 (마지막에 연 학교 · 학년도 · 학급)
            get_config,
            set_config,
            get_years,
            create_year,
            update_year,
            // 학교 설정 — 최대 교시 · 제출 기한 · 휴업일 · 태그 · 한도 규정
            get_schools,
            get_school,
            update_school,
            get_off_days,
            add_off_day,
            remove_off_day,
            get_tags,
            create_tag,
            rename_tag,
            retire_tag,
            get_quota_rules,
            create_quota_rule,
            revise_quota_rule,
            retire_quota_rule,
            // 학생 · 연락처
            detect_roster_class,
            get_students,
            get_classes,
            preview_roster,
            apply_roster,
            update_student,
            withdraw_student,
            get_contacts,
            set_contacts,
            // 출결 축 (구분 · 종류 · 쌍)
            get_reasons,
            create_reason,
            retire_reason,
            get_types,
            create_type,
            retire_type,
            get_codes,
            create_code,
            revise_code,
            retire_code,
            get_memo_suggestions,
            // 출결 입력 · 수정
            get_day_grid,
            stamp_span,
            edit_span,
            delete_span,
            set_span_memo,
            set_span_tag,
            get_month_log,
            get_spans_between,
            preview_bulk,
            apply_bulk,
            // 서류 · 나이스 표시
            set_doc_done,
            set_neis_done,
            mark_day_neis,
            get_doc_pending,
            get_neis_pending,
            // 나이스 가져오기 — 교체가 아니라 차분이다
            preview_neis_import,
            apply_neis_import,
            // 개요 · 통계
            get_home_summary,
            get_quota_reports,
            // 내보내기
            export_spans_csv,
            export_pending_csv,
            export_quota_csv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
