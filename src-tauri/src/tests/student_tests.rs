//! 명단과 연락처.
//!
//! 명렬표를 다시 여는 것은 **교체가 아니라 차분**이다. 사라진 번호를 지우면 그 학생의 출결
//! 기록이 함께 사라지므로, 명단에서 제외한 번호는 `class_member.left_on`을 채우는 것으로
//! 끝난다. **학생의 `enrolled_to`가 아니다** — 내 명단에서 제외된 것과 학교를 떠난 것은
//! 다른 일이다.
//!
//! 파일 열기는 두 가지 일을 한다. 학생을 만드는 일과 내 명단에 연결하는 일이고,
//! 이미 그 학적 자리에 있는 학생은 다시 만들지 않는다.

use crate::commands::class::member_rows_on;
use crate::commands::student::*;
use crate::tests::*;
use crate::types::{ContactItem, RosterDiffRow, RosterEntry, StudentItem};
use rusqlite::Connection;

fn entry(number: i64, name: &str) -> RosterEntry {
    entry_of(None, None, number, name)
}

/// 학년 · 반이 붙은 줄. 교과 명렬표는 줄마다 학적 자리를 들고 온다.
fn entry_at(grade: i64, class_no: i64, number: i64, name: &str) -> RosterEntry {
    entry_of(Some(grade), Some(class_no), number, name)
}

fn entry_of(grade: Option<i64>, class_no: Option<i64>, number: i64, name: &str) -> RosterEntry {
    RosterEntry {
        grade,
        class_no,
        number,
        name: name.to_string(),
        line: None,
    }
}

/// 지금 명단 한 줄. 차분은 DB를 모르는 순수 함수라 이 값을 그대로 받는다.
fn member(id: i64, number: i64, name: &str) -> StudentItem {
    member_at(id, 3, 6, number, name)
}

fn member_at(id: i64, grade: i64, class_no: i64, number: i64, name: &str) -> StudentItem {
    StudentItem {
        id,
        school_id: 1,
        grade,
        class_no,
        number,
        name: name.to_string(),
        enrolled_from: "2026-03-02".to_string(),
        enrolled_to: None,
    }
}

/// 미리보기 한 줄의 바탕. 칸이 늘어도 시험이 깨지지 않게 여기 한 곳에 모은다.
fn diff_row(number: i64, action: &str) -> RosterDiffRow {
    RosterDiffRow {
        key: 0,
        grade: None,
        class_no: None,
        line: None,
        number,
        incoming_name: None,
        current_name: None,
        student_id: None,
        action: action.to_string(),
        why: None,
    }
}

fn added_row(number: i64, name: &str) -> RosterDiffRow {
    RosterDiffRow {
        incoming_name: Some(name.to_string()),
        ..diff_row(number, "added")
    }
}

fn withdrawn_row(number: i64, name: &str, student_id: i64) -> RosterDiffRow {
    RosterDiffRow {
        current_name: Some(name.to_string()),
        student_id: Some(student_id),
        ..diff_row(number, "withdrawn")
    }
}

/// 그 학급의 명단 인원. 소속 줄은 남아도 지금 명단은 아니다.
fn member_count_of(conn: &Connection, class_id: i64) -> usize {
    get_students_impl(conn, class_id).unwrap().len()
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
    insert_school(conn, year_id(conn), "옆 학교")
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
    let rows = diff_roster(RosterKeying::ByNumber, &current, &[entry(1, "김철수")]);
    assert_eq!(rows[0].action, "added");
    assert_eq!(rows[0].student_id, None);
}

#[test]
fn same_number_and_name_is_unchanged() {
    let current = vec![member(10, 1, "김철수")];
    let rows = diff_roster(RosterKeying::ByNumber, &current, &[entry(1, "김철수")]);
    assert_eq!(rows[0].action, "unchanged");
}

#[test]
fn same_number_different_name_needs_teacher_decision() {
    // 개명인지, 번호를 물려받은 전입인지, 오타인지 프로그램은 판정할 수 없다.
    let current = vec![member(10, 1, "김철수")];
    let rows = diff_roster(RosterKeying::ByNumber, &current, &[entry(1, "김철호")]);
    assert_eq!(rows[0].action, "renamed");
    assert_eq!(rows[0].current_name.as_deref(), Some("김철수"));
    assert_eq!(rows[0].incoming_name.as_deref(), Some("김철호"));
}

