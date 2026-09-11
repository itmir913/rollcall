use rusqlite::Connection;

pub mod attendance_tests;
pub mod axis_tests;
pub mod class_tests;
pub mod db_tests;
pub mod due_tests;
pub mod export_tests;
pub mod flow_tests;
pub mod home_tests;
pub mod mark_tests;
pub mod neis_tests;
pub mod phrase_tests;
pub mod project_tests;
pub mod schema_lock_tests;
pub mod school_tests;
pub mod slots_tests;
pub mod stats_tests;
pub mod student_tests;
pub mod subject_tests;
pub mod terms_tests;
pub mod year_tests;

/// 그 해의 학교를 만들 때 쓰는 이름. 시드가 더는 학교를 만들지 않으므로
/// 테스트가 직접 만든다.
pub const TEST_SCHOOL: &str = "우리 학교";

/// 테스트가 기준으로 삼는 학년도.
pub const TEST_YEAR: i64 = 2026;

/// **시드만 적용한** 메모리 DB. 학년도도 학교도 없다 — 설치 직후의 앱이 이 상태다.
///
/// 시드 불변식을 보는 테스트만 이것을 쓴다. 나머지는 `setup_test_db`다.
pub fn setup_seed_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn.execute_batch(include_str!("../schema.sql")).unwrap();
    conn.execute_batch(include_str!("../seed.sql")).unwrap();
    conn
}

/// 스키마 + 시드에 **학년도 하나와 학교 하나**를 얹은 메모리 DB.
///
/// 학교를 `create_school_impl`로 만드는 것이 요점이다. 기본 출결 태그와 한도 규정을
/// 넣는 곳이 거기라, 직접 INSERT하면 테스트만 다른 경로를 타게 된다 —
/// 그 경로가 분리되면 `create_school`이 기본값을 빠뜨려도 테스트가 알아채지 못한다.
pub fn setup_test_db() -> Connection {
    let conn = setup_seed_db();
    let year = insert_year(&conn, TEST_YEAR);
    crate::commands::school::create_school_impl(&conn, year, TEST_SCHOOL, 7, 7, true).unwrap();
    conn
}

/// 그 DB의 학교 하나. `setup_test_db`가 만든 것이다.
pub fn school_id(conn: &Connection) -> i64 {
    conn.query_row("SELECT id FROM school ORDER BY id LIMIT 1", [], |r| r.get(0))
        .unwrap()
}

/// 그 DB의 학년도 하나.
pub fn year_id(conn: &Connection) -> i64 {
    conn.query_row("SELECT id FROM academic_year ORDER BY id LIMIT 1", [], |r| {
        r.get(0)
    })
    .unwrap()
}

/// 학년도를 만든다. **같은 해를 다시 부르면 있던 id를 돌려준다** —
/// `setup_test_db`가 이미 만들어 둔 해를 테스트가 이름으로 다시 조회할 수 있어야 한다.
pub fn insert_year(conn: &Connection, year: i64) -> i64 {
    conn.execute(
        "INSERT OR IGNORE INTO academic_year (year, starts_on, ends_on) VALUES (?1, ?2, ?3)",
        rusqlite::params![year, format!("{year}-03-01"), format!("{}-02-28", year + 1)],
    )
    .unwrap();
    conn.query_row(
        "SELECT id FROM academic_year WHERE year = ?1",
        rusqlite::params![year],
        |r| r.get(0),
    )
    .unwrap()
}

/// 학교를 하나 더 만든다. 순회 교사를 흉내 내는 테스트가 쓴다.
pub fn insert_school(conn: &Connection, year_id: i64, name: &str) -> i64 {
    crate::commands::school::create_school_impl(conn, year_id, name, 7, 7, true).unwrap()
}

/// 학적 하나. 3학년 6반이고 `homeroom`이 만드는 학급과 짝이다.
///
/// **학년도가 아니라 학교를 받는다.** 학교가 이미 학년도를 안다 —
/// 이 저장소의 계층이 학년도 → 학교 → 담당 학급 · 강좌다.
pub fn insert_student(conn: &Connection, school_id: i64, number: i64, name: &str) -> i64 {
    insert_student_at(conn, school_id, 3, 6, number, name)
}

