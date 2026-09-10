//! 배포된 DB 구조를 잠근다.
//!
//! `schema.sql`을 고치면 사용자 PC에 있는 기존 파일과 구조가 어긋난다. 이 테스트는
//! 마이그레이션 없이 스키마만 고치는 것을 막는다. **테스트를 통과시키려고 지문
//! 값만 고치는 것은 금지다.** 절차는 CLAUDE.md의 DB SCHEMA RULES를 따른다.
//!
//! `schema_history/vN.sql`은 배포된 구조의 기록이므로 절대 수정하지 않는다.

use crate::db::SCHEMA_VERSION;
use crate::tests::setup_test_db;
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

/// 스키마 버전은 **1에 고정**이다.
///
/// 정식 배포 전이므로 사용자 PC에 지켜야 할 DB가 아직 없다. 이 시기에 구조를 바꾸는 옳은
/// 방법은 마이그레이션을 쓰는 것이 아니라 `schema.sql`을 고치고 개발용 DB를 지우고 다시
/// 만드는 것이다. 마이그레이션을 미리 쌓으면 한 번도 실행된 적 없는 경로가 늘어난다.
///
/// **첫 릴리스 이후에는 이 테스트의 뜻이 바뀐다.** 그때부터는 배포된 구조가 함부로 움직이지
/// 않는지 보는 잠금장치다. 버전을 올려야 할 진짜 이유가 생기면 CLAUDE.md의 DB SCHEMA RULES를
/// 전부 밟은 뒤 이 상수를 함께 올린다 — 테스트를 지우고 지나가지 말 것.
#[test]
fn schema_version_is_pinned_to_one() {
    const PINNED: u32 = 1;
    assert_eq!(
        SCHEMA_VERSION, PINNED,
        "스키마 버전은 첫 릴리스까지 {PINNED}에 고정한다. 개발 중 구조를 바꿨다면 \
         마이그레이션을 추가하지 말고 개발용 DB를 지우고 다시 만들어라. 절차는 CLAUDE.md 참고."
    );
}