#[test]
fn missing_number_is_withdrawn_not_deleted() {
    let current = vec![member(10, 1, "김철수"), member(11, 2, "이영희")];
    let rows = diff_roster(RosterKeying::ByNumber, &current, &[entry(1, "김철수")]);
    let withdrawn: Vec<_> = rows.iter().filter(|r| r.action == "withdrawn").collect();
    assert_eq!(withdrawn.len(), 1);
    assert_eq!(withdrawn[0].number, 2);
    assert_eq!(withdrawn[0].incoming_name, None);
}

#[test]
fn diff_is_sorted_by_number() {
    let current = vec![member(10, 3, "박민수")];
    let rows = diff_roster(
        RosterKeying::ByNumber,
        &current,
        &[entry(2, "이영희"), entry(1, "김철수")],
    );
    assert_eq!(
        rows.iter().map(|r| r.number).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    // 자리표는 목록 순서 그대로다. 미리보기와 적용이 이 값으로 연결된다.
    assert_eq!(
        rows.iter().map(|r| r.key).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}

// ── 교과의 열쇠는 자리다 ──────────────────────────────────────

#[test]
fn 교과에서_같은_번호_다른_반_두_줄이_서로_다른_학생으로_온다() {
    // 선택과목이라 1반~n반이 섞인다. 번호 하나로 식별하면 두 학생이 한 줄로 합쳐진다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let subject = insert_class(&conn, school, "subject", "지구과학Ⅰ", None, None);

    let incoming = vec![entry_at(3, 6, 4, "박하늘"), entry_at(3, 1, 4, "김하늘")];
    let rows = preview_roster_impl(&conn, subject, &incoming).unwrap();
    assert_eq!(rows.len(), 2, "두 줄이 한 줄로 합쳐졌다");
    assert!(rows.iter().all(|r| r.action == "added"), "{rows:?}");
    // 교과는 학적 자리 순으로 정렬된다.
    assert_eq!(rows[0].class_no, Some(1));
    assert_eq!(rows[1].class_no, Some(6));

    let result = apply_roster_impl(&conn, subject, "2026-03-02", &rows).unwrap();
    assert_eq!(result.added, 2);
    assert_eq!(result.created, 2);

    let active = get_students_impl(&conn, subject).unwrap();
    assert_eq!(
        active
            .iter()
            .map(|s| (s.grade, s.class_no, s.number, s.name.clone()))
            .collect::<Vec<_>>(),
        vec![
            (3, 1, 4, "김하늘".to_string()),
            (3, 6, 4, "박하늘".to_string()),
        ]
    );
}

#[test]
fn 교과_파일에_학년이_빈_줄이_있으면_그_줄만_blocked이고_전출이_꺼진다() {
    // 읽지 못한 줄은 짝 찾기에 참여하지 못한다. 그대로 두면 그 줄이 가리키던 학생이
    // 짝을 잃어 전출로 잡히고, 교사가 [저장]을 누르면 명단에서 제외된다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let subject = insert_class(&conn, school, "subject", "지구과학Ⅰ", None, None);
    let a = insert_student_at(&conn, school, 3, 1, 4, "김하늘");
    let b = insert_student_at(&conn, school, 3, 6, 11, "박하늘");
    join_class(&conn, subject, a);
    join_class(&conn, subject, b);

    let incoming = vec![
        entry_at(3, 1, 4, "김하늘"),
        RosterEntry {
            line: Some(7),
            ..entry(11, "박하늘")
        },
    ];
    let rows = preview_roster_impl(&conn, subject, &incoming).unwrap();

    let blocked: Vec<_> = rows.iter().filter(|r| r.action == "blocked").collect();
    assert_eq!(blocked.len(), 1);
    // 줄 번호는 문장이 아니라 `line` 칸으로 온다. 화면이 한 곳에서만 붙인다 —
    // 양쪽이 각각 붙이면 "7번째 줄 — 7번째 줄: …"이 된다.
    assert_eq!(blocked[0].line, Some(7));
    assert!(!blocked[0].why.as_deref().unwrap().contains("번째 줄"));
    assert!(
        !rows.iter().any(|r| r.action == "withdrawn"),
        "읽지 못한 줄이 있는데 전출이 켜졌다: {rows:?}"
    );
    // 짝을 잃은 학생은 그대로 두고 이유를 적는다.
    let kept = rows
        .iter()
        .find(|r| r.student_id == Some(b) && r.incoming_name.is_none())
        .unwrap();
    assert_eq!(kept.action, "unchanged");
    assert!(kept.why.is_some());

    let result = apply_roster_impl(&conn, subject, "2026-09-01", &rows).unwrap();
    assert_eq!(result.blocked, 1);
    assert_eq!(result.withdrawn, 0);
    assert_eq!(member_count_of(&conn, subject), 2, "명단이 그대로다");
}

#[test]
fn 파일_안에_같은_자리가_두_줄이면_blocked다() {
    // 지금까지는 검사가 없어 둘 다 added로 갔고, 둘째가 ux_student_seat에서 실패해
    // 트랜잭션이 통째로 롤백됐다. 서른 줄을 확인한 교사에게 남는 것은 에러 하나뿐이다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let subject = insert_class(&conn, school, "subject", "지구과학Ⅰ", None, None);

    let incoming = vec![entry_at(3, 1, 4, "김하늘"), entry_at(3, 1, 4, "박하늘")];
    let rows = preview_roster_impl(&conn, subject, &incoming).unwrap();
    assert!(rows.iter().all(|r| r.action == "blocked"), "{rows:?}");
    assert!(rows[0].why.as_deref().unwrap().contains("3학년 1반 4번"));

    let result = apply_roster_impl(&conn, subject, "2026-03-02", &rows).unwrap();
    assert_eq!(result.blocked, 2);
    assert_eq!(result.added, 0);
    assert_eq!(student_count(&conn), 0, "아무것도 쓰지 않는다");
}

#[test]
fn 자리를_남이_써서_막힌_줄도_전출을_보류한다() {
    // **검토가 잡은 치명 결함의 회귀 시험이다.**
    //
    // 차분이 막은 줄만 전출을 멈추면, 자리를 맞춰 본 뒤에 막힌 줄은 그대로 지나간다.
    // 그런데 그 줄도 짝을 빼앗는다 — 파일의 반 오타 한 글자로 `3학년 6반 4번`이
    // `3학년 1반 4번`이 되면 그 줄은 남의 자리라 막히고, 내 명단의 김하늘은 짝을 잃는다.
    // 교사가 "한 줄만 못 넣었구나" 하고 저장하면 유일한 학생이 명단에서 제외된다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let subject = insert_class(&conn, school, "subject", "지구과학Ⅰ", None, None);

    let mine = insert_student_at(&conn, school, 3, 6, 4, "김하늘");
    join_class(&conn, subject, mine);
    // 3학년 1반 4번은 내 강좌에 없는 학생이 쓰고 있다.
    insert_student_at(&conn, school, 3, 1, 4, "이수현");

    // 반을 6 대신 1로 잘못 적은 파일 한 줄.
    let rows = preview_roster_impl(&conn, subject, &[entry_at(3, 1, 4, "김하늘")]).unwrap();

    let blocked = rows.iter().find(|r| r.action == "blocked").expect("막힌 줄");
    assert_eq!(blocked.class_no, Some(1));

    let mine_row = rows.iter().find(|r| r.student_id == Some(mine)).expect("내 학생");
    assert_eq!(
        mine_row.action, "unchanged",
        "막힌 줄이 있으면 전출을 자동으로 표시하지 않는다"
    );
    assert!(mine_row.why.is_some(), "왜 표시하지 않았는지 함께 말한다");

    // 교사가 그대로 저장해도 명단이 비지 않는다.
    let result = apply_roster_impl(&conn, subject, "2026-09-01", &rows).unwrap();
    assert_eq!(result.withdrawn, 0);
    assert_eq!(result.blocked, 1);
    assert_eq!(member_count(&conn), 1, "오타 한 글자가 유일한 학생을 뺐다");
}

#[test]
fn 교과는_남의_반_학적을_마감하지_않는다() {
    // 교과 파일의 반 오타 하나가 남의 반 학생을 전출시키면 안 된다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let subject = insert_class(&conn, school, "subject", "지구과학Ⅰ", None, None);
    let theirs = insert_student_at(&conn, school, 3, 7, 12, "최지훈");

    let rows = preview_roster_impl(&conn, subject, &[entry_at(3, 7, 12, "김하늘")]).unwrap();
    assert_eq!(rows[0].action, "blocked");
    assert_eq!(rows[0].student_id, Some(theirs), "linked가 쓸 값이다");
    assert!(rows[0]
        .why
        .as_deref()
        .unwrap()
        .contains("그 학생이 맞습니다"));

    apply_roster_impl(&conn, subject, "2026-09-01", &rows).unwrap();
    assert_eq!(student_count(&conn), 1, "아무것도 만들지 않았다");

    // 화면이 그 줄을 added로 바꿔 보내도 거절한다. 조용히 지나가지 않는다.
    let forced = vec![RosterDiffRow {
        action: "added".into(),
        ..rows[0].clone()
    }];
    let err = apply_roster_impl(&conn, subject, "2026-09-01", &forced).unwrap_err();
    assert!(err.contains("마감할 수 없습니다"), "{err}");

    let enrolled_to: Option<String> = conn
        .query_row(
            "SELECT enrolled_to FROM student WHERE id = ?1",
            rusqlite::params![theirs],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(enrolled_to, None, "남의 반 학적이 마감됐다");
}

#[test]
fn linked는_학적을_건드리지_않고_명단에만_연결한다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let subject = insert_class(&conn, school, "subject", "지구과학Ⅰ", None, None);
    let theirs = insert_student_at(&conn, school, 3, 7, 12, "최지훈");

    let row = RosterDiffRow {
        grade: Some(3),
        class_no: Some(7),
        incoming_name: Some("김하늘".into()),
        student_id: Some(theirs),
        ..diff_row(12, "linked")
    };
    let result = apply_roster_impl(&conn, subject, "2026-09-01", &[row]).unwrap();
    assert_eq!(result.added, 1);
    assert_eq!(result.created, 0, "학적을 만들지 않는다");

    assert_eq!(student_count(&conn), 1);
    assert_eq!(member_count(&conn), 1);
    let name: String = conn
        .query_row(
            "SELECT name FROM student WHERE id = ?1",
            rusqlite::params![theirs],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(name, "최지훈", "이름까지 고치면 그것은 학적 수정이다");

    // 화면이 보낸 student_id를 그대로 믿지 않는다.
    let bogus = RosterDiffRow {
        student_id: Some(9999),
        ..diff_row(12, "linked")
    };
    let err = apply_roster_impl(&conn, subject, "2026-09-01", &[bogus]).unwrap_err();
    assert!(err.contains("연결할 학생을 찾을 수 없습니다"), "{err}");
}

#[test]
fn 같은_교과_파일을_두_번_넣어도_아무_일도_일어나지_않는다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let subject = insert_class(&conn, school, "subject", "지구과학Ⅰ", None, None);
    let incoming = vec![
        entry_at(3, 1, 4, "김하늘"),
        entry_at(3, 6, 4, "박하늘"),
        entry_at(2, 3, 20, "최지훈"),
    ];

    let rows = preview_roster_impl(&conn, subject, &incoming).unwrap();
    apply_roster_impl(&conn, subject, "2026-03-02", &rows).unwrap();
    assert_eq!(student_count(&conn), 3);
    assert_eq!(member_count(&conn), 3);

    let again = preview_roster_impl(&conn, subject, &incoming).unwrap();
    assert!(again.iter().all(|r| r.action == "unchanged"), "{again:?}");
    let result = apply_roster_impl(&conn, subject, "2026-09-01", &again).unwrap();
    assert_eq!(result.added, 0);
    assert_eq!(result.created, 0);
    assert_eq!(result.withdrawn, 0);
    assert_eq!(student_count(&conn), 3, "학생 행은 그대로다");
    assert_eq!(member_count(&conn), 3);
}

#[test]
fn 담임_학급_학생이_내_교과_강좌에도_있으면_학생_행은_하나_소속이_둘이다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let home = homeroom(&conn, school);
    let subject = insert_class(&conn, school, "subject", "지구과학Ⅰ", None, None);

    let rows = preview_roster_impl(&conn, home, &[entry(1, "김철수")]).unwrap();
    apply_roster_impl(&conn, home, "2026-03-02", &rows).unwrap();

    // 교과 명렬표에는 그 학생이 학적 자리와 함께 들어온다.
    let rows = preview_roster_impl(&conn, subject, &[entry_at(3, 6, 1, "김철수")]).unwrap();
    assert_eq!(rows[0].action, "added", "내 강좌 명단에는 아직 없다");
    assert_eq!(rows[0].student_id, None, "화면 토글이 studentId로 구별된다");
    assert!(rows[0].why.as_deref().unwrap().contains("이미 있는 학생"));

    let result = apply_roster_impl(&conn, subject, "2026-03-02", &rows).unwrap();
    assert_eq!(result.added, 1);
    assert_eq!(result.created, 0, "학적을 다시 만들지 않는다");

    assert_eq!(student_count(&conn), 1, "학생 행은 하나다");
    assert_eq!(member_count(&conn), 2, "소속이 둘이다");
}

#[test]
fn 담임_명렬표에_반이_다른_학생이_있어도_다시_열어도_전출시키지_않는다() {
    // 담임 열쇠를 자리로 확장하면 파일의 12번 줄이 이 학생과 짝을 잃어 매 다시 열기마다
    // 전출 + 중복 학적이 된다. 지난 출결이 옛 행에 남아 격자에서 통째로 사라진다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let mine = insert_student_at(&conn, school, 3, 6, 1, "학생1");
    let other = insert_student_at(&conn, school, 3, 7, 12, "학생2");
    join_class(&conn, class, mine);
    join_class(&conn, class, other);

    let incoming = vec![entry(1, "학생1"), entry(12, "학생2")];
    let rows = preview_roster_impl(&conn, class, &incoming).unwrap();
    assert!(rows.iter().all(|r| r.action == "unchanged"), "{rows:?}");

    let result = apply_roster_impl(&conn, class, "2026-09-01", &rows).unwrap();
    assert_eq!(result.withdrawn, 0);
    assert_eq!(result.created, 0);
    assert_eq!(student_count(&conn), 2, "중복 학적이 생겼다");
    assert_eq!(member_count_of(&conn, class), 2);

    let class_no: i64 = conn
        .query_row(
            "SELECT class_no FROM student WHERE id = ?1",
            rusqlite::params![other],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        class_no, 7,
        "학적은 그대로다. 담임 명단은 반으로 걸러지지 않는다"
    );
}

#[test]
fn 담임은_자리를_넘겨받으며_옛_학적을_마감한다() {
    // 한 자리에 두 명일 수 없다. 되돌릴 수 없는 쓰기이므로 몇 건인지 세어 돌려준다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let old = insert_student_at(&conn, school, 3, 6, 5, "이영희");

    let rows = preview_roster_impl(&conn, class, &[entry(5, "최지훈")]).unwrap();
    assert_eq!(rows[0].action, "added");
    assert!(
        rows[0].why.as_deref().unwrap().contains("마감하고"),
        "저장 전에 알린다: {:?}",
        rows[0].why
    );

    let result = apply_roster_impl(&conn, class, "2026-09-01", &rows).unwrap();
    assert_eq!(result.seat_closed, 1);
    assert_eq!(result.created, 1);

    let enrolled_to: Option<String> = conn
        .query_row(
            "SELECT enrolled_to FROM student WHERE id = ?1",
            rusqlite::params![old],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(enrolled_to.as_deref(), Some("2026-09-01"));
}

// ── 적용 ──────────────────────────────────────────────────────

#[test]
fn apply_adds_and_withdraws() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    enroll(&conn, class, school, 1, "김철수");
    enroll(&conn, class, school, 2, "이영희");

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
fn 다시_열어도_학생이_중복으로_생기지_않는다() {
    // 같은 학적 자리의 학생은 한 행이다. 파일 열기가 학생을 다시 만들면 같은 사람이
    // 둘이 되고, 지난 기록이 어느 쪽에 붙었는지 알 수 없게 된다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);

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

    // 학적은 이미 있는데 내 명단에는 아직 없는 학생. 파일 열기는 그 행을 찾아
    // 명단에만 연결한다 — 같은 자리에 학생을 하나 더 만들면 안 된다.
    let existing = insert_student_at(&conn, school, 3, 6, 3, "박민수");
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
    assert_eq!(joined, existing, "이미 있던 그 학생을 연결했다");
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
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    enroll(&conn, class, school, 1, "김철수");

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
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let id = enroll(&conn, class, school, 2, "이영희");

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
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let old = enroll(&conn, class, school, 5, "이영희");

    let rows = vec![withdrawn_row(5, "이영희", old), added_row(5, "최지훈")];

    apply_roster_impl(&conn, class, "2026-09-01", &rows).unwrap();
    let active = get_students_impl(&conn, class).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].name, "최지훈");
    assert_ne!(active[0].id, old);
}

