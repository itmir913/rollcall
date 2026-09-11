//! 명단과 연락처.
//!
//! 재가져오기는 **교체가 아니라 차분**이다. 사라진 번호를 지우면 그 학생의 출결
//! 기록이 함께 사라지므로, 명단에서 빠진 번호는 `class_member.left_on`을 채우는 것으로
//! 끝난다. **학생의 `enrolled_to`가 아니다** — 내 명단에서 빠진 것과 학교를 떠난 것은
//! 다른 일이다.
//!
//! 가져오기는 두 가지 일을 한다. 학생을 만드는 일과 내 명단에 잇는 일이고,
//! 이미 그 학적 자리에 있는 학생은 다시 만들지 않는다.

use crate::commands::class::member_rows_on;
use crate::commands::student::*;
use crate::tests::*;
use crate::types::{ContactItem, RosterDiffRow, RosterEntry};
use rusqlite::Connection;

fn entry(number: i64, name: &str) -> RosterEntry {
    RosterEntry {
        grade: None,
        class_no: None,
        number,
        name: name.to_string(),
    }
}

fn entry_of(grade: Option<i64>, class_no: Option<i64>, number: i64, name: &str) -> RosterEntry {
    RosterEntry {
        grade,
        class_no,
        number,
        name: name.to_string(),
    }
}

fn contact(kind: &str, phone: &str) -> ContactItem {
    ContactItem {
        id: 0,
        kind: kind.into(),
        phone: phone.into(),
        memo: String::new(),
        sort_order: 0,
    }
}

/// 학교 하나를 더 만든다. 순회 교사가 학교를 둘 등록한 상태를 흉내낸다.
fn another_school(conn: &Connection) -> i64 {
    conn.execute(
        "INSERT INTO school (name, max_slot, due_days, due_skip_offdays, sort_order, active)
         VALUES ('옆 학교', 6, 5, 1, 20, 1)",
        [],
    )
    .unwrap();
    conn.last_insert_rowid()
}

// ── 명렬표가 말하는 학급 ──────────────────────────────────────
//
// 파일 파싱은 프론트(`frontend/src/services/rosterFile.js`)가 한다. 여기로 오는 것은 이미
// (학년, 반, 번호, 이름)으로 정리된 목록이고, 남은 판단은 "어느 학급인가"뿐이다.

#[test]
fn class_is_confirmed_when_every_row_agrees() {
    let class = detect_class(&[
        entry_of(Some(3), Some(6), 1, "김철수"),
        entry_of(Some(3), Some(6), 2, "이영희"),
    ]);
    assert_eq!(class.grade, Some(3));
    assert_eq!(class.class_no, Some(6));
    assert!(!class.mixed);
}

#[test]
fn a_mixed_roster_is_flagged_for_the_teacher() {
    let class = detect_class(&[
        entry_of(Some(3), Some(6), 1, "김철수"),
        entry_of(Some(3), Some(7), 1, "최지훈"),
    ]);
    assert!(class.mixed);
    assert_eq!(class.class_no, None);
    // 학년은 같으므로 그것만은 확정된다.
    assert_eq!(class.grade, Some(3));
}

#[test]
fn a_two_column_roster_leaves_the_class_unknown() {
    // 파일에 학년·반 열이 없으면 추측하지 않는다. 화면이 묻는다.
    let class = detect_class(&[entry_of(None, None, 1, "김철수")]);
    assert_eq!(class.grade, None);
    assert_eq!(class.class_no, None);
    assert!(!class.mixed);
}

#[test]
fn an_empty_roster_says_nothing() {
    let class = detect_class(&[]);
    assert_eq!(class.grade, None);
    assert!(!class.mixed);
}

// ── 차분 ──────────────────────────────────────────────────────

#[test]
fn new_number_is_added() {
    let current = vec![];
    let rows = diff_roster(&current, &[entry(1, "김철수")]);
    assert_eq!(rows[0].action, "added");
    assert_eq!(rows[0].student_id, None);
}

#[test]
fn same_number_and_name_is_unchanged() {
    let current = vec![(10, 1, "김철수".to_string())];
    let rows = diff_roster(&current, &[entry(1, "김철수")]);
    assert_eq!(rows[0].action, "unchanged");
}

