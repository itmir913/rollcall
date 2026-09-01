use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct DbState(pub Mutex<Option<Connection>>);
pub struct DbPathState(pub Mutex<Option<PathBuf>>);

/// SQLite 제약 위반을 한국어 메시지로 바꾼다.
///
/// 번역하지 않으면 "UNIQUE constraint failed: student.number" 같은 영문 원문이
/// 그대로 교사에게 표시된다. CHECK 위반은 커맨드 진입부 검증이 먼저 막는 것이
/// 원칙이고, 이 번역은 검증이 놓친 경우를 위한 방어선이다.
pub fn constraint_err(e: &rusqlite::Error, conflict_msg: &str) -> String {
    let text = e.to_string();
    if text.contains("UNIQUE constraint failed") {
        conflict_msg.to_string()
    } else if text.contains("CHECK constraint failed") {
        "입력값이 허용 범위를 벗어났습니다.".to_string()
    } else if text.contains("FOREIGN KEY constraint failed") {
        "참조하는 항목이 없습니다. 목록을 새로 고친 뒤 다시 시도해주세요.".to_string()
    } else {
        text
    }
}

/// DB가 열려 있지 않을 때의 공통 메시지.
pub const DB_NOT_OPEN: &str = "데이터 파일이 열려 있지 않습니다.";