/// 시드에서 라벨로 구분 id를 찾는다. (질병 / 미인정 / 기타 / 출석인정)
pub fn reason_id(conn: &Connection, label: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM attendance_reason WHERE label = ?1 AND valid_to IS NULL",
        rusqlite::params![label],
        |r| r.get(0),
    )
    .unwrap()
}

/// 시드에서 라벨로 종류 id를 찾는다. (지각 / 조퇴 / 결석 / 결과)
pub fn type_id(conn: &Connection, label: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM attendance_type WHERE label = ?1 AND valid_to IS NULL",
        rusqlite::params![label],
        |r| r.get(0),
    )
    .unwrap()
}

/// 두 축을 한 번에. `axes(&conn, "질병", "결석")`
pub fn axes(conn: &Connection, reason: &str, r#type: &str) -> (Option<i64>, Option<i64>) {
    (Some(reason_id(conn, reason)), Some(type_id(conn, r#type)))
}

/// 시드에서 이름으로 태그 id를 찾는다. (체험학습 / 생리통)
pub fn tag_id(conn: &Connection, name: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM span_tag WHERE name = ?1 AND valid_to IS NULL",
        rusqlite::params![name],
        |r| r.get(0),
    )
    .unwrap()
}

/// 시드에서 이름으로 한도 규정 id를 찾는다.
pub fn quota_rule_id(conn: &Connection, name: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM quota_rule WHERE name = ?1 AND valid_to IS NULL",
        rusqlite::params![name],
        |r| r.get(0),
    )
    .unwrap()
}

/// 담당 학급 · 강좌 하나. role은 homeroom · subject다. **학교에 연결한다.**
pub fn insert_class(
    conn: &Connection,
    school_id: i64,
    role: &str,
    name: &str,
    grade: Option<i64>,
    class_no: Option<i64>,
) -> i64 {
    conn.execute(
        "INSERT INTO teaching_class (school_id, role, name, grade, class_no, valid_from)
         VALUES (?1, ?2, ?3, ?4, ?5, '2026-03-02')",
        rusqlite::params![school_id, role, name, grade, class_no],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 학생을 그 명단에 등록한다. **학적과 무관하다** — 소속은 이 표가 말한다.
pub fn join_class(conn: &Connection, class_id: i64, student_id: i64) -> i64 {
    conn.execute(
        "INSERT INTO class_member (class_id, student_id, joined_on) VALUES (?1, ?2, '2026-03-02')",
        rusqlite::params![class_id, student_id],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 시험에서 쓰는 담임 학급 하나. 3학년 6반이고, `insert_student`가 만드는 학적과 짝이다.
pub fn homeroom(conn: &Connection, school_id: i64) -> i64 {
    insert_class(conn, school_id, "homeroom", "3학년 6반", Some(3), Some(6))
}

/// 시험에서 쓰는 교과 강좌 하나. 반이 섞이므로 학년 · 반이 비어 있다.
pub fn subject(conn: &Connection, school_id: i64, name: &str) -> i64 {
    insert_class(conn, school_id, "subject", name, None, None)
}

/// 학생을 만들고 그 명단에 등록한다. 화면이 보는 상태가 이것이다 —
/// `insert_student`는 학적만 만들므로 명단이 비어 있다.
pub fn enroll(conn: &Connection, class_id: i64, school_id: i64, number: i64, name: &str) -> i64 {
    let id = insert_student(conn, school_id, number, name);
    join_class(conn, class_id, id);
    id
}

/// 학적 자리를 직접 지정해 학생을 넣는다. 다른 반 학생을 만들 때 쓴다.
pub fn insert_student_at(
    conn: &Connection,
    school_id: i64,
    grade: i64,
    class_no: i64,
    number: i64,
    name: &str,
) -> i64 {
    conn.execute(
        "INSERT INTO student (school_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, ?2, ?3, ?4, ?5, '2026-03-02')",
        rusqlite::params![school_id, grade, class_no, number, name],
    )
    .unwrap();
    conn.last_insert_rowid()
}
