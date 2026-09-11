//! 담당 학급 · 강좌(`teaching_class`)와 소속(`class_member`), 그리고 연락처.
//!
//! 학생 이름은 전부 가짜다.
//!
//! 이 파일이 지키는 결정은 셋이다.
//!   · **학생은 학급에 종속되지 않는다.** 학년 · 반 · 번호는 학적이고, 소속은 따로 있다.
//!   · **담임 명렬표가 반으로 걸러지지 않는다.** 반이 다른 학생을 막지 않는다.
//!   · **두 모드는 한 표에 있되 role로 구별된다.** 이동 · 설정을 두 벌 만들지 않으려는 것이다.

use super::*;
use crate::commands::class::*;
use rusqlite::Connection;

const TODAY: &str = "2026-09-11";

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
    let s = school_id(&conn);
    let class = insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));

    let mine = insert_student_at(&conn, s, 3, 6, 1, "학생1");
    // 전학 · 위탁처럼 반이 다른 학생이 내 명단에 들어오는 날이 있다.
    let other = insert_student_at(&conn, s, 3, 7, 12, "학생2");
    join_class(&conn, class, mine);
    join_class(&conn, class, other);

    let rows = members(&conn, class);
    assert_eq!(rows.len(), 2, "반이 달라도 막지 않는다");
    assert_eq!(rows[1].1, 7, "7반 학생이 6반 명단에 남아 있다");
}

#[test]
fn 같은_학생이_담임_학급과_교과_강좌에_함께_있는다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let home = insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));
    let subject = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);

    let student = insert_student_at(&conn, s, 3, 6, 1, "학생1");
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
    let s = school_id(&conn);
    let subject = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);

    for (grade, class_no, number) in [(3, 1, 4), (3, 6, 11), (2, 3, 20)] {
        let student = insert_student_at(&conn, s, grade, class_no, number, "학생");
        join_class(&conn, subject, student);
    }

    let rows = members(&conn, subject);
    assert_eq!(rows.len(), 3);
    // 학년까지 섞인다. 반으로 걸러내는 질의였다면 담을 수 없는 명단이다.
    assert_eq!(rows[0].0, 2, "2학년 학생도 같은 강좌에 있다");
}

#[test]
fn 같은_명단에_두_번_넣지_않는다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let class = insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));
    let student = insert_student_at(&conn, s, 3, 6, 1, "학생1");
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
    let s = school_id(&conn);
    let class = insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));
    let student = insert_student_at(&conn, s, 3, 6, 1, "학생1");
    join_class(&conn, class, student);

    // 명렬표 다시 열기와 같은 규칙이다 — 지난 기록이 어느 명단의 것이었는지 남아야 한다.
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
    let s = school_id(&conn);
    let class = insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));
    let student = insert_student_at(&conn, s, 3, 6, 1, "학생1");
    join_class(&conn, class, student);

    conn.execute("DELETE FROM teaching_class WHERE id = ?1", [class])
        .unwrap();

    let left: i64 = conn
        .query_row("SELECT COUNT(*) FROM class_member", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 0);
    // 학생은 학급에 종속되지 않으므로 남는다.
    let students: i64 = conn
        .query_row("SELECT COUNT(*) FROM student", [], |r| r.get(0))
        .unwrap();
    assert_eq!(students, 1, "학생은 학교에 속하지 학급에 속하지 않는다");
}

// ── 담당 학급 · 강좌 ──────────────────────────────────────────

#[test]
fn 두_모드는_한_표에_있되_role로_구별된다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));
    insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);
    insert_class(&conn, s, "subject", "지구과학Ⅱ", None, None);

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
    let s = school_id(&conn);
    let bad = conn.execute(
        "INSERT INTO teaching_class (school_id, role, name, valid_from)
         VALUES (?1, '부담임', '3학년 6반', '2026-03-02')",
        rusqlite::params![s],
    );
    assert!(bad.is_err(), "role은 homeroom · subject 둘뿐이다");
}