#[test]
fn same_number_different_name_needs_teacher_decision() {
    // 개명인지, 번호를 물려받은 전입인지, 오타인지 프로그램은 판정할 수 없다.
    let current = vec![(10, 1, "김철수".to_string())];
    let rows = diff_roster(&current, &[entry(1, "김철호")]);
    assert_eq!(rows[0].action, "renamed");
    assert_eq!(rows[0].current_name.as_deref(), Some("김철수"));
    assert_eq!(rows[0].incoming_name.as_deref(), Some("김철호"));
}

#[test]
fn missing_number_is_withdrawn_not_deleted() {
    let current = vec![(10, 1, "김철수".to_string()), (11, 2, "이영희".to_string())];
    let rows = diff_roster(&current, &[entry(1, "김철수")]);
    let withdrawn: Vec<_> = rows.iter().filter(|r| r.action == "withdrawn").collect();
    assert_eq!(withdrawn.len(), 1);
    assert_eq!(withdrawn[0].number, 2);
    assert_eq!(withdrawn[0].incoming_name, None);
}

#[test]
fn diff_is_sorted_by_number() {
    let current = vec![(10, 3, "박민수".to_string())];
    let rows = diff_roster(&current, &[entry(2, "이영희"), entry(1, "김철수")]);
    assert_eq!(
        rows.iter().map(|r| r.number).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

// ── 적용 ──────────────────────────────────────────────────────

#[test]
fn apply_adds_and_withdraws() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    enroll(&conn, class, year, 1, "김철수");
    enroll(&conn, class, year, 2, "이영희");

    let incoming = vec![entry(1, "김철수"), entry(3, "박민수")];
    let rows = preview_roster_impl(&conn, class, &incoming).unwrap();
    let result = apply_roster_impl(&conn, class, "2026-09-01", &rows).unwrap();

    assert_eq!(result.added, 1);
    assert_eq!(result.withdrawn, 1);
    assert_eq!(result.renamed, 0);

    let active = get_students_impl(&conn, class).unwrap();
    assert_eq!(
        active.iter().map(|s| s.number).collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert!(active.iter().all(|s| s.school_id == school));
}

#[test]
fn 다시_가져와도_학생이_중복으로_생기지_않는다() {
    // 같은 학적 자리의 학생은 한 행이다. 가져오기가 학생을 다시 만들면 같은 사람이
    // 둘이 되고, 지난 기록이 어느 쪽에 붙었는지 알 수 없게 된다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);

    let incoming = vec![entry(1, "김철수"), entry(2, "이영희")];
    apply_roster_impl(
        &conn,
        class,
        "2026-03-02",
        &preview_roster_impl(&conn, class, &incoming).unwrap(),
    )
    .unwrap();
    assert_eq!(student_count(&conn), 2);
    assert_eq!(member_count(&conn), 2);

    // 같은 파일을 다시 가져온다. 전부 unchanged이므로 아무것도 늘지 않는다.
    let again = preview_roster_impl(&conn, class, &incoming).unwrap();
    assert!(again.iter().all(|r| r.action == "unchanged"));
    apply_roster_impl(&conn, class, "2026-09-01", &again).unwrap();
    assert_eq!(student_count(&conn), 2, "학생 행은 그대로다");
    assert_eq!(member_count(&conn), 2);

    // 학적은 이미 있는데 내 명단에는 아직 없는 학생. 가져오기는 그 행을 찾아
    // 명단에만 잇는다 — 같은 자리에 학생을 하나 더 만들면 안 된다.
    let existing = insert_student_at(&conn, year, 3, 6, 3, "박민수");
    let grown = vec![entry(1, "김철수"), entry(2, "이영희"), entry(3, "박민수")];
    let rows = preview_roster_impl(&conn, class, &grown).unwrap();
    assert_eq!(rows[2].action, "added", "내 명단에는 아직 없다");
    apply_roster_impl(&conn, class, "2026-09-01", &rows).unwrap();

    assert_eq!(student_count(&conn), 3, "학생 행은 하나만 늘었다");
    assert_eq!(member_count(&conn), 3, "소속이 하나 늘었다");
    let joined: i64 = conn
        .query_row(
            "SELECT student_id FROM class_member WHERE class_id = ?1 ORDER BY id DESC LIMIT 1",
            rusqlite::params![class],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(joined, existing, "이미 있던 그 학생을 이었다");
}

fn student_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM student", [], |r| r.get(0))
        .unwrap()
}

fn member_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM class_member", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn apply_renames_when_the_teacher_says_so() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    enroll(&conn, class, year, 1, "김철수");

    let rows = preview_roster_impl(&conn, class, &[entry(1, "김철호")]).unwrap();
    assert_eq!(rows[0].action, "renamed");
    let result = apply_roster_impl(&conn, class, "2026-09-01", &rows).unwrap();

    assert_eq!(result.renamed, 1);
    let active = get_students_impl(&conn, class).unwrap();
    assert_eq!(active[0].name, "김철호");
    // 개명은 같은 행이다. 새 학생이 생기지 않는다.
    assert_eq!(active.len(), 1);
}

#[test]
fn withdrawn_student_keeps_its_row() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let id = enroll(&conn, class, year, 2, "이영희");

    let rows = preview_roster_impl(&conn, class, &[]).unwrap();
    apply_roster_impl(&conn, class, "2026-09-01", &rows).unwrap();

    // 닫히는 것은 **소속 기간**이다. 학적은 그대로 열려 있다 —
    // 담임이 아는 것은 "내 명단에 더는 없다"까지다.
    let left_on: Option<String> = conn
        .query_row(
            "SELECT left_on FROM class_member WHERE student_id = ?1",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(left_on.as_deref(), Some("2026-09-01"));

    let enrolled_to: Option<String> = conn
        .query_row(
            "SELECT enrolled_to FROM student WHERE id = ?1",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(enrolled_to, None, "학교를 떠났다고 말할 근거가 없다");
    assert!(get_students_impl(&conn, class).unwrap().is_empty());
}

#[test]
fn number_can_be_reused_after_withdrawal_in_one_apply() {
    // 전출을 먼저 처리하지 않으면 부분 유니크 인덱스가 막는다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let old = enroll(&conn, class, year, 5, "이영희");

    let rows = vec![
        RosterDiffRow {
            number: 5,
            incoming_name: None,
            current_name: Some("이영희".into()),
            student_id: Some(old),
            action: "withdrawn".into(),
        },
        RosterDiffRow {
            number: 5,
            incoming_name: Some("최지훈".into()),
            current_name: None,
            student_id: None,
            action: "added".into(),
        },
    ];

    apply_roster_impl(&conn, class, "2026-09-01", &rows).unwrap();
    let active = get_students_impl(&conn, class).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].name, "최지훈");
    assert_ne!(active[0].id, old);
}

