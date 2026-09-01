//! 배포된 DB 구조를 잠근다.
//!
//! `schema.sql`을 고치면 사용자 PC에 있는 기존 파일과 구조가 어긋난다. 이 테스트는
//! 마이그레이션 없이 스키마만 고치는 것을 막는다. **테스트를 통과시키려고 지문
//! 값만 고치는 것은 금지다.** 절차는 CLAUDE.md의 DB SCHEMA RULES를 따른다.
//!
//! `schema_history/vN.sql`은 배포된 구조의 기록이므로 절대 수정하지 않는다.

use crate::db::SCHEMA_VERSION;
use rusqlite::Connection;

/// 각 버전의 스키마 스냅샷. 새 버전을 추가할 때만 항목이 늘어난다.
const SCHEMA_BASELINES: &[(u32, &str)] = &[(1, include_str!("schema_history/v1.sql"))];

/// 의존성 없이 쓰는 FNV-1a. 암호학적 용도가 아니라 "바뀌었는가"만 본다.
fn fingerprint(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// DB에 실제로 만들어진 객체 목록. 주석·공백 차이를 걸러내고 구조만 본다.
fn schema_objects(sql: &str) -> String {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(sql).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT type, name, IFNULL(sql, '') FROM sqlite_master
             WHERE name NOT LIKE 'sqlite_%'
             ORDER BY type, name",
        )
        .unwrap();
    let rows: Vec<String> = stmt
        .query_map([], |r| {
            Ok(format!(
                "{}|{}|{}",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    rows.join("\n")
}

#[test]
fn every_version_has_a_snapshot() {
    assert_eq!(
        SCHEMA_BASELINES.len() as u32,
        SCHEMA_VERSION,
        "SCHEMA_VERSION을 올렸으면 schema_history/vN.sql과 SCHEMA_BASELINES 항목을 \
         함께 추가해야 한다. 자세한 절차는 CLAUDE.md 참고."
    );
}

#[test]
fn current_schema_matches_its_snapshot() {
    let (_, snapshot) = SCHEMA_BASELINES
        .iter()
        .find(|(v, _)| *v == SCHEMA_VERSION)
        .expect("현재 버전의 스냅샷이 없다");

    let current = schema_objects(include_str!("../schema.sql"));
    let recorded = schema_objects(snapshot);

    assert_eq!(
        fingerprint(&current),
        fingerprint(&recorded),
        "schema.sql이 v{SCHEMA_VERSION} 스냅샷과 다르다.\n\
         스키마를 고쳤다면 SCHEMA_VERSION을 올리고 새 스냅샷을 추가하라. \
         스냅샷 파일을 고쳐서 맞추지 말 것.\n\n현재:\n{current}\n\n기록:\n{recorded}"
    );
}

#[test]
fn snapshots_still_load() {
    // 과거 스냅샷이 SQLite에서 열리지 않으면 마이그레이션 경로를 검증할 수 없다.
    for (version, sql) in SCHEMA_BASELINES {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(sql)
            .unwrap_or_else(|e| panic!("v{version} 스냅샷을 적용하지 못했다: {e}"));
    }
}

#[test]
fn seed_applies_to_the_current_schema() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(include_str!("../schema.sql")).unwrap();
    conn.execute_batch(include_str!("../seed.sql")).unwrap();
}