#[test]
fn 교과_강좌는_반을_비워_둔다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let subject = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);

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
    let s = school_id(&conn);
    let bad = conn.execute(
        "INSERT INTO teaching_class (school_id, role, name, valid_from)
         VALUES (?1, 'subject', '', '2026-03-02')",
        rusqlite::params![s],
    );
    assert!(bad.is_err(), "화면에 적을 이름이 없으면 선택할 수도 없다");
}

// ── 담당 학급 · 강좌를 더하고 고치고 마감한다 ─────────────────

/// 그 학급에 출결 한 건을 남긴다. 마감이 기록을 건드리지 않는 것을 확인할 때 쓴다.
fn stamp(conn: &Connection, class_id: i64, student_id: i64) {
    let (reason, r#type) = axes(conn, "질병", "결석");
    conn.execute(
        "INSERT INTO absence_span (class_id, student_id, date, reason_id, type_id)
         VALUES (?1, ?2, '2026-09-01', ?3, ?4)",
        rusqlite::params![class_id, student_id, reason, r#type],
    )
    .unwrap();
}

fn class_names(conn: &Connection, school_id: i64) -> Vec<String> {
    get_teaching_classes_impl(conn, school_id, None)
        .unwrap()
        .into_iter()
        .map(|c| c.name)
        .collect()
}

#[test]
fn 담당_학급과_강좌는_역할로_걸러_온다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    create_teaching_class_impl(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6), None, TODAY)
        .unwrap();
    create_teaching_class_impl(&conn, s, "subject", "지구과학Ⅰ", None, None, None, TODAY).unwrap();

    assert_eq!(class_names(&conn, s).len(), 2);
    let only = get_teaching_classes_impl(&conn, s, Some("subject")).unwrap();
    assert_eq!(only.len(), 1);
    assert_eq!(only[0].name, "지구과학Ⅰ");
}

#[test]
fn 담임_학급은_학년과_반이_있어야_들어간다() {
    // 그 둘이 명렬표가 학생을 배치할 학적 자리다. 나중에 채우게 두면 명렬표를
    // 여는 자리에서야 빠진 것을 알게 된다.
    let conn = setup_test_db();
    let s = school_id(&conn);

    let err = create_teaching_class_impl(&conn, s, "homeroom", "3학년 6반", None, None, None, TODAY)
        .unwrap_err();
    assert!(err.contains("담임 학급은 학년과 반이 필요합니다"), "{err}");

    let err = create_teaching_class_impl(&conn, s, "homeroom", "3학년 6반", Some(3), None, None, TODAY)
        .unwrap_err();
    assert!(err.contains("담임 학급은 학년과 반이 필요합니다"), "{err}");

    assert!(class_names(&conn, s).is_empty(), "거절된 학급이 새어 들어갔다");
}

#[test]
fn 교과_강좌에_학년과_반이_들어오면_거절한다() {
    // 교과 강좌는 여러 반에서 모이므로 가리킬 반이 없다. 한 반을 적어 두면
    // 그 값을 믿는 화면이 나머지 반 학생을 조용히 빠뜨린다.
    let conn = setup_test_db();
    let s = school_id(&conn);

    let err = create_teaching_class_impl(&conn, s, "subject", "지구과학Ⅰ", Some(3), Some(6), None, TODAY)
        .unwrap_err();
    assert!(err.contains("학년 · 반을 두지 않습니다"), "{err}");

    // 고치는 길로도 같은 규칙이 걸린다.
    let subject = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);
    let err = update_teaching_class_impl(&conn, subject, "지구과학Ⅱ", Some(3), Some(6), None).unwrap_err();
    assert!(err.contains("학년 · 반을 두지 않습니다"), "{err}");

    let home = insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));
    let err = update_teaching_class_impl(&conn, home, "3학년 7반", None, None, None).unwrap_err();
    assert!(err.contains("담임 학급은 학년과 반이 필요합니다"), "{err}");
}

