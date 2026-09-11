//! 교과 차시 — 내 수업에 있었는가 하나만 기록한다.
//!
//! 학생 이름은 전부 가짜다.
//!
//! 이 파일이 지키는 결정은 넷이다.
//!   · **행이 있다는 것이 곧 그 교시를 불렀다는 뜻이다.** 결석자 행만으로는
//!     "빠진 사람이 없는 날"과 "아직 안 부른 날"이 구별되지 않는다.
//!   · **같은 칸을 두 번 만들지 않는다.** [교시 추가]를 두 번 누른 것은 실수다.
//!   · **같은 학생을 다시 누르면 취소다.** 담임 쪽 무르기와 같은 규칙이다.
//!   · **담임 출결은 읽기 전용 참고로만 온다.** 내가 담임인 학급의 기록만 본다.

use crate::commands::attendance::stamp_span_impl;
use crate::commands::subject::*;
use crate::tests::*;
use crate::types::StampInput;
use rusqlite::Connection;

const DAY: &str = "2026-09-10";

/// 교과 강좌 하나와 명단 셋. 반이 섞이는 것이 교과의 정상 상태다.
struct Fixture {
    conn: Connection,
    school: i64,
    class: i64,
    students: Vec<i64>,
}

fn fixture() -> Fixture {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = subject(&conn, school, "인공지능기초 A반");
    let students = vec![
        insert_student_at(&conn, school, 3, 1, 4, "김하늘"),
        insert_student_at(&conn, school, 3, 6, 11, "박서연"),
        insert_student_at(&conn, school, 3, 6, 2, "이두리"),
    ];
    for id in &students {
        join_class(&conn, class, *id);
    }
    Fixture {
        conn,
        school,
        class,
        students,
    }
}

fn count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

// ── 차시 ──────────────────────────────────────────────────────

#[test]
fn a_session_row_is_the_record_that_the_slot_was_called() {
    let f = fixture();
    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();

    let sessions = get_subject_sessions_impl(&f.conn, f.class, DAY, DAY).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, id);
    assert_eq!(sessions[0].date, DAY);
    assert_eq!(sessions[0].slot, "1");
    assert_eq!(sessions[0].memo, "");
    // 빠진 사람이 없는 날과 아직 안 부른 날이 이렇게 구별된다.
    assert_eq!(sessions[0].absent_count, 0);
    assert_eq!(sessions[0].total, 3);
}

/// [교시 추가]를 두 번 누른 것은 실수지 새 칸을 만들라는 뜻이 아니다.
#[test]
fn making_the_same_slot_twice_returns_the_one_that_is_there() {
    let f = fixture();
    let first = create_subject_session_impl(&f.conn, f.class, DAY, "3").unwrap();
    let second = create_subject_session_impl(&f.conn, f.class, DAY, "3").unwrap();

    assert_eq!(first, second);
    assert_eq!(count(&f.conn, "subject_session"), 1);
}

/// **연강(3 · 4교시)도 두 칸이다.** 중간에 나간 학생이 실제로 있다.
#[test]
fn back_to_back_periods_are_two_separate_slots() {
    let f = fixture();
    let third = create_subject_session_impl(&f.conn, f.class, DAY, "3").unwrap();
    let fourth = create_subject_session_impl(&f.conn, f.class, DAY, "4").unwrap();
    assert_ne!(third, fourth);

    // 3교시에는 있었고 4교시에 나갔다.
    toggle_subject_absence_impl(&f.conn, fourth, f.students[0]).unwrap();

    let sessions = get_subject_sessions_impl(&f.conn, f.class, DAY, DAY).unwrap();
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].slot, "3");
    assert_eq!(sessions[0].absent_count, 0);
    assert_eq!(sessions[1].slot, "4");
    assert_eq!(sessions[1].absent_count, 1);
}

/// 차시 번호를 저장하지 않으므로 **날짜 · 교시 순**이 곧 `N차시`다.
#[test]
fn sessions_come_back_in_date_then_slot_order() {
    let f = fixture();
    create_subject_session_impl(&f.conn, f.class, "2026-09-14", "2").unwrap();
    create_subject_session_impl(&f.conn, f.class, DAY, "5").unwrap();
    create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();

    let sessions = get_subject_sessions_impl(&f.conn, f.class, "2026-09-01", "2026-09-30").unwrap();
    let seen: Vec<(&str, &str)> = sessions
        .iter()
        .map(|s| (s.date.as_str(), s.slot.as_str()))
        .collect();
    assert_eq!(
        seen,
        vec![(DAY, "1"), (DAY, "5"), ("2026-09-14", "2")]
    );
}