#[test]
fn apply_rolls_back_on_error() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);

    let rows = vec![
        added_row(1, "김철수"),
        added_row(2, "  "), // 실패 유발
    ];
    assert!(apply_roster_impl(&conn, class, "2026-09-01", &rows).is_err());
    assert!(get_students_impl(&conn, class).unwrap().is_empty());

    // 트랜잭션이 열린 채 남지 않았는지 — 다음 쓰기가 정상 동작해야 한다.
    enroll(&conn, class, school, 1, "김철수");
    assert_eq!(get_students_impl(&conn, class).unwrap().len(), 1);
}

#[test]
fn apply_refuses_a_row_that_points_at_another_class() {
    // 학급을 잘못 선택한 채 적용하면 옆 반 명단이 조용히 정리된다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let other = insert_class(&conn, school, "homeroom", "3학년 7반", Some(3), Some(7));
    let id = enroll(&conn, class, school, 1, "김철수");

    let rows = vec![withdrawn_row(1, "김철수", id)];
    let err = apply_roster_impl(&conn, other, "2026-09-01", &rows).unwrap_err();
    assert!(err.contains("찾을 수 없습니다"), "{err}");

    // 원래 학급의 명단은 그대로다.
    assert_eq!(get_students_impl(&conn, class).unwrap().len(), 1);
}