#[test]
fn 이름_학년_반을_고치면_같은_줄이_그대로_남는다() {
    // 학급 이름은 화면에 적히는 이름표이고 기록은 class_id로 가리킨다. 여기서
    // 마감 후 추가를 하면 지난 출결이 마감된 학급에 남아 화면에서 통째로 사라진다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let class = insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));
    let student = insert_student_at(&conn, s, 3, 6, 1, "학생1");
    join_class(&conn, class, student);
    stamp(&conn, class, student);

    update_teaching_class_impl(&conn, class, "  3학년 7반  ", Some(3), Some(7), None).unwrap();

    let classes = get_teaching_classes_impl(&conn, s, None).unwrap();
    assert_eq!(classes.len(), 1, "줄이 늘지 않았다");
    assert_eq!(classes[0].id, class, "id가 그대로라 지난 기록이 따라온다");
    assert_eq!(classes[0].name, "3학년 7반");
    assert_eq!(classes[0].class_no, Some(7));

    let kept: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM absence_span WHERE class_id = ?1",
            [class],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(kept, 1);
}

#[test]
fn 이름이_비면_만들지도_고치지도_않는다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let class = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);

    assert!(
        create_teaching_class_impl(&conn, s, "subject", "   ", None, None, None, TODAY)
            .unwrap_err()
            .contains("비어")
    );
    assert!(update_teaching_class_impl(&conn, class, "  ", None, None, None)
        .unwrap_err()
        .contains("비어"));
    assert_eq!(class_names(&conn, s), vec!["지구과학Ⅰ"]);
}

#[test]
fn 마감한_학급은_목록에서_빠진다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));
    let last_year = insert_class(&conn, s, "homeroom", "2학년 4반", Some(2), Some(4));

    retire_teaching_class_impl(&conn, last_year, TODAY).unwrap();

    assert_eq!(class_names(&conn, s), vec!["3학년 6반"]);
    // 두 번 마감하지 않는다. 마감된 학급은 고칠 수도 없다.
    assert!(retire_teaching_class_impl(&conn, last_year, TODAY)
        .unwrap_err()
        .contains("유효한 학급을 찾을 수 없습니다"));
    assert!(
        update_teaching_class_impl(&conn, last_year, "2학년 5반", Some(2), Some(5), None)
            .unwrap_err()
            .contains("유효한 학급을 찾을 수 없습니다")
    );
}

#[test]
fn 학급을_마감해도_그_학급의_출결은_남는다() {
    // absence_span.class_id가 ON DELETE CASCADE다. 지우는 방식이었다면 3월에
    // 지난해 학급을 정리하는 동작이 지난해 출결을 지우는 동작이 된다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let class = insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));
    let student = insert_student_at(&conn, s, 3, 6, 1, "학생1");
    join_class(&conn, class, student);
    stamp(&conn, class, student);

    retire_teaching_class_impl(&conn, class, TODAY).unwrap();

    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM teaching_class", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 1, "마감이지 삭제가 아니다");
    let spans: i64 = conn
        .query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(spans, 1);
    // 명단도 그대로다.
    assert_eq!(members(&conn, class).len(), 1);
}

#[test]
fn 기한일은_ISO여야_한다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let class = insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));

    assert!(retire_teaching_class_impl(&conn, class, "2026.09.11.(금)")
        .unwrap_err()
        .contains("날짜 형식"));
    assert_eq!(class_names(&conn, s), vec!["3학년 6반"], "아무것도 마감되지 않았다");
}

#[test]
fn 같은_이름의_학급과_강좌를_두_번_만들지_않는다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let make = || {
        create_teaching_class_impl(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6), None, TODAY)
    };
    let first = make().unwrap();

    // 목록에 '3학년 6반'이 둘이면 어느 쪽에 입력했는지 교사가 알 수 없다.
    assert!(make().unwrap_err().contains("이미 같은"));

    // 마감한 것은 세지 않는다 — 같은 이름을 다시 맡는 해가 온다.
    retire_teaching_class_impl(&conn, first, TODAY).unwrap();
    make().unwrap();
}

