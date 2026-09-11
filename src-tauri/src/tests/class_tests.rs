//! 맡은 것(`teaching_class`)과 소속(`class_member`), 그리고 연락처.
//!
//! 학생 이름은 전부 가짜다.
//!
//! 이 파일이 붙들고 있는 결정은 셋이다.
//!   · **학생은 학급에 매달리지 않는다.** 학년 · 반 · 번호는 학적이고, 소속은 따로 있다.
//!   · **담임 명렬표가 반으로 걸러지지 않는다.** 반이 다른 학생을 막지 않는다.
//!   · **두 모드는 한 표에 있되 role로 갈린다.** 이동 · 설정을 두 벌 만들지 않으려는 것이다.

use super::*;
use rusqlite::Connection;

fn year(conn: &Connection) -> i64 {
    insert_year(conn, 2026)
}

fn members(conn: &Connection, class_id: i64) -> Vec<(i64, i64, String)> {
    let mut stmt = conn
        .prepare(
            "SELECT st.grade, st.class_no, st.name
               FROM class_member m JOIN student st ON st.id = m.student_id
              WHERE m.class_id = ?1 AND m.left_on IS NULL
              ORDER BY st.grade, st.class_no, st.number",
        )
        .unwrap();
    let rows = stmt
        .query_map([class_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    rows
}

// ── 소속 ──────────────────────────────────────────────────────

#[test]
fn 담임_명렬표는_반으로_걸러지지_않는다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let class = insert_class(&conn, y, "homeroom", "3학년 6반", Some(3), Some(6));

    let mine = insert_student_at(&conn, y, 3, 6, 1, "학생1");
    // 전학 · 위탁처럼 반이 다른 학생이 내 명단에 들어오는 날이 있다.
    let other = insert_student_at(&conn, y, 3, 7, 12, "학생2");
    join_class(&conn, class, mine);
    join_class(&conn, class, other);

    let rows = members(&conn, class);
    assert_eq!(rows.len(), 2, "반이 달라도 막지 않는다");
    assert_eq!(rows[1].1, 7, "7반 학생이 6반 명단에 남아 있다");
}

#[test]
fn 같은_학생이_담임_학급과_교과_강좌에_함께_있는다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let home = insert_class(&conn, y, "homeroom", "3학년 6반", Some(3), Some(6));
    let subject = insert_class(&conn, y, "subject", "지구과학Ⅰ", None, None);

    let student = insert_student_at(&conn, y, 3, 6, 1, "학생1");
    join_class(&conn, home, student);
    join_class(&conn, subject, student);

    // **학생 행은 하나다.** 명단이 둘일 뿐이다.
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM student", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(members(&conn, home).len(), 1);
    assert_eq!(members(&conn, subject).len(), 1);
}

#[test]
fn 교과_강좌는_여러_반에서_모인다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let subject = insert_class(&conn, y, "subject", "지구과학Ⅰ", None, None);

    for (grade, class_no, number) in [(3, 1, 4), (3, 6, 11), (2, 3, 20)] {
        let s = insert_student_at(&conn, y, grade, class_no, number, "학생");
        join_class(&conn, subject, s);
    }

    let rows = members(&conn, subject);
    assert_eq!(rows.len(), 3);
    // 학년까지 섞인다. 반으로 걸러내는 질의였다면 담을 수 없는 명단이다.
    assert_eq!(rows[0].0, 2, "2학년 학생도 같은 강좌에 있다");
}

#[test]
fn 같은_명단에_두_번_넣지_않는다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let class = insert_class(&conn, y, "homeroom", "3학년 6반", Some(3), Some(6));
    let student = insert_student_at(&conn, y, 3, 6, 1, "학생1");
    join_class(&conn, class, student);

    let again = conn.execute(
        "INSERT INTO class_member (class_id, student_id, joined_on) VALUES (?1, ?2, '2026-03-02')",
        rusqlite::params![class, student],
    );
    assert!(again.is_err(), "같은 학생이 한 명단에 두 줄일 수 없다");
}