// ── 학교가 명단을 구별한다 ────────────────────────────────────

#[test]
fn two_schools_can_hold_the_same_class_without_mixing() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let other_school = another_school(&conn);
    let theirs_class = insert_class(
        &conn,
        other_school,
        "homeroom",
        "옆 학교 3학년 6반",
        Some(3),
        Some(6),
    );

    enroll(&conn, class, school, 1, "김철수");
    apply_roster_impl(&conn, theirs_class, "2026-03-02", &[added_row(1, "최지훈")]).unwrap();

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
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let other = insert_class(&conn, school, "homeroom", "3학년 7반", Some(3), Some(7));
    enroll(&conn, class, school, 1, "김철수");

    let rows = preview_roster_impl(&conn, other, &[entry(1, "최지훈")]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].action, "added");
}

// ── 그날 명단 ─────────────────────────────────────────────────

#[test]
fn grid_shows_only_students_on_the_roster_that_date() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let id = enroll(&conn, class, school, 1, "김철수");
    enroll(&conn, class, school, 2, "이영희");
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
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let old = enroll(&conn, class, school, 5, "이영희");

    let rows = vec![withdrawn_row(5, "이영희", old), added_row(5, "최지훈")];
    apply_roster_impl(&conn, class, "2026-09-01", &rows).unwrap();

    let on_day = member_rows_on(&conn, class, "2026-09-01").unwrap();
    assert_eq!(on_day.len(), 1);
    assert_eq!(on_day[0].2, "최지훈");
}

