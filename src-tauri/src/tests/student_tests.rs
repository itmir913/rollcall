//! 명단과 연락처.
//!
//! 재가져오기는 **교체가 아니라 차분**이다. 사라진 번호를 지우면 그 학생의 출결
//! 기록이 함께 사라지므로, 전출은 `enrolled_to`를 채우는 것으로 끝난다.

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

fn contact(label: &str, value: &str) -> ContactItem {
    ContactItem {
        id: 0,
        label: label.into(),
        value: value.into(),
        note: None,
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
    insert_student(&conn, year, 1, "김철수");
    insert_student(&conn, year, 2, "이영희");

    let incoming = vec![entry(1, "김철수"), entry(3, "박민수")];
    let rows = preview_roster_impl(&conn, school, year, 3, 6, &incoming).unwrap();
    let result = apply_roster_impl(&conn, school, year, 3, 6, "2026-09-01", &rows).unwrap();

    assert_eq!(result.added, 1);
    assert_eq!(result.withdrawn, 1);
    assert_eq!(result.renamed, 0);

    let active = get_students_impl(&conn, school, year, 3, 6).unwrap();
    assert_eq!(
        active.iter().map(|s| s.number).collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert!(active.iter().all(|s| s.school_id == school));
}

#[test]
fn apply_renames_when_the_teacher_says_so() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");

    let rows = preview_roster_impl(&conn, school, year, 3, 6, &[entry(1, "김철호")]).unwrap();
    assert_eq!(rows[0].action, "renamed");
    let result = apply_roster_impl(&conn, school, year, 3, 6, "2026-09-01", &rows).unwrap();

    assert_eq!(result.renamed, 1);
    let active = get_students_impl(&conn, school, year, 3, 6).unwrap();
    assert_eq!(active[0].name, "김철호");
    // 개명은 같은 행이다. 새 학생이 생기지 않는다.
    assert_eq!(active.len(), 1);
}

#[test]
fn withdrawn_student_keeps_its_row() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let id = insert_student(&conn, year, 2, "이영희");

    let rows = preview_roster_impl(&conn, school, year, 3, 6, &[]).unwrap();
    apply_roster_impl(&conn, school, year, 3, 6, "2026-09-01", &rows).unwrap();

    let enrolled_to: Option<String> = conn
        .query_row(
            "SELECT enrolled_to FROM student WHERE id = ?1",
            rusqlite::params![id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(enrolled_to.as_deref(), Some("2026-09-01"));
}

#[test]
fn number_can_be_reused_after_withdrawal_in_one_apply() {
    // 전출을 먼저 처리하지 않으면 부분 유니크 인덱스가 막는다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let old = insert_student(&conn, year, 5, "이영희");

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

    apply_roster_impl(&conn, school, year, 3, 6, "2026-09-01", &rows).unwrap();
    let active = get_students_impl(&conn, school, year, 3, 6).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].name, "최지훈");
    assert_ne!(active[0].id, old);
}

#[test]
fn apply_rolls_back_on_error() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);

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
    assert!(apply_roster_impl(&conn, school, year, 3, 6, "2026-09-01", &rows).is_err());
    assert!(get_students_impl(&conn, school, year, 3, 6)
        .unwrap()
        .is_empty());

    // 트랜잭션이 열린 채 남지 않았는지 — 다음 쓰기가 정상 동작해야 한다.
    insert_student(&conn, year, 1, "김철수");
    assert_eq!(get_students_impl(&conn, school, year, 3, 6).unwrap().len(), 1);
}

#[test]
fn apply_refuses_a_row_that_points_at_another_school() {
    // 학교를 잘못 고른 채 적용하면 옆 학교 명단이 조용히 전출 처리된다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let other = another_school(&conn);
    let year = insert_year(&conn, 2026);
    let id = insert_student(&conn, year, 1, "김철수");

    let rows = vec![RosterDiffRow {
        number: 1,
        incoming_name: None,
        current_name: Some("김철수".into()),
        student_id: Some(id),
        action: "withdrawn".into(),
    }];
    let err = apply_roster_impl(&conn, other, year, 3, 6, "2026-09-01", &rows).unwrap_err();
    assert!(err.contains("찾을 수 없습니다"), "{err}");

    // 원래 학교의 학생은 그대로 재학 중이다.
    assert_eq!(get_students_impl(&conn, school, year, 3, 6).unwrap().len(), 1);
}

// ── 학교가 명단을 구분한다 ────────────────────────────────────

#[test]
fn two_schools_can_hold_the_same_class_without_mixing() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let other = another_school(&conn);
    let year = insert_year(&conn, 2026);

    insert_student(&conn, year, 1, "김철수");
    apply_roster_impl(
        &conn,
        other,
        year,
        3,
        6,
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

    let mine = get_students_impl(&conn, school, year, 3, 6).unwrap();
    let theirs = get_students_impl(&conn, other, year, 3, 6).unwrap();
    assert_eq!(mine.len(), 1);
    assert_eq!(theirs.len(), 1);
    assert_eq!(mine[0].name, "김철수");
    assert_eq!(theirs[0].name, "최지훈");
}