#[test]
fn 명단에서_빠져도_지우지_않는다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let class = insert_class(&conn, y, "homeroom", "3학년 6반", Some(3), Some(6));
    let student = insert_student_at(&conn, y, 3, 6, 1, "학생1");
    join_class(&conn, class, student);

    // 명렬표 재가져오기와 같은 규칙이다 — 지난 기록이 어느 명단의 것이었는지 남아야 한다.
    conn.execute(
        "UPDATE class_member SET left_on = '2026-09-01' WHERE class_id = ?1",
        rusqlite::params![class],
    )
    .unwrap();

    assert!(members(&conn, class).is_empty(), "지금 명단에는 없다");
    let kept: i64 = conn
        .query_row("SELECT COUNT(*) FROM class_member", [], |r| r.get(0))
        .unwrap();
    assert_eq!(kept, 1, "줄은 남는다");
}

#[test]
fn 학급을_지우면_소속도_함께_사라진다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let class = insert_class(&conn, y, "homeroom", "3학년 6반", Some(3), Some(6));
    let student = insert_student_at(&conn, y, 3, 6, 1, "학생1");
    join_class(&conn, class, student);

    conn.execute("DELETE FROM teaching_class WHERE id = ?1", [class])
        .unwrap();

    let left: i64 = conn
        .query_row("SELECT COUNT(*) FROM class_member", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 0);
    // 학생은 학급에 매달리지 않으므로 남는다.
    let students: i64 = conn
        .query_row("SELECT COUNT(*) FROM student", [], |r| r.get(0))
        .unwrap();
    assert_eq!(students, 1, "학생은 학교에 속하지 학급에 속하지 않는다");
}

// ── 맡은 것 ───────────────────────────────────────────────────

#[test]
fn 두_모드는_한_표에_있되_role로_갈린다() {
    let conn = setup_test_db();
    let y = year(&conn);
    insert_class(&conn, y, "homeroom", "3학년 6반", Some(3), Some(6));
    insert_class(&conn, y, "subject", "지구과학Ⅰ", None, None);
    insert_class(&conn, y, "subject", "지구과학Ⅱ", None, None);

    let count = |role: &str| -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM teaching_class WHERE role = ?1 AND valid_to IS NULL",
            [role],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(count("homeroom"), 1);
    assert_eq!(count("subject"), 2);
}

#[test]
fn 모르는_역할은_들어가지_않는다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let bad = conn.execute(
        "INSERT INTO teaching_class (school_id, year_id, role, name, valid_from)
         VALUES (?1, ?2, '부담임', '3학년 6반', '2026-03-02')",
        rusqlite::params![school_id(&conn), y],
    );
    assert!(bad.is_err(), "role은 homeroom · subject 둘뿐이다");
}

#[test]
fn 교과_강좌는_반을_비워_둔다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let subject = insert_class(&conn, y, "subject", "지구과학Ⅰ", None, None);

    let (grade, class_no): (Option<i64>, Option<i64>) = conn
        .query_row(
            "SELECT grade, class_no FROM teaching_class WHERE id = ?1",
            [subject],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(grade.is_none() && class_no.is_none(), "반이 섞이므로 가리킬 반이 없다");
}

#[test]
fn 이름_없는_학급은_들어가지_않는다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let bad = conn.execute(
        "INSERT INTO teaching_class (school_id, year_id, role, name, valid_from)
         VALUES (?1, ?2, 'subject', '', '2026-03-02')",
        rusqlite::params![school_id(&conn), y],
    );
    assert!(bad.is_err(), "화면에 적을 이름이 없으면 고를 수도 없다");
}

// ── 교과 수업 한 칸 ───────────────────────────────────────────