/// **조회 · 종례는 들어올 수 없다.** 그 둘은 담임이 하루의 양 끝에서 보는 것이지
/// 누가 가르치는 시간이 아니다.
#[test]
fn homeroom_and_closing_are_not_teachable_slots() {
    let f = fixture();
    for bad in ["조회", "종례"] {
        let err = create_subject_session_impl(&f.conn, f.class, DAY, bad).unwrap_err();
        assert!(err.contains("교과 수업이 아닙니다"), "{bad}: {err}");
    }
    assert_eq!(count(&f.conn, "subject_session"), 0);
}

/// **최대 교시는 학교가 들고 있는 값이다.** 앱 상수로 비교하지 않는다.
#[test]
fn a_slot_beyond_the_school_maximum_is_refused() {
    let f = fixture();
    let err = create_subject_session_impl(&f.conn, f.class, DAY, "8").unwrap_err();
    assert!(err.contains("1교시부터 7교시까지"), "{err}");

    // 학교가 최대 교시를 8로 바꾸면 그때부터 들어간다.
    crate::commands::school::update_school_impl(&f.conn, f.school, TEST_SCHOOL, 8, 7, true).unwrap();
    assert!(create_subject_session_impl(&f.conn, f.class, DAY, "8").is_ok());

    for bad in ["0", "-1", "1교시", ""] {
        assert!(
            create_subject_session_impl(&f.conn, f.class, DAY, bad).is_err(),
            "{bad}가 들어갔다"
        );
    }
}

/// 저장은 언제나 자리를 채운 ISO다. `2026-9-10`이 그대로 들어가면 날짜로 거르는
/// 모든 화면에서 그 행이 사라진다.
#[test]
fn a_date_is_stored_as_padded_iso() {
    let f = fixture();
    let id = create_subject_session_impl(&f.conn, f.class, "2026-9-10", "1").unwrap();
    let stored: String = f
        .conn
        .query_row(
            "SELECT date FROM subject_session WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stored, DAY);

    // 같은 날을 어느 표기로 물어도 같은 칸이다.
    assert_eq!(create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap(), id);
}

/// 담임 학급이 오면 거절한다. 트리거는 쓰기에만 걸리므로 읽기까지 막아야
/// 빈 화면을 보여주지 않는다.
#[test]
fn a_homeroom_class_is_refused_by_every_subject_command() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let home = homeroom(&conn, school);

    let err = create_subject_session_impl(&conn, home, DAY, "1").unwrap_err();
    assert!(err.contains("교과 강좌가 아닙니다"), "{err}");
    let err = get_subject_sessions_impl(&conn, home, DAY, DAY).unwrap_err();
    assert!(err.contains("교과 강좌가 아닙니다"), "{err}");
}

#[test]
fn an_end_date_before_the_start_is_reported() {
    let f = fixture();
    let err = get_subject_sessions_impl(&f.conn, f.class, "2026-09-30", "2026-09-01").unwrap_err();
    assert!(err.contains("끝 날짜가 시작 날짜보다 앞입니다"), "{err}");
}

/// 차시를 지우면 **그 칸의 결석도 함께 사라진다.** 다른 칸은 건드리지 않는다.
#[test]
fn deleting_a_session_takes_its_absences_with_it() {
    let f = fixture();
    let first = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();
    let third = create_subject_session_impl(&f.conn, f.class, DAY, "3").unwrap();
    toggle_subject_absence_impl(&f.conn, first, f.students[0]).unwrap();
    toggle_subject_absence_impl(&f.conn, first, f.students[1]).unwrap();
    toggle_subject_absence_impl(&f.conn, third, f.students[2]).unwrap();
    assert_eq!(count(&f.conn, "subject_absence"), 3);

    delete_subject_session_impl(&f.conn, first).unwrap();

    assert_eq!(count(&f.conn, "subject_session"), 1);
    assert_eq!(count(&f.conn, "subject_absence"), 1);
    let left = get_session_roll_impl(&f.conn, third).unwrap();
    assert!(left.iter().find(|r| r.student_id == f.students[2]).unwrap().absent);

    let err = delete_subject_session_impl(&f.conn, first).unwrap_err();
    assert!(err.contains("차시를 찾을 수 없습니다"), "{err}");
}