#[test]
fn 역할이_다르면_이름이_같아도_된다() {
    // 공통과목을 한 반에 통째로 가르치는 교사는 강좌 이름을 반 이름으로 짓는다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    create_teaching_class_impl(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6), None, TODAY)
        .unwrap();
    create_teaching_class_impl(&conn, s, "subject", "3학년 6반", None, None, None, TODAY).unwrap();
}

// ── 교과 수업 한 칸 ───────────────────────────────────────────

#[test]
fn 기록한_교시와_아직인_교시를_구별한다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let subject = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);

    // 2교시는 불렀고 빠진 학생이 없다. 5교시는 아직 부르지 않았다.
    // 결석자 행만으로는 이 둘이 구별되지 않아 수업 칸을 따로 둔다.
    conn.execute(
        "INSERT INTO subject_session (class_id, date, slot)
         VALUES (?1, '2026-09-11', '2')",
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
    let s = school_id(&conn);
    let subject = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);
    let insert = || {
        conn.execute(
            "INSERT INTO subject_session (class_id, date, slot)
             VALUES (?1, '2026-09-11', '2')",
            rusqlite::params![subject],
        )
    };
    insert().unwrap();
    assert!(insert().is_err());
}

#[test]
fn 수업_칸을_지우면_결석자도_함께_사라진다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let subject = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);
    let student = insert_student_at(&conn, s, 3, 6, 1, "학생1");
    join_class(&conn, subject, student);

    conn.execute(
        "INSERT INTO subject_session (class_id, date, slot)
         VALUES (?1, '2026-09-11', '2')",
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

#[test]
fn 하루에_차시를_교시마다_따로_더한다() {
    // 오늘 1교시와 3교시에 같은 강좌가 들어간다. 두 차시는 서로 독립된 칸이다 —
    // 1교시에 빠진 학생이 3교시에는 와 있을 수 있다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let subject = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);
    for slot in ["1", "3"] {
        conn.execute(
            "INSERT INTO subject_session (class_id, date, slot) VALUES (?1, '2026-09-11', ?2)",
            rusqlite::params![subject, slot],
        )
        .unwrap();
    }

    // 교시가 한 자리라 텍스트 정렬이 곧 교시 순서다.
    let slots: Vec<String> = conn
        .prepare(
            "SELECT slot FROM subject_session
              WHERE class_id = ?1 AND date = '2026-09-11' ORDER BY slot",
        )
        .unwrap()
        .query_map([subject], |r| r.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(slots, vec!["1".to_string(), "3".to_string()]);
}

#[test]
fn 교과_차시는_조회와_종례를_받지_않는다() {
    // 조회 · 종례는 담임이 하루의 양 끝에서 보는 것이지 누가 가르치는 시간이 아니다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let subject = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);
    for bad in ["조회", "종례", "0", "10", ""] {
        assert!(
            conn.execute(
                "INSERT INTO subject_session (class_id, date, slot) VALUES (?1, '2026-09-11', ?2)",
                rusqlite::params![subject, bad],
            )
            .is_err(),
            "'{bad}'은 교시가 아니다"
        );
    }
}

#[test]
fn 교과_차시를_담임_학급으로_옮길_수_없다() {
    // INSERT만 막으면 UPDATE로 새어 들어간다. 담임 쪽 트리거와 짝이다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let home = insert_class(&conn, s, "homeroom", "3학년 6반", Some(3), Some(6));
    let subject = insert_class(&conn, s, "subject", "지구과학Ⅰ", None, None);
    conn.execute(
        "INSERT INTO subject_session (class_id, date, slot) VALUES (?1, '2026-09-11', '2')",
        rusqlite::params![subject],
    )
    .unwrap();
    let session = conn.last_insert_rowid();

    assert!(conn
        .execute(
            "UPDATE subject_session SET class_id = ?1 WHERE id = ?2",
            rusqlite::params![home, session],
        )
        .is_err());
}

// ── 연락처 ────────────────────────────────────────────────────

#[test]
fn 연락처는_학생마다_있는_만큼만_담는다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let only_self = insert_student_at(&conn, s, 3, 6, 1, "학생1");
    let both = insert_student_at(&conn, s, 3, 6, 2, "학생2");

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
fn 먼저_걸_번호가_설정되어_있다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let student = insert_student_at(&conn, s, 3, 6, 1, "학생1");
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
    let s = school_id(&conn);
    let student = insert_student_at(&conn, s, 3, 6, 1, "학생1");
    let blank = |kind: &str, phone: &str| {
        conn.execute(
            "INSERT INTO contact (student_id, type, phone) VALUES (?1, ?2, ?3)",
            rusqlite::params![student, kind, phone],
        )
    };
    assert!(blank("", "010-0000-0001").is_err());
    assert!(blank("모", "").is_err());
}

#[test]
fn 담당_학급과_강좌에_명단_인원이_함께_온다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let class = insert_class(&conn, s, "subject", "인공지능기초A", None, None);
    // 반이 섞인 강좌. 이름만으로는 어느 쪽인지 구별하지 못할 때 이 숫자가 구별한다.
    for (grade, class_no, number) in [(3, 1, 4), (3, 6, 11), (2, 3, 20)] {
        let student = insert_student_at(&conn, s, grade, class_no, number, "학생");
        join_class(&conn, class, student);
    }
    // 명단에서 제외한 학생은 세지 않는다. 줄은 남지만 지금 명단은 아니다.
    let gone = insert_student_at(&conn, s, 3, 2, 7, "학생");
    join_class(&conn, class, gone);
    conn.execute(
        "UPDATE class_member SET left_on = '2026-09-01' WHERE student_id = ?1",
        [gone],
    )
    .unwrap();

    let rows = get_teaching_classes_impl(&conn, s, None).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].member_count, 3);
}

