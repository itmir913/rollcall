use crate::commands::student::*;
use crate::tests::*;
use crate::types::RosterEntry;

fn entry(number: i64, name: &str) -> RosterEntry {
    RosterEntry {
        number,
        name: name.to_string(),
        guardian_phone: None,
    }
}

// ── 붙여넣기 파싱 ─────────────────────────────────────────────

#[test]
fn parses_tab_separated_paste() {
    let rows = parse_roster_text("1\t김철수\n2\t이영희\n");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].number, 1);
    assert_eq!(rows[1].name, "이영희");
}

#[test]
fn parses_comma_separated_paste() {
    // 교사가 어느 쪽을 붙여넣었는지 되묻지 않는다.
    let rows = parse_roster_text("1,김철수\n2,이영희");
    assert_eq!(rows.len(), 2);
}

#[test]
fn skips_header_and_blank_lines() {
    let rows = parse_roster_text("번호\t성명\n\n1\t김철수\n\n");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "김철수");
}

#[test]
fn ignores_leading_columns_before_number() {
    // 명렬표 양식이 무엇이든 상관없어야 한다. 학년/반 열이 앞에 붙어도 번호를 찾는다.
    let rows = parse_roster_text("3반\t7\t박민수");
    assert_eq!(rows[0].number, 7);
    assert_eq!(rows[0].name, "박민수");
}

#[test]
fn trims_whitespace_in_cells() {
    let rows = parse_roster_text(" 1 \t 김철수 ");
    assert_eq!(rows[0].name, "김철수");
}

#[test]
fn line_without_name_is_dropped() {
    let rows = parse_roster_text("1\t\n2\t이영희");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].number, 2);
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

    let err = update_student_impl(&conn, id, 1, "이영희", None).unwrap_err();
    assert!(err.contains("이미 같은 번호"), "영문 원문이 새어나왔다: {err}");
}
