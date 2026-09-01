use crate::commands::student::*;
use crate::tests::*;
use crate::types::RosterEntry;

fn entry(number: i64, name: &str) -> RosterEntry {
    RosterEntry {
        grade: None,
        class_no: None,
        number,
        name: name.to_string(),
    }
}

// ── 명렬표가 말하는 학급 ──────────────────────────────────────
//
// 파일 파싱은 프론트(`services/rosterFile.js`)가 한다. 여기로 오는 것은 이미
// (학년, 반, 번호, 이름)으로 정리된 목록이고, 남은 판단은 "어느 학급인가"뿐이다.

fn entry_of(grade: Option<i64>, class_no: Option<i64>, number: i64, name: &str) -> RosterEntry {
    RosterEntry {
        grade,
        class_no,
        number,
        name: name.to_string(),
    }
}

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
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");
    insert_student(&conn, year, 2, "이영희");

    let incoming = vec![entry(1, "김철수"), entry(3, "박민수")];
    let rows = preview_roster_impl(&conn, year, 3, 6, &incoming).unwrap();
    let result = apply_roster_impl(&conn, year, 3, 6, "2026-09-01", &rows).unwrap();

    assert_eq!(result.added, 1);
    assert_eq!(result.withdrawn, 1);

    let active = get_students_impl(&conn, year, 3, 6).unwrap();
    assert_eq!(
        active.iter().map(|s| s.number).collect::<Vec<_>>(),
        vec![1, 3]
    );
}

#[test]
fn withdrawn_student_keeps_its_row() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let id = insert_student(&conn, year, 2, "이영희");

    let rows = preview_roster_impl(&conn, year, 3, 6, &[]).unwrap();
    apply_roster_impl(&conn, year, 3, 6, "2026-09-01", &rows).unwrap();

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
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 5, "이영희");

    let rows = vec![
        crate::types::RosterDiffRow {
            number: 5,
            incoming_name: None,
            current_name: Some("이영희".into()),
            student_id: Some(
                conn.query_row(
                    "SELECT id FROM student WHERE number = 5",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap(),
            ),
            action: "withdrawn".into(),
        },
        crate::types::RosterDiffRow {
            number: 5,
            incoming_name: Some("최지훈".into()),
            current_name: None,
            student_id: None,
            action: "added".into(),
        },
    ];

    apply_roster_impl(&conn, year, 3, 6, "2026-09-01", &rows).unwrap();
    let active = get_students_impl(&conn, year, 3, 6).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].name, "최지훈");
}

#[test]
fn apply_rolls_back_on_error() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);

    let rows = vec![
        crate::types::RosterDiffRow {
            number: 1,
            incoming_name: Some("김철수".into()),
            current_name: None,
            student_id: None,
            action: "added".into(),
        },
        crate::types::RosterDiffRow {
            number: 2,
            incoming_name: Some("  ".into()), // 실패 유발
            current_name: None,
            student_id: None,
            action: "added".into(),
        },
    ];
    assert!(apply_roster_impl(&conn, year, 3, 6, "2026-09-01", &rows).is_err());
    assert!(get_students_impl(&conn, year, 3, 6).unwrap().is_empty());

    // 트랜잭션이 열린 채 남지 않았는지 — 다음 쓰기가 정상 동작해야 한다.
    insert_student(&conn, year, 1, "김철수");
    assert_eq!(get_students_impl(&conn, year, 3, 6).unwrap().len(), 1);
}

// ── 그날 재학생 ───────────────────────────────────────────────

#[test]
fn grid_shows_only_students_enrolled_on_that_date() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let id = insert_student(&conn, year, 1, "김철수");
    insert_student(&conn, year, 2, "이영희");
    withdraw_student_impl(&conn, id, "2026-06-30").unwrap();

    let before = get_students_on_impl(&conn, year, 3, 6, "2026-06-01").unwrap();
    assert_eq!(before.len(), 2);

    let after = get_students_on_impl(&conn, year, 3, 6, "2026-07-01").unwrap();
    assert_eq!(after.len(), 1);

    // 전출일 당일은 아직 재학이다.
    let on_day = get_students_on_impl(&conn, year, 3, 6, "2026-06-30").unwrap();
    assert_eq!(on_day.len(), 2);
}

#[test]
fn duplicate_active_number_is_rejected_in_korean() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");
    let id = insert_student(&conn, year, 2, "이영희");

    let err = update_student_impl(&conn, id, 1, "이영희").unwrap_err();
    assert!(err.contains("이미 같은 번호"), "영문 원문이 새어나왔다: {err}");
}

// ── 연락처 ────────────────────────────────────────────────────

#[test]
fn a_student_can_hold_many_contacts_in_order() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    set_contacts_impl(
        &conn,
        sid,
        &[
            crate::types::ContactItem {
                id: 0,
                label: "어머니".into(),
                value: "010-1111-1111".into(),
                note: None,
                sort_order: 0,
            },
            crate::types::ContactItem {
                id: 0,
                label: "학생".into(),
                value: "010-2222-2222".into(),
                note: Some("본인".into()),
                sort_order: 0,
            },
        ],
    )
    .unwrap();

    let contacts = get_contacts_impl(&conn, sid).unwrap();
    assert_eq!(contacts.len(), 2);
    assert_eq!(contacts[0].label, "어머니");
    assert_eq!(contacts[1].note.as_deref(), Some("본인"));
    // 순서는 저장 시점의 배열 순서를 따른다.
    assert_eq!(contacts[0].sort_order, 0);
    assert_eq!(contacts[1].sort_order, 1);
}

#[test]
fn saving_contacts_replaces_the_whole_list() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let one = |label: &str| crate::types::ContactItem {
        id: 0,
        label: label.into(),
        value: "010-0000-0000".into(),
        note: None,
        sort_order: 0,
    };

    set_contacts_impl(&conn, sid, &[one("어머니"), one("아버지")]).unwrap();
    set_contacts_impl(&conn, sid, &[one("어머니")]).unwrap();

    let contacts = get_contacts_impl(&conn, sid).unwrap();
    assert_eq!(contacts.len(), 1);
}

#[test]
fn a_blank_contact_is_rejected_and_nothing_is_saved() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let bad = crate::types::ContactItem {
        id: 0,
        label: "어머니".into(),
        value: "  ".into(),
        note: None,
        sort_order: 0,
    };
    assert!(set_contacts_impl(&conn, sid, &[bad]).is_err());
    assert!(get_contacts_impl(&conn, sid).unwrap().is_empty());
}

#[test]
fn contacts_go_away_with_the_student_row() {
    // 전출을 삭제로 처리하지 않는 이유가 여기에도 있다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    set_contacts_impl(
        &conn,
        sid,
        &[crate::types::ContactItem {
            id: 0,
            label: "어머니".into(),
            value: "010-0000-0000".into(),
            note: None,
            sort_order: 0,
        }],
    )
    .unwrap();

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
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");
    conn.execute(
        "INSERT INTO student (year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, 1, 2, 1, '최지훈', '2026-03-02')",
        rusqlite::params![year],
    )
    .unwrap();

    assert_eq!(get_classes_impl(&conn, year).unwrap(), vec![(1, 2), (3, 6)]);
}