#[test]
fn a_session_memo_is_free_text_and_the_app_does_not_read_it() {
    let f = fixture();
    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();
    set_session_memo_impl(&f.conn, id, "수행평가 1차").unwrap();

    let sessions = get_subject_sessions_impl(&f.conn, f.class, DAY, DAY).unwrap();
    assert_eq!(sessions[0].memo, "수행평가 1차");

    assert!(set_session_memo_impl(&f.conn, 9999, "없는 차시")
        .unwrap_err()
        .contains("차시를 찾을 수 없습니다"));
}

// ── 차시 명단 ─────────────────────────────────────────────────

/// 그 날짜 명단 전원이 줄로 나온다. **학적 자리 순이다** — 교과는 반이 섞여
/// 번호만으로는 줄을 구분할 수 없다.
#[test]
fn the_roll_lists_everyone_on_the_list_in_seat_order() {
    let f = fixture();
    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();

    let roll = get_session_roll_impl(&f.conn, id).unwrap();
    let seen: Vec<(i64, i64, i64, &str)> = roll
        .iter()
        .map(|r| (r.grade, r.class_no, r.number, r.name.as_str()))
        .collect();
    assert_eq!(
        seen,
        vec![
            (3, 1, 4, "김하늘"),
            (3, 6, 2, "이두리"),
            (3, 6, 11, "박서연"),
        ]
    );
    assert!(roll.iter().all(|r| !r.absent), "출석은 저장하지 않는다");
    assert!(roll.iter().all(|r| r.memo.is_empty()));
    assert!(roll.iter().all(|r| r.homeroom_note.is_none()));
}

/// 누르면 결석, 다시 누르면 취소. **돌려주는 값이 입력한 뒤의 상태다.**
#[test]
fn stamping_the_same_student_twice_cancels_it() {
    let f = fixture();
    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();

    assert!(toggle_subject_absence_impl(&f.conn, id, f.students[0]).unwrap());
    assert_eq!(count(&f.conn, "subject_absence"), 1);

    assert!(!toggle_subject_absence_impl(&f.conn, id, f.students[0]).unwrap());
    assert_eq!(count(&f.conn, "subject_absence"), 0);

    // 세 번째는 다시 결석이다.
    assert!(toggle_subject_absence_impl(&f.conn, id, f.students[0]).unwrap());
    let roll = get_session_roll_impl(&f.conn, id).unwrap();
    assert_eq!(roll.iter().filter(|r| r.absent).count(), 1);
}

/// 무르기는 **그 차시 안에서의 일이다.** 다른 칸에 입력해 둔 같은 학생이 함께 지워지면
/// 교사가 못 본 사이에 기록이 사라진다.
#[test]
fn cancelling_in_one_slot_leaves_the_other_slot_alone() {
    let f = fixture();
    let first = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();
    let third = create_subject_session_impl(&f.conn, f.class, DAY, "3").unwrap();
    toggle_subject_absence_impl(&f.conn, first, f.students[0]).unwrap();
    toggle_subject_absence_impl(&f.conn, third, f.students[0]).unwrap();

    toggle_subject_absence_impl(&f.conn, first, f.students[0]).unwrap();

    assert!(!get_session_roll_impl(&f.conn, first).unwrap()[0].absent);
    assert!(get_session_roll_impl(&f.conn, third).unwrap()[0].absent);
}

#[test]
fn a_student_outside_the_list_is_refused_in_korean() {
    let f = fixture();
    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();
    let outsider = insert_student_at(&f.conn, f.school, 2, 3, 7, "남의 반 학생");

    let err = toggle_subject_absence_impl(&f.conn, id, outsider).unwrap_err();
    assert!(err.contains("이 강좌 명단에 없는 학생입니다"), "{err}");
    assert_eq!(count(&f.conn, "subject_absence"), 0);

    let err = toggle_subject_absence_impl(&f.conn, 9999, f.students[0]).unwrap_err();
    assert!(err.contains("차시를 찾을 수 없습니다"), "{err}");
}

/// 메모는 결석에 붙는 말이다. 빠지지 않은 학생에게 메모만 남으면 그 줄이 결석인지
/// 아닌지 화면이 말할 수 없다.
#[test]
fn an_absence_memo_needs_an_absence_to_hang_on() {
    let f = fixture();
    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();

    let err = set_subject_absence_memo_impl(&f.conn, id, f.students[0], "보건실").unwrap_err();
    assert!(err.contains("빠진 것으로 적힌 학생이 아닙니다"), "{err}");

    toggle_subject_absence_impl(&f.conn, id, f.students[0]).unwrap();
    set_subject_absence_memo_impl(&f.conn, id, f.students[0], "보건실").unwrap();

    let roll = get_session_roll_impl(&f.conn, id).unwrap();
    let row = roll.iter().find(|r| r.student_id == f.students[0]).unwrap();
    assert!(row.absent);
    assert_eq!(row.memo, "보건실");
}