#[test]
fn withdrawing_a_student_of_another_class_fails_instead_of_doing_nothing() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let other = insert_class(&conn, school, "homeroom", "3학년 7반", Some(3), Some(7));
    let id = enroll(&conn, class, school, 1, "김철수");

    let err = withdraw_student_impl(&conn, other, id, "2026-06-30").unwrap_err();
    assert!(err.contains("학생을 찾을 수 없습니다"), "{err}");
}

// ── 소속 날짜는 자리를 채운 ISO다 ─────────────────────────────
//
// `class_member`의 날짜 비교가 문자열 비교라, 깨진 날짜나 자리를 채우지 않은 날짜가
// 들어가면 `joined_on <= 날짜 < left_on`이 어긋나 그 학급이 조용히 빈 명단으로 보인다.

#[test]
fn 명렬표_적용은_날짜_형식을_확인한다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);

    let err = apply_roster_impl(&conn, class, "2026/03/02", &[added_row(1, "김하나")]).unwrap_err();
    assert!(err.contains("날짜 형식"), "{err}");
    assert_eq!(member_count(&conn), 0, "거절했으면 한 줄도 들어가지 않는다");
}

#[test]
fn 명렬표_적용은_자리를_채우지_않은_날짜도_ISO로_맞춘다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);

    apply_roster_impl(&conn, class, "2026-3-2", &[added_row(1, "김하나")]).unwrap();

    let joined: String = conn
        .query_row(
            "SELECT joined_on FROM class_member WHERE class_id = ?1",
            rusqlite::params![class],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(joined, "2026-03-02");
}