#[test]
fn a_reimport_only_sees_its_own_school() {
    // 학교로 거르지 않으면 옆 학교의 같은 반이 통째로 전출 대상이 된다.
    let conn = setup_test_db();
    let other = another_school(&conn);
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수"); // 시드 학교의 3학년 6반 1번

    let rows = preview_roster_impl(&conn, other, year, 3, 6, &[entry(1, "최지훈")]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].action, "added");
}

// ── 그날 재학생 ───────────────────────────────────────────────

#[test]
fn grid_shows_only_students_enrolled_on_that_date() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let id = insert_student(&conn, year, 1, "김철수");
    insert_student(&conn, year, 2, "이영희");
    withdraw_student_impl(&conn, school, id, "2026-06-30").unwrap();

    let before = get_students_on_impl(&conn, school, year, 3, 6, "2026-06-01").unwrap();
    assert_eq!(before.len(), 2);

    let after = get_students_on_impl(&conn, school, year, 3, 6, "2026-07-01").unwrap();
    assert_eq!(after.len(), 1);

    // 경계일에는 이미 전출이다. `valid_to`와 같은 규칙이고, 개요·기록 화면이
    // 세는 재학생도 같은 식이라 여기만 다르면 같은 날 수가 어긋난다.
    let on_day = get_students_on_impl(&conn, school, year, 3, 6, "2026-06-30").unwrap();
    assert_eq!(on_day.len(), 1);
}

#[test]
fn a_number_handed_over_on_one_day_shows_one_student_that_day() {
    // 전출과 전입이 한 번의 적용에 함께 들어오면 두 행이 같은 날짜를 가진다.
    // 경계일을 재학으로 보면 그날 격자에 5번이 두 명 뜬다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let old = insert_student(&conn, year, 5, "이영희");

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
    apply_roster_impl(&conn, school, year, 3, 6, "2026-09-01", &rows).unwrap();

    let on_day = get_students_on_impl(&conn, school, year, 3, 6, "2026-09-01").unwrap();
    assert_eq!(on_day.len(), 1);
    assert_eq!(on_day[0].name, "최지훈");
}

#[test]
fn withdrawing_a_student_of_another_school_fails_instead_of_doing_nothing() {
    let conn = setup_test_db();
    let other = another_school(&conn);
    let year = insert_year(&conn, 2026);
    let id = insert_student(&conn, year, 1, "김철수");

    let err = withdraw_student_impl(&conn, other, id, "2026-06-30").unwrap_err();
    assert!(err.contains("학생을 찾을 수 없습니다"), "{err}");
}

// ── 개별 수정 ─────────────────────────────────────────────────

#[test]
fn duplicate_active_number_is_rejected_in_korean() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");
    let id = insert_student(&conn, year, 2, "이영희");

    let err = update_student_impl(&conn, school, id, 1, "이영희").unwrap_err();
    assert!(err.contains("이미 같은 번호"), "영문 원문이 새어나왔다: {err}");
}

#[test]
fn updating_a_missing_student_says_so() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let err = update_student_impl(&conn, school, 9999, 1, "김철수").unwrap_err();
    assert!(err.contains("학생을 찾을 수 없습니다"), "{err}");
}

#[test]
fn a_number_below_one_and_a_blank_name_are_rejected() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let id = insert_student(&conn, year, 1, "김철수");

    assert!(update_student_impl(&conn, school, id, 0, "김철수").is_err());
    assert!(update_student_impl(&conn, school, id, 1, "   ").is_err());
    // 실패했으니 원래 값이 그대로여야 한다.
    let active = get_students_impl(&conn, school, year, 3, 6).unwrap();
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
    assert_eq!(contacts[0].label, "어머니");
    assert_eq!(contacts[1].label, "학생 본인");
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
    assert_eq!(contacts[0].label, "어머니");
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

#[test]
fn classes_come_from_the_roster_not_a_separate_table() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");
    conn.execute(
        "INSERT INTO student (school_id, year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, ?2, 1, 2, 1, '최지훈', '2026-03-02')",
        rusqlite::params![school, year],
    )
    .unwrap();

    assert_eq!(
        get_classes_impl(&conn, school, year).unwrap(),
        vec![(1, 2), (3, 6)]
    );
}

#[test]
fn classes_of_one_school_do_not_show_in_another() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let other = another_school(&conn);
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");

    assert_eq!(get_classes_impl(&conn, school, year).unwrap(), vec![(3, 6)]);
    assert!(get_classes_impl(&conn, other, year).unwrap().is_empty());
}

#[test]
fn a_class_with_only_withdrawn_students_is_no_longer_listed() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let year = insert_year(&conn, 2026);
    let id = insert_student(&conn, year, 1, "김철수");
    withdraw_student_impl(&conn, school, id, "2026-06-30").unwrap();

    assert!(get_classes_impl(&conn, school, year).unwrap().is_empty());
}