#[test]
fn apply_rolls_back_on_error() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);

    let rows = vec![
        RosterDiffRow {
            number: 1,
            incoming_name: Some("김철수".into()),
            current_name: None,
            student_id: None,
            action: "added".into(),
        },
        RosterDiffRow {
            number: 2,
            incoming_name: Some("  ".into()), // 실패 유발
            current_name: None,
            student_id: None,
            action: "added".into(),
        },
    ];
    assert!(apply_roster_impl(&conn, class, "2026-09-01", &rows).is_err());
    assert!(get_students_impl(&conn, class).unwrap().is_empty());

    // 트랜잭션이 열린 채 남지 않았는지 — 다음 쓰기가 정상 동작해야 한다.
    enroll(&conn, class, year, 1, "김철수");
    assert_eq!(get_students_impl(&conn, class).unwrap().len(), 1);
}

#[test]
fn apply_refuses_a_row_that_points_at_another_class() {
    // 학급을 잘못 고른 채 적용하면 옆 반 명단이 조용히 정리된다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let other = insert_class(&conn, year, "homeroom", "3학년 7반", Some(3), Some(7));
    let id = enroll(&conn, class, year, 1, "김철수");

    let rows = vec![RosterDiffRow {
        number: 1,
        incoming_name: None,
        current_name: Some("김철수".into()),
        student_id: Some(id),
        action: "withdrawn".into(),
    }];
    let err = apply_roster_impl(&conn, other, "2026-09-01", &rows).unwrap_err();
    assert!(err.contains("찾을 수 없습니다"), "{err}");

    // 원래 학급의 명단은 그대로다.
    assert_eq!(get_students_impl(&conn, class).unwrap().len(), 1);
}

// ── 학교가 명단을 구분한다 ────────────────────────────────────