#[test]
fn 명단에서_빼는_날짜도_확인하고_ISO로_맞춘다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let id = enroll(&conn, class, school, 1, "김하나");

    let err = withdraw_student_impl(&conn, class, id, "2026.06.30").unwrap_err();
    assert!(err.contains("날짜 형식"), "{err}");

    withdraw_student_impl(&conn, class, id, "2026-6-30").unwrap();
    let left: Option<String> = conn
        .query_row(
            "SELECT left_on FROM class_member WHERE class_id = ?1 AND student_id = ?2",
            rusqlite::params![class, id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(left.as_deref(), Some("2026-06-30"));

    // 그날부터 그 명단이 아니다 — 날짜가 어긋났다면 이 줄이 남는다.
    assert!(member_rows_on(&conn, class, "2026-07-01").unwrap().is_empty());
}

// ── 개별 수정 ─────────────────────────────────────────────────

#[test]
fn duplicate_active_number_is_rejected_in_korean() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    enroll(&conn, class, school, 1, "김철수");
    let id = enroll(&conn, class, school, 2, "이영희");

    let err = update_student_impl(&conn, class, id, 1, "이영희").unwrap_err();
    assert!(err.contains("이미 같은 번호"), "영문 원문이 새어나왔다: {err}");
}

#[test]
fn updating_a_missing_student_says_so() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let err = update_student_impl(&conn, class, 9999, 1, "김철수").unwrap_err();
    assert!(err.contains("학생을 찾을 수 없습니다"), "{err}");
}