// ── 학교가 범위다 ─────────────────────────────────────────────

/// **담당 학급 · 강좌는 학교 안에 있다.** 학년도는 학교가 들고 있으므로 목록도 학교로 묻는다.
///
/// 순회 교사가 A학교에서는 담임+교과, B학교에서는 교과만인 것이 정상이다 —
/// 두 학교의 목록이 섞이면 어느 학교의 화면인지 말할 수 없다.
#[test]
fn 담당_학급과_강좌는_학교_밖으로_유출되지_않는다() {
    let conn = setup_test_db();
    let a = school_id(&conn);
    let b = insert_school(&conn, year_id(&conn), "나다고등학교");

    insert_class(&conn, a, "homeroom", "3학년 6반", Some(3), Some(6));
    insert_class(&conn, a, "subject", "지구과학Ⅰ", None, None);
    insert_class(&conn, b, "subject", "인공지능기초", None, None);

    assert_eq!(class_names(&conn, a), vec!["3학년 6반", "지구과학Ⅰ"]);
    assert_eq!(class_names(&conn, b), vec!["인공지능기초"]);

    // B학교에는 담임이 없다 — 그 교사는 B에서 비담임이다.
    assert!(get_teaching_classes_impl(&conn, b, Some("homeroom"))
        .unwrap()
        .is_empty());
}

/// 학교가 다르면 같은 이름을 다시 쓸 수 있다. `ux_class_name_active`가 학교까지 본다.
#[test]
fn 학교가_다르면_같은_이름을_맡을_수_있다() {
    let conn = setup_test_db();
    let a = school_id(&conn);
    let b = insert_school(&conn, year_id(&conn), "나다고등학교");

    create_teaching_class_impl(&conn, a, "homeroom", "3학년 6반", Some(3), Some(6), None, TODAY)
        .unwrap();
    create_teaching_class_impl(&conn, b, "homeroom", "3학년 6반", Some(3), Some(6), None, TODAY)
        .unwrap();
}