#[test]
fn two_schools_can_hold_the_same_class_without_mixing() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let other_school = another_school(&conn);
    let theirs_class = conn
        .query_row(
            "INSERT INTO teaching_class (school_id, year_id, role, name, grade, class_no, valid_from)
             VALUES (?1, ?2, 'homeroom', '옆 학교 3학년 6반', 3, 6, '2026-03-02') RETURNING id",
            rusqlite::params![other_school, year],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();

    enroll(&conn, class, year, 1, "김철수");
    apply_roster_impl(
        &conn,
        theirs_class,
        "2026-03-02",
        &[RosterDiffRow {
            number: 1,
            incoming_name: Some("최지훈".into()),
            current_name: None,
            student_id: None,
            action: "added".into(),
        }],
    )
    .unwrap();

    let mine = get_students_impl(&conn, class).unwrap();
    let theirs = get_students_impl(&conn, theirs_class).unwrap();
    assert_eq!(mine.len(), 1);
    assert_eq!(theirs.len(), 1);
    assert_eq!(mine[0].name, "김철수");
    assert_eq!(theirs[0].name, "최지훈");
}

#[test]
fn a_reimport_only_sees_its_own_class() {
    // 명단으로 거르지 않으면 옆 반이 통째로 정리 대상이 된다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let other = insert_class(&conn, year, "homeroom", "3학년 7반", Some(3), Some(7));
    enroll(&conn, class, year, 1, "김철수");

    let rows = preview_roster_impl(&conn, other, &[entry(1, "최지훈")]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].action, "added");
}

// ── 그날 명단 ─────────────────────────────────────────────────

#[test]
fn grid_shows_only_students_on_the_roster_that_date() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let id = enroll(&conn, class, year, 1, "김철수");
    enroll(&conn, class, year, 2, "이영희");
    withdraw_student_impl(&conn, class, id, "2026-06-30").unwrap();

    let before = member_rows_on(&conn, class, "2026-06-01").unwrap();
    assert_eq!(before.len(), 2);

    let after = member_rows_on(&conn, class, "2026-07-01").unwrap();
    assert_eq!(after.len(), 1);

    // 경계일에는 이미 명단 밖이다. `valid_to`와 같은 규칙이고, 개요·기록 화면이
    // 세는 인원도 같은 식이라 여기만 다르면 같은 날 수가 어긋난다.
    let on_day = member_rows_on(&conn, class, "2026-06-30").unwrap();
    assert_eq!(on_day.len(), 1);
}

#[test]
fn a_number_handed_over_on_one_day_shows_one_student_that_day() {
    // 나간 것과 들어온 것이 한 번의 적용에 함께 오면 두 줄이 같은 날짜를 가진다.
    // 경계일을 명단으로 보면 그날 격자에 5번이 두 명 뜬다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let old = enroll(&conn, class, year, 5, "이영희");

    let rows = vec![
        RosterDiffRow {
            number: 5,
            incoming_name: None,
            current_name: Some("이영희".into()),
            student_id: Some(old),
            action: "withdrawn".into(),
        },
        RosterDiffRow {
            number: 5,
            incoming_name: Some("최지훈".into()),
            current_name: None,
            student_id: None,
            action: "added".into(),
        },
    ];
    apply_roster_impl(&conn, class, "2026-09-01", &rows).unwrap();

    let on_day = member_rows_on(&conn, class, "2026-09-01").unwrap();
    assert_eq!(on_day.len(), 1);
    assert_eq!(on_day[0].2, "최지훈");
}

#[test]
fn withdrawing_a_student_of_another_class_fails_instead_of_doing_nothing() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let other = insert_class(&conn, year, "homeroom", "3학년 7반", Some(3), Some(7));
    let id = enroll(&conn, class, year, 1, "김철수");

    let err = withdraw_student_impl(&conn, other, id, "2026-06-30").unwrap_err();
    assert!(err.contains("학생을 찾을 수 없습니다"), "{err}");
}

// ── 개별 수정 ─────────────────────────────────────────────────

#[test]
fn duplicate_active_number_is_rejected_in_korean() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    enroll(&conn, class, year, 1, "김철수");
    let id = enroll(&conn, class, year, 2, "이영희");

    let err = update_student_impl(&conn, class, id, 1, "이영희").unwrap_err();
    assert!(err.contains("이미 같은 번호"), "영문 원문이 새어나왔다: {err}");
}

#[test]
fn updating_a_missing_student_says_so() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let err = update_student_impl(&conn, class, 9999, 1, "김철수").unwrap_err();
    assert!(err.contains("학생을 찾을 수 없습니다"), "{err}");
}