/// 버전이 1이면 실행될 마이그레이션도 없어야 한다.
///
/// MIGRATIONS[0]은 "v0 → v1" 자리인데, 버전 도입 이전 DB가 존재하지 않으므로 비어 있는 것이
/// 정상이다. 여기에 SQL이 들어차기 시작하면 위 고정이 이름만 남는다.
#[test]
fn no_migration_runs_while_pinned() {
    assert_eq!(
        crate::db::MIGRATIONS.len(),
        1,
        "v1 고정 상태에서는 마이그레이션 자리가 v0→v1 하나뿐이어야 한다"
    );
    assert!(
        crate::db::MIGRATIONS[0].trim().is_empty(),
        "v0 → v1 자리는 비어 있어야 한다. 개발 중 스키마 변경은 DB 재생성으로 처리한다."
    );
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

// ── 시드 불변식 ──────────────────────────────────────────────
//
// 시드는 "설치 직후의 앱"이다. 여기가 어긋나면 처음 실행한 교사가 빈 목록을
// 마주하고, 그 상태는 화면에서 원인을 짐작할 수 없다. 스키마와 함께 잠근다.

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

fn column(conn: &Connection, sql: &str) -> Vec<String> {
    let mut stmt = conn.prepare(sql).unwrap();
    stmt.query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

/// 학교는 하나로 시작한다. 순회 교사가 둘째 학교를 등록하는 것은 설정에서 할 일이고,
/// 시드가 미리 만들어 두면 어느 쪽이 진짜인지 알 수 없다.
#[test]
fn seed_creates_a_single_school() {
    let conn = setup_test_db();
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM school"),
        1,
        "시드는 학교 한 행으로 시작한다"
    );
}

/// 최대 교시와 제출 기한은 학교 행이 들고 있다. 흔한 값일 뿐 규정이 아니지만,
/// 기본값이 바뀌면 설정 화면과 마감 계산이 함께 움직여야 하므로 여기서 잡는다.
#[test]
fn seed_school_carries_the_school_level_settings() {
    let conn = setup_test_db();
    let (max_slot, due_days, skip): (i64, i64, i64) = conn
        .query_row(
            "SELECT max_slot, due_days, due_skip_offdays FROM school",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(max_slot, 7);
    assert_eq!(due_days, 7);
    assert_eq!(skip, 1);
}

/// 학교 단위 값이 `app_config`로 새지 않았는지 확인한다. `app_config`는 학교가
/// 하나뿐이라는 가정이 들어간 전역 키-값이라, 여기에 최대 교시를 넣으면 둘째
/// 학교를 등록하는 날 그 값을 읽는 모든 곳을 다시 찾아야 한다.
#[test]
fn school_settings_do_not_live_in_app_config() {
    let conn = setup_test_db();
    let leaked = count(
        &conn,
        "SELECT COUNT(*) FROM app_config
         WHERE config_key IN ('max_slot', 'due_days', 'due_skip_offdays', 'school_name')",
    );
    assert_eq!(
        leaked, 0,
        "학교 단위 값은 school 행에 둔다. app_config는 앱 전체에 걸린 값만 담는다."
    );
}

/// 태그는 세는 대상이다. 기본값 둘이 없으면 한도 규정이 가리킬 곳이 없다.
#[test]
fn seed_tags_are_the_two_defaults() {
    let conn = setup_test_db();
    let names = column(
        &conn,
        "SELECT name FROM span_tag WHERE valid_to IS NULL ORDER BY sort_order",
    );
    assert_eq!(names, vec!["체험학습", "생리통"]);
}

/// 한도 규정은 태그를 가리킨다. 구분 × 종류 조합으로 세면 체험학습 조퇴를
/// 빠뜨리거나 교외 대회까지 섞인다.
#[test]
fn seed_quota_rules_point_at_the_seeded_tags() {
    let conn = setup_test_db();

    let untagged = count(
        &conn,
        "SELECT COUNT(*) FROM quota_rule WHERE valid_to IS NULL AND tag_id IS NULL",
    );
    assert_eq!(untagged, 0, "시드의 한도 규정은 전부 태그를 가리킨다");

    let pairs = column(
        &conn,
        "SELECT t.name FROM quota_rule q
         JOIN span_tag t ON t.id = q.tag_id
         WHERE q.valid_to IS NULL
         ORDER BY q.sort_order",
    );
    assert_eq!(pairs, vec!["체험학습", "생리통"]);
}

/// 규정과 태그는 같은 학교에 속한다. 학교가 달라지면 다른 학교의 태그를 세게 된다.
#[test]
fn quota_rules_and_tags_belong_to_the_same_school() {
    let conn = setup_test_db();
    let crossed = count(
        &conn,
        "SELECT COUNT(*) FROM quota_rule q
         JOIN span_tag t ON t.id = q.tag_id
         WHERE q.school_id <> t.school_id",
    );
    assert_eq!(crossed, 0);
}

/// 한도 규정의 기간과 단위는 코드가 아는 값이어야 한다. 스키마의 CHECK가 잡지만,
/// 시드가 그 안에 있다는 사실을 한 번 더 못 박는다.
#[test]
fn seed_quota_rules_use_known_periods_and_units() {
    let conn = setup_test_db();
    let bad = count(
        &conn,
        "SELECT COUNT(*) FROM quota_rule
         WHERE period NOT IN ('year', 'semester', 'month')
            OR unit NOT IN ('day', 'count')
            OR limit_n <= 0",
    );
    assert_eq!(bad, 0);
}

/// 종류의 `slot_prompt`는 데이터지만, 코드가 해석하는 값이다. DB에만 있고 코드가
/// 모르는 값이 들어오면 교시를 묻는 화면이 조용히 아무것도 안 묻는다.
#[test]
fn every_seeded_slot_prompt_is_known_to_the_code() {
    let conn = setup_test_db();
    let prompts = column(&conn, "SELECT DISTINCT slot_prompt FROM attendance_type");
    assert!(!prompts.is_empty(), "시드에 종류가 없다");
    for prompt in &prompts {
        assert!(
            crate::slots::is_slot_prompt(prompt),
            "slots.rs가 모르는 slot_prompt다: {prompt}"
        );
    }
}

/// 기본 종류가 `SLOT_PROMPTS`의 모든 갈래를 덮는지 확인한다. 하나라도 빠지면 그 입력 경로가
/// 앱을 처음 켠 상태에서 한 번도 실행되지 않는다.
#[test]
fn seeded_types_cover_every_slot_prompt() {
    let conn = setup_test_db();
    let prompts = column(&conn, "SELECT DISTINCT slot_prompt FROM attendance_type");
    for expected in crate::slots::SLOT_PROMPTS {
        assert!(
            prompts.iter().any(|p| p == expected),
            "시드에 slot_prompt가 {expected}인 종류가 없다"
        );
    }
}

/// 구분 × 종류가 빠짐없이 코드로 생성됐는지. 개수를 상수로 적지 않고 두 축의
/// 곱과 비교한다 — 축이 늘면 곱도 함께 늘어야 한다.
#[test]
fn every_reason_type_pair_has_a_code() {
    let conn = setup_test_db();
    let reasons = count(
        &conn,
        "SELECT COUNT(*) FROM attendance_reason WHERE valid_to IS NULL",
    );
    let types = count(
        &conn,
        "SELECT COUNT(*) FROM attendance_type WHERE valid_to IS NULL",
    );
    let codes = count(
        &conn,
        "SELECT COUNT(*) FROM attendance_code WHERE valid_to IS NULL",
    );
    assert!(reasons > 0 && types > 0, "시드에 축이 비어 있다");
    assert_eq!(
        codes,
        reasons * types,
        "구분 × 종류가 빠짐없이 코드로 생성돼야 한다"
    );

    let missing = count(
        &conn,
        "SELECT COUNT(*) FROM attendance_reason r
         CROSS JOIN attendance_type t
         WHERE r.valid_to IS NULL
           AND t.valid_to IS NULL
           AND NOT EXISTS (SELECT 1 FROM attendance_code c
                           WHERE c.reason_id = r.id AND c.type_id = t.id
                             AND c.valid_to IS NULL)",
    );
    assert_eq!(missing, 0, "코드가 없는 구분 × 종류 조합이 있다");
}

/// 코드 라벨은 구분 라벨 + 종류 라벨이다. 화면과 내보내기가 이 규칙을 전제한다.
#[test]
fn code_label_is_the_two_axis_labels_joined() {
    let conn = setup_test_db();
    let mismatched = count(
        &conn,
        "SELECT COUNT(*) FROM attendance_code c
         JOIN attendance_reason r ON r.id = c.reason_id
         JOIN attendance_type t ON t.id = c.type_id
         WHERE c.label <> r.label || t.label",
    );
    assert_eq!(mismatched, 0);
}

/// 나이스 검증이 기본 표기부터 읽지 못하면 첫 대조에서 전부 불일치로 나온다.
#[test]
fn every_code_has_an_alias_for_its_own_label() {
    let conn = setup_test_db();
    let missing = count(
        &conn,
        "SELECT COUNT(*) FROM attendance_code c
         WHERE NOT EXISTS (SELECT 1 FROM code_alias a
                           WHERE a.code_id = c.id AND a.raw = c.label)",
    );
    assert_eq!(missing, 0);
}

/// 최초 집합의 `valid_from`은 '1900-01-01'이다. 설치일을 넣으면 설치 전 날짜의
/// 출결을 입력할 때 목록이 통째로 빈다.
#[test]
fn seeded_rows_are_valid_from_the_distant_past() {
    let conn = setup_test_db();
    for table in [
        "attendance_reason",
        "attendance_type",
        "attendance_code",
        "span_tag",
        "quota_rule",
    ] {
        let late = count(
            &conn,
            &format!("SELECT COUNT(*) FROM {table} WHERE valid_from <> '1900-01-01'"),
        );
        assert_eq!(late, 0, "{table}의 시드 행에 설치일이 들어갔다");
    }
}

/// 시드에는 마감된 행이 없다. 처음부터 valid_to가 채워져 있으면 그 행은 어느
/// 화면에도 나오지 않는다.
#[test]
fn no_seeded_row_starts_out_retired() {
    let conn = setup_test_db();
    for table in [
        "attendance_reason",
        "attendance_type",
        "attendance_code",
        "span_tag",
        "quota_rule",
    ] {
        let retired = count(
            &conn,
            &format!("SELECT COUNT(*) FROM {table} WHERE valid_to IS NOT NULL"),
        );
        assert_eq!(retired, 0, "{table}에 마감된 시드 행이 있다");
    }
}

/// 시드는 기록을 만들지 않는다. 학생도 출결도 없는 상태가 설치 직후다.
#[test]
fn seed_creates_no_records() {
    let conn = setup_test_db();
    for table in [
        "student",
        "absence_span",
        "academic_year",
        "off_day",
        "contact",
    ] {
        assert_eq!(
            count(&conn, &format!("SELECT COUNT(*) FROM {table}")),
            0,
            "{table}에 시드 행이 있다"
        );
    }
}