/// 학교를 지우면 그 학교의 담당 학급 · 강좌도 함께 사라진다. **그래서 학교는 지우지 않고 내린다**
/// (`retire_school`). 계층이 CASCADE로 연결되어 있다는 사실을 여기서 못 박는다.
#[test]
fn 학교를_지우면_담당_학급과_강좌도_함께_사라진다() {
    let conn = setup_test_db();
    let a = school_id(&conn);
    let b = insert_school(&conn, year_id(&conn), "나다고등학교");
    let mine = insert_class(&conn, a, "homeroom", "3학년 6반", Some(3), Some(6));
    insert_class(&conn, b, "subject", "인공지능기초", None, None);
    let student = enroll(&conn, mine, a, 1, "학생1");
    stamp(&conn, mine, student);

    conn.execute("DELETE FROM school WHERE id = ?1", [b]).unwrap();

    assert_eq!(class_names(&conn, a), vec!["3학년 6반"]);
    assert!(get_teaching_classes_impl(&conn, b, None).unwrap().is_empty());
    // 내 학교의 기록은 그대로다.
    let spans: i64 = conn
        .query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(spans, 1);
}

// ── 강좌 묶음 이름표 ──────────────────────────────────────────
//
// `프로그래밍A · B · C`를 묶는 이름이다. **교과 - 분반 2단 구조가 아니다** —
// 강좌는 저마다 독립한 행이고 이 이름표는 화면에서 함께 보이게 할 뿐이다.

fn tag_names(conn: &Connection, school: i64) -> Vec<String> {
    get_class_tags_impl(conn, school)
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect()
}

/// 이름표를 단 강좌가 이름표 이름을 함께 들고 온다. 화면이 그것으로 묶어 보인다.
#[test]
fn 강좌가_묶음_이름을_함께_들고_온다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let tag = create_class_tag_impl(&conn, s, "  프로그래밍  ").unwrap();

    let a =
        create_teaching_class_impl(&conn, s, "subject", "프로그래밍A", None, None, Some(tag), TODAY)
            .unwrap();
    create_teaching_class_impl(&conn, s, "subject", "프로그래밍B", None, None, Some(tag), TODAY)
        .unwrap();
    // 묶을 것이 없는 과목이 더 많다. 이름표는 없어도 된다.
    create_teaching_class_impl(&conn, s, "subject", "지구과학Ⅰ", None, None, None, TODAY).unwrap();

    let rows = get_teaching_classes_impl(&conn, s, Some("subject")).unwrap();
    let seen: Vec<(&str, Option<&str>)> = rows
        .iter()
        .map(|c| (c.name.as_str(), c.group_tag_name.as_deref()))
        .collect();
    assert_eq!(
        seen,
        vec![
            ("프로그래밍A", Some("프로그래밍")),
            ("프로그래밍B", Some("프로그래밍")),
            ("지구과학Ⅰ", None),
        ]
    );
    assert_eq!(rows[0].group_tag_id, Some(tag));

    // 묶음 목록은 몇 강좌가 달려 있는지 함께 센다 — 지우기 전에 무엇이 풀리는지 보여준다.
    let tags = get_class_tags_impl(&conn, s).unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "프로그래밍");
    assert_eq!(tags[0].class_count, 2);

    // 나중에 달아도 된다.
    update_teaching_class_impl(&conn, a, "프로그래밍A", None, None, None).unwrap();
    assert_eq!(get_class_tags_impl(&conn, s).unwrap()[0].class_count, 1);
}

#[test]
fn 묶음_이름은_비거나_겹칠_수_없다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    create_class_tag_impl(&conn, s, "프로그래밍").unwrap();

    assert!(create_class_tag_impl(&conn, s, "   ")
        .unwrap_err()
        .contains("비어"));
    let err = create_class_tag_impl(&conn, s, "프로그래밍").unwrap_err();
    assert!(err.contains("이미 있는 강좌 묶음"), "영문 원문이 새어나왔다: {err}");

    // 학교가 다르면 같은 이름을 쓸 수 있다.
    let other = insert_school(&conn, year_id(&conn), "나다고등학교");
    assert!(create_class_tag_impl(&conn, other, "프로그래밍").is_ok());
    assert_eq!(tag_names(&conn, s), vec!["프로그래밍"]);
}

