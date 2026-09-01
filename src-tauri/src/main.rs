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
            // 파일
            init_db,
            migrate_schema,
            export_backup,
            get_db_path,
            // 설정
            get_config,
            set_config,
            get_years,
            create_year,
            update_year,
            // 학생
            parse_roster,
            get_students,
            preview_roster,
            apply_roster,
            update_student,
            withdraw_student,
            // 출결 코드
            get_codes,
            create_code,
            revise_code,
            retire_code,
            get_symptom_suggestions,
            // 출결 입력
            get_day_grid,
            render_phrase,
            add_span,
            update_span,
            delete_span,
            set_daily_reason,
            copy_previous,
            bulk_preview,
            bulk_apply,
            // 체크
            get_check_items,
            create_check_item,
            update_check_item,
            deactivate_check_item,
            set_check,
            set_check_due,
            set_group_check,
            get_pending,
            get_pending_summary,
            // 내보내기
            export_pending_csv,
            export_backup_csv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