#[test]
fn a_number_below_one_and_a_blank_name_are_rejected() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let id = enroll(&conn, class, year, 1, "김철수");

    assert!(update_student_impl(&conn, class, id, 0, "김철수").is_err());
    assert!(update_student_impl(&conn, class, id, 1, "   ").is_err());
    // 실패했으니 원래 값이 그대로여야 한다.
    let active = get_students_impl(&conn, class).unwrap();
    assert_eq!(active[0].name, "김철수");
    assert_eq!(active[0].number, 1);
}

// ── 연락처 ────────────────────────────────────────────────────
//
// 연락처는 관계(label)와 번호(value), 그리고 순서뿐이다. 덧붙일 말은 label에 적는다.

#[test]
fn a_student_can_hold_many_contacts_in_order() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    set_contacts_impl(
        &conn,
        sid,
        &[
            contact("어머니", "010-1111-1111"),
            contact("학생 본인", "010-2222-2222"),
        ],
    )
    .unwrap();

    let contacts = get_contacts_impl(&conn, sid).unwrap();
    assert_eq!(contacts.len(), 2);
    assert_eq!(contacts[0].kind, "어머니");
    assert_eq!(contacts[1].kind, "학생 본인");
    // 순서는 저장 시점의 배열 순서를 따른다.
    assert_eq!(contacts[0].sort_order, 0);
    assert_eq!(contacts[1].sort_order, 1);
}

#[test]
fn saving_contacts_replaces_the_whole_list() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    set_contacts_impl(
        &conn,
        sid,
        &[
            contact("어머니", "010-0000-0000"),
            contact("아버지", "010-0000-0000"),
        ],
    )
    .unwrap();
    set_contacts_impl(&conn, sid, &[contact("어머니", "010-0000-0000")]).unwrap();

    let contacts = get_contacts_impl(&conn, sid).unwrap();
    assert_eq!(contacts.len(), 1);
}

#[test]
fn an_empty_list_clears_the_contacts() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    set_contacts_impl(&conn, sid, &[contact("어머니", "010-0000-0000")]).unwrap();
    set_contacts_impl(&conn, sid, &[]).unwrap();
    assert!(get_contacts_impl(&conn, sid).unwrap().is_empty());
}

#[test]
fn a_blank_contact_is_rejected_and_nothing_is_saved() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    assert!(set_contacts_impl(&conn, sid, &[contact("어머니", "  ")]).is_err());
    assert!(set_contacts_impl(&conn, sid, &[contact(" ", "010-0000-0000")]).is_err());
    assert!(get_contacts_impl(&conn, sid).unwrap().is_empty());
}

#[test]
fn a_rejected_save_leaves_the_previous_list_intact() {
    // 검증이 트랜잭션 앞에 있으므로 지우기부터 하고 실패하는 일이 없어야 한다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    set_contacts_impl(&conn, sid, &[contact("어머니", "010-1111-1111")]).unwrap();

    assert!(set_contacts_impl(
        &conn,
        sid,
        &[contact("아버지", "010-2222-2222"), contact("학생", "  ")]
    )
    .is_err());

    let contacts = get_contacts_impl(&conn, sid).unwrap();
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].kind, "어머니");
}

#[test]
fn contacts_go_away_with_the_student_row() {
    // 전출을 삭제로 처리하지 않는 이유가 여기에도 있다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    set_contacts_impl(&conn, sid, &[contact("어머니", "010-0000-0000")]).unwrap();

    conn.execute("DELETE FROM student WHERE id = ?1", rusqlite::params![sid])
        .unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM contact", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

// ── 학급 ──────────────────────────────────────────────────────
//
// 학급 목록은 이제 명단에서 끌어내지 않는다. **내가 맡은 것이 행이다**(`teaching_class`).
// 학생이 아직 하나도 없는 반도 골라야 하고, 교과 강좌는 반이 섞여 학적으로는
// 이름조차 만들 수 없기 때문이다. 그쪽 시험은 `class_tests.rs`에 있다.

#[test]
fn 명단이_비어도_학급은_남는다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let class = homeroom(&conn, year);
    let id = enroll(&conn, class, year, 1, "김철수");
    withdraw_student_impl(&conn, class, id, "2026-06-30").unwrap();

    assert!(get_students_impl(&conn, class).unwrap().is_empty());
    let left: i64 = conn
        .query_row("SELECT COUNT(*) FROM teaching_class", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 1, "명렬표를 다시 가져올 자리가 남아야 한다");
}