#[test]
fn a_number_below_one_and_a_blank_name_are_rejected() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let id = enroll(&conn, class, school, 1, "김철수");

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
    let school = school_id(&conn);
    let sid = insert_student(&conn, school, 2, "이영희");

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
    let school = school_id(&conn);
    let sid = insert_student(&conn, school, 2, "이영희");

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
    let school = school_id(&conn);
    let sid = insert_student(&conn, school, 2, "이영희");
    set_contacts_impl(&conn, sid, &[contact("어머니", "010-0000-0000")]).unwrap();
    set_contacts_impl(&conn, sid, &[]).unwrap();
    assert!(get_contacts_impl(&conn, sid).unwrap().is_empty());
}

#[test]
fn a_blank_contact_is_rejected_and_nothing_is_saved() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let sid = insert_student(&conn, school, 2, "이영희");

    assert!(set_contacts_impl(&conn, sid, &[contact("어머니", "  ")]).is_err());
    assert!(set_contacts_impl(&conn, sid, &[contact(" ", "010-0000-0000")]).is_err());
    assert!(get_contacts_impl(&conn, sid).unwrap().is_empty());
}

#[test]
fn a_rejected_save_leaves_the_previous_list_intact() {
    // 검증이 트랜잭션 앞에 있으므로 지우기부터 하고 실패하는 일이 없어야 한다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let sid = insert_student(&conn, school, 2, "이영희");
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
    let school = school_id(&conn);
    let sid = insert_student(&conn, school, 2, "이영희");
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
// 학급 목록은 이제 명단에서 추출하지 않는다. **담당 학급 · 강좌가 행이다**(`teaching_class`).
// 학생이 아직 하나도 없는 반도 선택해야 하고, 교과 강좌는 반이 섞여 학적으로는
// 이름조차 만들 수 없기 때문이다. 그쪽 시험은 `class_tests.rs`에 있다.

#[test]
fn 명단이_비어도_학급은_남는다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let id = enroll(&conn, class, school, 1, "김철수");
    withdraw_student_impl(&conn, class, id, "2026-06-30").unwrap();

    assert!(get_students_impl(&conn, class).unwrap().is_empty());
    let left: i64 = conn
        .query_row("SELECT COUNT(*) FROM teaching_class", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 1, "명렬표를 다시 열 자리가 남아야 한다");
}

// ── 명단에서 제외한 학생 ────────────────────────────────────────

