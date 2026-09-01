pub mod attendance;
pub mod check;
pub mod code;
pub mod config;
pub mod export;
pub mod project;
pub mod student;
pub mod year;

pub use attendance::*;
pub use check::*;
pub use code::*;
pub use config::*;
pub use export::*;
pub use project::*;
pub use student::*;
pub use year::*;

use crate::state::{DbState, DB_NOT_OPEN};
use rusqlite::Connection;
use tauri::State;

/// 커맨드 래퍼가 공통으로 쓰는 잠금 헬퍼.
///
/// 모든 Tauri 커맨드는 이 헬퍼로 커넥션을 얻어 `*_impl`에 넘기는 **얇은 래퍼**여야 한다.
/// `State<DbState>`를 받는 함수는 테스트에서 호출할 수 없기 때문이다.
pub(crate) fn with_conn<T>(
    db: &State<DbState>,
    f: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let guard = db.0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or_else(|| DB_NOT_OPEN.to_string())?;
    f(conn)
}