#[test]
fn 기록한_교시와_아직인_교시를_구별한다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let subject = insert_class(&conn, y, "subject", "지구과학Ⅰ", None, None);

    // 2교시는 불렀고 빠진 학생이 없다. 5교시는 아직 부르지 않았다.
    // 결석자 행만으로는 이 둘이 구별되지 않아 수업 칸을 따로 둔다.
    conn.execute(
        "INSERT INTO subject_session (class_id, date, slot, taken_on)
         VALUES (?1, '2026-09-11', '2', '2026-09-11')",
        rusqlite::params![subject],
    )
    .unwrap();

    let taken: Vec<String> = conn
        .prepare("SELECT slot FROM subject_session WHERE class_id = ?1 AND date = '2026-09-11'")
        .unwrap()
        .query_map([subject], |r| r.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(taken, vec!["2".to_string()], "5교시는 기록한 적이 없다");
}

#[test]
fn 같은_교시를_두_번_기록하지_않는다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let subject = insert_class(&conn, y, "subject", "지구과학Ⅰ", None, None);
    let insert = || {
        conn.execute(
            "INSERT INTO subject_session (class_id, date, slot, taken_on)
             VALUES (?1, '2026-09-11', '2', '2026-09-11')",
            rusqlite::params![subject],
        )
    };
    insert().unwrap();
    assert!(insert().is_err());
}

#[test]
fn 수업_칸을_지우면_결석자도_함께_사라진다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let subject = insert_class(&conn, y, "subject", "지구과학Ⅰ", None, None);
    let student = insert_student_at(&conn, y, 3, 6, 1, "학생1");
    join_class(&conn, subject, student);

    conn.execute(
        "INSERT INTO subject_session (class_id, date, slot, taken_on)
         VALUES (?1, '2026-09-11', '2', '2026-09-11')",
        rusqlite::params![subject],
    )
    .unwrap();
    let session = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO subject_absence (session_id, student_id) VALUES (?1, ?2)",
        rusqlite::params![session, student],
    )
    .unwrap();

    conn.execute("DELETE FROM subject_session WHERE id = ?1", [session])
        .unwrap();
    let left: i64 = conn
        .query_row("SELECT COUNT(*) FROM subject_absence", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 0);
}

// ── 연락처 ────────────────────────────────────────────────────

#[test]
fn 연락처는_학생마다_있는_만큼만_담는다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let only_self = insert_student_at(&conn, y, 3, 6, 1, "학생1");
    let both = insert_student_at(&conn, y, 3, 6, 2, "학생2");

    let add = |student: i64, kind: &str, phone: &str, order: i64| {
        conn.execute(
            "INSERT INTO contact (student_id, type, phone, sort_order) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![student, kind, phone, order],
        )
        .unwrap();
    };
    add(only_self, "본인", "010-0000-0001", 0);
    add(both, "모", "010-0000-0002", 0);
    add(both, "부", "010-0000-0003", 1);

    let count = |student: i64| -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM contact WHERE student_id = ?1",
            [student],
            |r| r.get(0),
        )
        .unwrap()
    };
    // 칸을 고정했다면 앞의 학생에게 빈 칸 둘이 남았을 자리다.
    assert_eq!(count(only_self), 1);
    assert_eq!(count(both), 2);
}

#[test]
fn 먼저_걸_번호가_정해져_있다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let student = insert_student_at(&conn, y, 3, 6, 1, "학생1");
    for (kind, phone, order) in [("부", "010-0000-0003", 2), ("모", "010-0000-0002", 1)] {
        conn.execute(
            "INSERT INTO contact (student_id, type, phone, sort_order) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![student, kind, phone, order],
        )
        .unwrap();
    }
    let first: String = conn
        .query_row(
            "SELECT type FROM contact WHERE student_id = ?1 ORDER BY sort_order LIMIT 1",
            [student],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(first, "모", "급한 순간에 어느 번호부터 걸지 고민하지 않게 한다");
}

#[test]
fn 관계도_번호도_비어_있을_수_없다() {
    let conn = setup_test_db();
    let y = year(&conn);
    let student = insert_student_at(&conn, y, 3, 6, 1, "학생1");
    let blank = |kind: &str, phone: &str| {
        conn.execute(
            "INSERT INTO contact (student_id, type, phone) VALUES (?1, ?2, ?3)",
            rusqlite::params![student, kind, phone],
        )
    };
    assert!(blank("", "010-0000-0001").is_err());
    assert!(blank("모", "").is_err());
}
