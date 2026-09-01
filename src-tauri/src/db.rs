use rusqlite::{Connection, Result};
use std::path::Path;

/// 현재 앱이 지원하는 스키마 버전.
///
/// `schema.sql`을 고칠 때는 예외 없이 다음을 함께 해야 한다.
///   1. 이 값을 올린다.
///   2. MIGRATIONS에 이전 버전 → 새 버전 SQL을 추가한다.
///   3. tests/schema_history/vN.sql 스냅샷을 추가한다 (기존 파일은 수정 금지).
///   4. tauri.conf.json의 version도 올린다.
/// 빠뜨리면 `tests/schema_lock_tests.rs`가 실패한다.
pub const SCHEMA_VERSION: u32 = 1;

/// 인덱스 i: 버전 i → i+1 로 올리는 SQL.
/// [0] v0→v1: 버전 도입 이전 DB는 존재하지 않는다. 자리만 채운다.
pub(crate) const MIGRATIONS: &[&str] = &[
    "", // v0 → v1
];

/// BEGIN ~ COMMIT/ROLLBACK을 감싸 트랜잭션이 열린 채 남지 않도록 보장한다.
///
/// DB Connection은 Mutex로 공유되는 하나뿐이다. 트랜잭션을 연 채 함수를 빠져나가면
/// 세션 내내 그 상태로 남아, 이후 모든 BEGIN이 실패하고 트랜잭션 없는 쓰기(셀 편집 등)가
/// 열린 트랜잭션에 묶인다. 그 상태로 앱을 닫으면 작업이 통째로 롤백된다.
///
/// 따라서 트랜잭션 안에서 조기 반환이 필요하면 반드시 클로저 안에서 `?`를 쓴다.
/// 클로저 밖에서 `?`를 쓰면 ROLLBACK을 건너뛴다.
pub fn with_transaction<T>(
    conn: &Connection,
    action: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    match action() {
        Ok(value) => {
            if let Err(e) = conn.execute_batch("COMMIT") {
                // COMMIT 실패도 트랜잭션을 열어둔다. 반드시 되돌린다.
                let _ = conn.execute_batch("ROLLBACK");
                return Err(e.to_string());
            }
            Ok(value)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

// ── 내부 헬퍼 ────────────────────────────────────────────────

fn get_version(conn: &Connection) -> Result<u32> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
}

/// 현재 버전에서 SCHEMA_VERSION까지 마이그레이션을 단계별로 실행한다.
/// - 각 단계는 rusqlite Transaction으로 감싸 실패 시 자동 ROLLBACK된다.
/// - foreign_keys는 트랜잭션 외부에서만 변경 가능하므로, IIFE 종료 후 복구한다.
/// - 각 단계 커밋 전 PRAGMA foreign_key_check로 무결성을 검증한다.
pub fn migrate(conn: &mut Connection, from: u32) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;

    let result: Result<()> = (|| {
        for v in from..SCHEMA_VERSION {
            let idx = v as usize;
            let sql = MIGRATIONS.get(idx).copied().ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(format!(
                    "마이그레이션 스크립트 누락: v{v} → v{}",
                    v + 1
                ))
            })?;

            let tx = conn.transaction()?;

            if !sql.is_empty() {
                tx.execute_batch(sql)?;
            }

            tx.pragma_update(None, "user_version", v + 1)?;

            {
                let mut stmt = tx.prepare("PRAGMA foreign_key_check;")?;
                if stmt.exists([])? {
                    return Err(rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                        Some(format!("v{v} → v{} 마이그레이션 후 외래키 무결성 위반", v + 1)),
                    ));
                }
            }

            tx.commit()?;
        }
        Ok(())
    })();

    // 트랜잭션이 모두 닫힌 후 복구 — 열린 트랜잭션이 없으므로 PRAGMA가 반드시 적용된다.
    let fk_result = conn.execute_batch("PRAGMA foreign_keys = ON;");

    result.and(fk_result)
}

// ── 공개 API ─────────────────────────────────────────────────

/// 새 DB 파일 생성 → 스키마 + 시드 적용 → 버전 기록
pub fn create_new(db_path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    {
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("schema.sql"))?;
        tx.execute_batch(include_str!("seed.sql"))?;
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        tx.commit()?;
    }
    Ok(conn)
}

/// 기존 DB 파일 열기 — 버전 검사만 한다. 마이그레이션은 `migrate_schema` 커맨드에서.
pub fn open_existing(db_path: &Path) -> Result<Connection, OpenError> {
    let conn = Connection::open(db_path).map_err(OpenError::Db)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(OpenError::Db)?;

    let db_version = get_version(&conn).map_err(OpenError::Db)?;

    if db_version > SCHEMA_VERSION {
        return Err(OpenError::TooNew {
            db_version,
            app_version: SCHEMA_VERSION,
        });
    }

    Ok(conn)
}

/// 열기 전 DB 파일의 버전만 확인한다.
pub fn file_version(db_path: &Path) -> Result<u32> {
    let conn = Connection::open(db_path)?;
    get_version(&conn)
}

// ── 오류 타입 ─────────────────────────────────────────────────

#[derive(Debug)]
pub enum OpenError {
    Db(rusqlite::Error),
    /// DB 파일이 현재 앱보다 상위 버전
    TooNew { db_version: u32, app_version: u32 },
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Db(e) => write!(f, "데이터베이스 오류: {e}"),
            OpenError::TooNew {
                db_version,
                app_version,
            } => write!(
                f,
                "이 파일은 더 최신 버전의 앱에서 만들어졌습니다. \
                 앱을 업데이트해주세요. (파일 버전: v{db_version}, 현재 앱: v{app_version})"
            ),
        }
    }
}