#[test]
fn 명단에서_뺀_학생의_지난_기록은_보는_화면에_그대로_남는다() {
    // **숨겨야 할 곳은 새로 입력하는 자리 하나뿐이다.** 학년말에 5월 기록을 확인하는데
    // 전출 간 학생만 빠져 있으면 통계도 서류 목록도 틀린 값이 된다.
    //
    // 기록 질의에 `class_member`를 연결하면 이것이 조용히 깨진다 — 화면에는
    // "그런 기록이 없다"로 표시되어 아무도 알아채지 못한다.
    use crate::commands::attendance::{get_day_grid_impl, get_month_log_impl};
    use crate::commands::mark::get_doc_pending_impl;

    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let student = enroll(&conn, class, school, 7, "김하늘");
    let (reason, kind) = axes(&conn, "질병", "결석");

    conn.execute(
        "INSERT INTO absence_span (class_id, student_id, date, reason_id, type_id, doc_due)
         VALUES (?1, ?2, '2026-05-12', ?3, ?4, '2026-05-19')",
        rusqlite::params![class, student, reason, kind],
    )
    .unwrap();

    // 6월에 전출했다. 명단에서만 뺀다 — 학적을 마감하지 않는다.
    withdraw_student_impl(&conn, class, student, "2026-06-01").unwrap();

    // 보는 화면 — 전부 그대로 표시된다.
    let log = get_month_log_impl(&conn, class, 2026, 5).unwrap();
    let names: Vec<String> = log
        .iter()
        .flat_map(|day| day.spans.iter().map(|s| s.name.clone()))
        .collect();
    assert_eq!(names, vec!["김하늘".to_string()], "출결 기록에서 사라졌다");

    let pending = get_doc_pending_impl(&conn, class, None, None, false, "2026-09-11").unwrap();
    assert_eq!(pending.len(), 1, "서류 미제출자에서 사라졌다");

    // 새로 입력하는 자리 — 오늘 격자에서만 사라진다.
    let today = get_day_grid_impl(&conn, class, "2026-09-11").unwrap();
    assert!(
        today.rows.iter().all(|row| row.student_id != student),
        "전출한 학생이 오늘 격자에 남아 있다"
    );

    // **날짜를 함께 본다.** 5월로 옮기면 그때 있던 학생이 다시 표시된다 —
    // 지난 기록을 뒤늦게 고치는 일이 실제로 발생한다.
    let may = get_day_grid_impl(&conn, class, "2026-05-12").unwrap();
    assert!(
        may.rows.iter().any(|row| row.student_id == student),
        "5월 격자에서 그날 있던 학생을 찾을 수 없다"
    );
}

// ── 전입생 한 명 등록 ─────────────────────────────────────────

#[test]
fn 전입생을_한_명씩_등록한다() {
    // 전입생 한 명 때문에 명렬표 파일을 다시 만들게 하지 않는다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    enroll(&conn, class, school, 1, "김하늘");

    let id = add_student_impl(&conn, class, 3, 6, 30, "박서연", "2026-09-11").unwrap();
    let names: Vec<String> = get_students_impl(&conn, class)
        .unwrap()
        .iter()
        .map(|s| s.name.clone())
        .collect();
    assert_eq!(names, vec!["김하늘".to_string(), "박서연".to_string()]);
    assert!(id > 0);
}

#[test]
fn 이미_있는_학생은_다시_만들지_않고_명단에만_연결한다() {
    // 담임 반 학생이 내 교과 강좌에 중간에 추가되는 경우다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let home = homeroom(&conn, school);
    let course = subject(&conn, school, "인공지능기초A");
    enroll(&conn, home, school, 4, "김하늘");

    add_student_impl(&conn, course, 3, 6, 4, "김하늘", "2026-09-11").unwrap();
    assert_eq!(student_count(&conn), 1, "학생 행이 둘로 분리됐다");
    assert_eq!(get_students_impl(&conn, course).unwrap().len(), 1);
}

#[test]
fn 자리에_다른_이름이_있으면_등록을_거절한다() {
    // 한 명을 추가하는 동작이 남의 학적을 마감해서는 안 된다.
    let conn = setup_test_db();
    let school = school_id(&conn);
    let home = homeroom(&conn, school);
    let course = subject(&conn, school, "인공지능기초A");
    enroll(&conn, home, school, 4, "김하늘");

    let err = add_student_impl(&conn, course, 3, 6, 4, "박서연", "2026-09-11").unwrap_err();
    assert!(err.contains("김하늘"), "{err}");
    assert_eq!(student_count(&conn), 1, "학적을 하나 더 만들었다");
    // 거절된 등록이 명단에 흔적을 남기지 않는다.
    assert_eq!(get_students_impl(&conn, course).unwrap().len(), 0);
}

#[test]
fn 등록은_이름과_자리를_확인한다() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);

    assert!(add_student_impl(&conn, class, 3, 6, 1, "   ", "2026-09-11")
        .unwrap_err()
        .contains("이름"));
    assert!(add_student_impl(&conn, class, 0, 6, 1, "김하늘", "2026-09-11")
        .unwrap_err()
        .contains("1 이상"));
    assert!(add_student_impl(&conn, class, 3, 6, 1, "김하늘", "2026.09.11.")
        .unwrap_err()
        .contains("날짜"));
    assert_eq!(student_count(&conn), 0, "거절했는데 학생이 만들어졌다");
}