/// 무르면 메모도 함께 사라진다. 결석이 없는데 사유만 남을 자리가 없다.
#[test]
fn cancelling_an_absence_drops_its_memo() {
    let f = fixture();
    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();
    toggle_subject_absence_impl(&f.conn, id, f.students[0]).unwrap();
    set_subject_absence_memo_impl(&f.conn, id, f.students[0], "보건실").unwrap();

    toggle_subject_absence_impl(&f.conn, id, f.students[0]).unwrap();
    toggle_subject_absence_impl(&f.conn, id, f.students[0]).unwrap();

    let roll = get_session_roll_impl(&f.conn, id).unwrap();
    let row = roll.iter().find(|r| r.student_id == f.students[0]).unwrap();
    assert!(row.absent);
    assert_eq!(row.memo, "", "무른 뒤의 메모가 남아 있다");
}

// ── 담임 출결 참고 (읽기 전용) ────────────────────────────────

/// 그 학급에 담임 출결 한 건을 남긴다.
fn stamp_homeroom(
    conn: &Connection,
    class: i64,
    student: i64,
    date: &str,
    reason: &str,
    r#type: &str,
    slots: &[&str],
) {
    let (reason_id, type_id) = axes(conn, reason, r#type);
    stamp_span_impl(
        conn,
        &StampInput {
            class_id: class,
            student_id: student,
            date: date.to_string(),
            reason_id,
            type_id,
            slots: slots.iter().map(|s| s.to_string()).collect(),
        },
    )
    .unwrap();
}

/// **의도한 참고다.** 교과 수업에서 빈 자리를 보았는데 아침에 담임으로 질병결석을
/// 입력해 두었으면 그것을 알려준다.
#[test]
fn a_homeroom_record_shows_up_as_a_read_only_note() {
    let f = fixture();
    let home = homeroom(&f.conn, f.school);
    // 같은 학생이 내 담임 반에도 내 교과 강좌에도 있다.
    join_class(&f.conn, home, f.students[2]);
    stamp_homeroom(&f.conn, home, f.students[2], DAY, "질병", "결석", &[]);

    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();
    let roll = get_session_roll_impl(&f.conn, id).unwrap();
    let row = roll.iter().find(|r| r.student_id == f.students[2]).unwrap();

    assert_eq!(row.homeroom_note.as_deref(), Some("질병결석 · 하루 종일"));
    // **읽기 전용이다.** 담임 기록을 보여줬다고 교과 결석이 기록되지는 않는다.
    assert!(!row.absent);
    assert_eq!(count(&f.conn, "subject_absence"), 0);

    // 다른 학생에게는 붙지 않는다.
    assert!(roll
        .iter()
        .filter(|r| r.student_id != f.students[2])
        .all(|r| r.homeroom_note.is_none()));
}

/// 두 축이 다 채워지지 않은 것이 정상 상태다. 그때도 문장이 나와야 한다.
#[test]
fn a_half_filled_homeroom_record_still_reads_as_a_sentence() {
    let f = fixture();
    let home = homeroom(&f.conn, f.school);
    join_class(&f.conn, home, f.students[0]);
    let (reason, _) = axes(&f.conn, "질병", "결석");
    stamp_span_impl(
        &f.conn,
        &StampInput {
            class_id: home,
            student_id: f.students[0],
            date: DAY.to_string(),
            reason_id: reason,
            type_id: None,
            slots: vec![],
        },
    )
    .unwrap();

    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();
    let roll = get_session_roll_impl(&f.conn, id).unwrap();
    let note = roll[0].homeroom_note.clone().unwrap();
    assert!(note.contains("질병"), "{note}");
    assert!(note.contains("종류 미정"), "{note}");
    assert!(note.contains("기간 미정"), "{note}");
}

/// 하루 두 구간이 정상이므로 한 문장에 둘 다 들어간다.
#[test]
fn two_homeroom_records_on_one_day_join_into_one_note() {
    let f = fixture();
    let home = homeroom(&f.conn, f.school);
    join_class(&f.conn, home, f.students[1]);
    stamp_homeroom(&f.conn, home, f.students[1], DAY, "질병", "지각", &["1"]);
    stamp_homeroom(&f.conn, home, f.students[1], DAY, "기타", "조퇴", &["6"]);

    let id = create_subject_session_impl(&f.conn, f.class, DAY, "3").unwrap();
    let roll = get_session_roll_impl(&f.conn, id).unwrap();
    let note = roll
        .iter()
        .find(|r| r.student_id == f.students[1])
        .unwrap()
        .homeroom_note
        .clone()
        .unwrap();

    assert!(note.contains("질병지각 · 조회부터 1교시까지"), "{note}");
    assert!(note.contains("기타조퇴 · 6교시부터 종례까지"), "{note}");
}

/// **그 날짜의 기록만 본다.** 어제 결석한 학생이 오늘 수업에서 결석으로 보이면 안 된다.
#[test]
fn a_note_only_covers_the_day_of_the_session() {
    let f = fixture();
    let home = homeroom(&f.conn, f.school);
    join_class(&f.conn, home, f.students[0]);
    stamp_homeroom(&f.conn, home, f.students[0], "2026-09-09", "질병", "결석", &[]);

    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();
    let roll = get_session_roll_impl(&f.conn, id).unwrap();
    assert!(roll.iter().all(|r| r.homeroom_note.is_none()));
}

/// **내 담임 학급의 기록만 본다.** 같은 학생이 다른 학교에서 받은 기록은 오지 않는다.
#[test]
fn a_note_never_crosses_into_another_school() {
    let f = fixture();
    let other_school = insert_school(&f.conn, year_id(&f.conn), "나다고등학교");
    let their_home = insert_class(
        &f.conn,
        other_school,
        "homeroom",
        "나다 3학년 6반",
        Some(3),
        Some(6),
    );
    let their_student = enroll(&f.conn, their_home, other_school, 11, "박서연");
    stamp_homeroom(&f.conn, their_home, their_student, DAY, "질병", "결석", &[]);

    // 내 강좌에도 같은 자리 · 같은 이름의 학생이 있다. 학적은 서로 다른 행이다.
    let id = create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();
    let roll = get_session_roll_impl(&f.conn, id).unwrap();
    assert!(
        roll.iter().all(|r| r.homeroom_note.is_none()),
        "다른 학교의 담임 기록이 새어 들어왔다"
    );
}

/// 교과 강좌에는 담임 출결이 붙을 수 없다(`trg_span_homeroom_only`).
/// 그래서 참고 문구도 담임 학급에서만 나온다 — 두 표는 완전히 격리된다.
#[test]
fn a_subject_class_can_never_hold_a_homeroom_record() {
    let f = fixture();
    let (reason, r#type) = axes(&f.conn, "질병", "결석");
    let refused = stamp_span_impl(
        &f.conn,
        &StampInput {
            class_id: f.class,
            student_id: f.students[0],
            date: DAY.to_string(),
            reason_id: reason,
            type_id: r#type,
            slots: vec![],
        },
    );
    assert!(refused.is_err(), "교과 강좌에 담임 출결이 붙었다");
    assert_eq!(count(&f.conn, "absence_span"), 0);
}

// ── 명단이 바뀌어도 지난 차시는 그대로다 ─────────────────────

/// `total`은 **그 날짜의** 명단 인원이다. 지금 인원이 아니다 —
/// 명렬표를 다시 가져와 인원이 달라져도 지난 차시의 분모가 움직이면 안 된다.
#[test]
fn the_denominator_is_the_list_as_it_was_on_that_day() {
    let f = fixture();
    create_subject_session_impl(&f.conn, f.class, DAY, "1").unwrap();

    // 9월 12일에 한 명이 명단에서 빠졌다.
    f.conn
        .execute(
            "UPDATE class_member SET left_on = '2026-09-12'
              WHERE class_id = ?1 AND student_id = ?2",
            rusqlite::params![f.class, f.students[0]],
        )
        .unwrap();
    create_subject_session_impl(&f.conn, f.class, "2026-09-14", "1").unwrap();

    let sessions = get_subject_sessions_impl(&f.conn, f.class, "2026-09-01", "2026-09-30").unwrap();
    assert_eq!(sessions[0].total, 3, "9월 10일에는 세 명이었다");
    assert_eq!(sessions[1].total, 2, "9월 14일에는 두 명이다");
}
