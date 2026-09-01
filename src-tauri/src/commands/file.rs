//! 프론트엔드가 만든 바이트를 파일로 쓴다.
//!
//! 샘플 명렬표(xlsx)는 exceljs가 브라우저 쪽에서 만든다. 그 바이트를 디스크에
//! 내려놓는 일만 여기서 한다. fs 플러그인을 통째로 여는 대신 이 커맨드 하나만
//! 두는 이유는, 앱이 실제로 필요한 권한이 "사용자가 고른 경로에 쓰기" 하나뿐이기
//! 때문이다.

use base64::Engine;
use std::path::Path;

/// 저장할 폴더가 실제로 있는지 본다.
///
/// 없는 폴더에 쓰면 OS가 내는 영문 오류가 그대로 교사에게 표시된다.
/// 다이얼로그를 거치면 대개 문제없지만, 그 사이에 폴더가 사라졌거나 USB가
/// 빠진 경우가 있다.
fn validate_parent_dir(path: &str) -> Result<(), String> {
    let parent = Path::new(path)
        .parent()
        .ok_or_else(|| "저장 위치가 올바르지 않습니다.".to_string())?;
    if parent.as_os_str().is_empty() || parent.is_dir() {
        Ok(())
    } else {
        Err("저장 위치의 폴더가 존재하지 않습니다.".to_string())
    }
}

#[tauri::command]
pub fn write_bytes_file(path: String, data: String) -> Result<(), String> {
    validate_parent_dir(&path)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| format!("파일 내용을 해독하지 못했습니다: {e}"))?;
    std::fs::write(&path, bytes).map_err(|e| format!("파일을 저장하지 못했습니다: {e}"))
}