/// 이름표는 세는 데 쓰지 않으므로 **UPDATE로 고친다.** 학급 이름과 같은 이유다 —
/// 과거 기록이 뜻으로 가리키는 것이 아니다.
#[test]
fn 묶음_이름은_그_자리에서_고친다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let tag = create_class_tag_impl(&conn, s, "프로그래밍").unwrap();
    let class =
        create_teaching_class_impl(&conn, s, "subject", "프로그래밍A", None, None, Some(tag), TODAY)
            .unwrap();

    rename_class_tag_impl(&conn, tag, "  정보 · 프로그래밍  ").unwrap();

    assert_eq!(tag_names(&conn, s), vec!["정보 · 프로그래밍"]);
    let rows = get_teaching_classes_impl(&conn, s, Some("subject")).unwrap();
    assert_eq!(rows[0].id, class);
    assert_eq!(rows[0].group_tag_id, Some(tag), "강좌가 가리키는 곳은 그대로다");
    assert_eq!(rows[0].group_tag_name.as_deref(), Some("정보 · 프로그래밍"));

    assert!(rename_class_tag_impl(&conn, tag, "  ")
        .unwrap_err()
        .contains("비어"));
    assert!(rename_class_tag_impl(&conn, 9999, "무엇")
        .unwrap_err()
        .contains("강좌 묶음을 찾을 수 없습니다"));
}

/// **묶음을 지워도 강좌는 남는다.** `ON DELETE SET NULL`이라 이름표만 풀린다 —
/// 이름표를 지우는 동작이 강좌와 그 차시를 지우는 동작이어서는 안 된다.
#[test]
fn 묶음을_지워도_강좌는_남는다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let tag = create_class_tag_impl(&conn, s, "프로그래밍").unwrap();
    create_teaching_class_impl(&conn, s, "subject", "프로그래밍A", None, None, Some(tag), TODAY)
        .unwrap();

    delete_class_tag_impl(&conn, tag).unwrap();

    assert!(tag_names(&conn, s).is_empty());
    let rows = get_teaching_classes_impl(&conn, s, Some("subject")).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "프로그래밍A");
    assert_eq!(rows[0].group_tag_id, None);
    assert_eq!(rows[0].group_tag_name, None);

    assert!(delete_class_tag_impl(&conn, tag)
        .unwrap_err()
        .contains("강좌 묶음을 찾을 수 없습니다"));
}

/// 남의 학교 이름표를 단 강좌는 목록에서 이름 없는 묶음으로 보인다 —
/// 어떤 입력의 결과도 아닌 상태다. FK는 존재만 볼 뿐 학교를 보지 않으므로 여기서 막는다.
#[test]
fn 다른_학교의_묶음은_달_수_없다() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let other = insert_school(&conn, year_id(&conn), "나다고등학교");
    let theirs = create_class_tag_impl(&conn, other, "남의 묶음").unwrap();

    let err = create_teaching_class_impl(
        &conn,
        s,
        "subject",
        "프로그래밍A",
        None,
        None,
        Some(theirs),
        TODAY,
    )
    .unwrap_err();
    assert!(err.contains("이 학교의 강좌 묶음이 아닙니다"), "{err}");

    // 아예 없는 묶음도 같은 문장으로 막는다.
    let err = create_teaching_class_impl(
        &conn,
        s,
        "subject",
        "프로그래밍A",
        None,
        None,
        Some(9999),
        TODAY,
    )
    .unwrap_err();
    assert!(err.contains("이 학교의 강좌 묶음이 아닙니다"), "{err}");
    assert!(class_names(&conn, s).is_empty(), "거절된 강좌가 새어 들어갔다");

    // 고치는 길로도 막힌다 — 확인을 나누어 두면 한쪽으로만 들어온다.
    let mine =
        create_teaching_class_impl(&conn, s, "subject", "프로그래밍A", None, None, None, TODAY)
            .unwrap();
    assert!(
        update_teaching_class_impl(&conn, mine, "프로그래밍A", None, None, Some(theirs))
            .unwrap_err()
            .contains("이 학교의 강좌 묶음이 아닙니다")
    );
}
