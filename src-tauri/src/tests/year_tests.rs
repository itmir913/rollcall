//! 학년도 — 이 앱 계층의 뿌리다.
//!
//! 여기서 붙드는 것은 다섯이다.
//!   · **학년도가 바뀌면 학교 설정도 처음부터다.** 지난해 학교는 지난해에 남는다.
//!   · **`starts_on` · `ends_on`은 날짜 울타리가 아니다.** 학년도가 2026이어도
//!     2027년 3월 출결을 막지 않는다.
//!   · **없는 학년도를 고치거나 지우면 알린다.** 조용히 성공하면 화면은 저장된 줄 알고
//!     넘어간다.
//!   · **삭제는 그 아래를 통째로 지운다.** 되돌릴 수 없으므로 화면이 묻기 전에 무엇이
//!     사라지는지 보여줄 수 있어야 한다 — 그 수가 `AcademicYearItem`에 실려 온다.
//!   · **마지막 하나는 지우지 않는다.** 학년도가 0개가 되면 교사는 아무것도 할 수 없다.

use crate::commands::attendance::stamp_span_impl;
use crate::commands::class::get_teaching_classes_impl;
use crate::commands::config::{get_config_impl, set_config_impl};
use crate::commands::school::{create_school_impl, get_schools_impl, retire_school_impl};
use crate::commands::subject::create_subject_session_impl;
use crate::commands::year::*;
use crate::tests::*;
use crate::types::{AcademicYearItem, StampInput};

fn years(conn: &rusqlite::Connection) -> Vec<i64> {
    get_years_impl(conn).unwrap().into_iter().map(|y| y.year).collect()
}

/// 그 해의 목록 한 줄. 수를 확인하는 시험이 쓴다.
fn item(conn: &rusqlite::Connection, year: i64) -> AcademicYearItem {
    get_years_impl(conn)
        .unwrap()
        .into_iter()
        .find(|y| y.year == year)
        .unwrap_or_else(|| panic!("{year}학년도가 목록에 없다"))
}

fn count(conn: &rusqlite::Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

/// 그 해의 id. `year_id`는 가장 앞 행을 주므로 해를 지정할 때 이것을 쓴다.
fn id_of(conn: &rusqlite::Connection, year: i64) -> i64 {
    conn.query_row(
        "SELECT id FROM academic_year WHERE year = ?1",
        rusqlite::params![year],
        |r| r.get(0),
    )
    .unwrap()
}

/// 한 해 안에 학교 · 담당 · 학생 · 담임 출결 · 교과 차시를 하나씩 채운다.
/// 돌려주는 것은 그 학년도의 id다.
fn fill_year(conn: &rusqlite::Connection, year: i64) -> i64 {
    let year_id = insert_year(conn, year);
    let school = insert_school(conn, year_id, &format!("{year} 가나고등학교"));
    let class = homeroom(conn, school);
    let course = subject(conn, school, "지구과학Ⅰ");
    let student = enroll(conn, class, school, 1, "김하나");
    let (reason, r#type) = axes(conn, "질병", "결석");
    stamp_span_impl(
        conn,
        &StampInput {
            class_id: class,
            student_id: student,
            date: format!("{year}-09-01"),
            reason_id: reason,
            type_id: r#type,
            slots: vec![],
        },
    )
    .unwrap();
    create_subject_session_impl(conn, course, &format!("{year}-09-01"), "3").unwrap();
    year_id
}

#[test]
fn years_come_back_newest_first() {
    let conn = setup_seed_db();
    create_year_impl(&conn, 2026, Some("2026-03-01"), Some("2027-02-28")).unwrap();
    create_year_impl(&conn, 2027, None, None).unwrap();
    create_year_impl(&conn, 2025, None, None).unwrap();

    assert_eq!(years(&conn), vec![2027, 2026, 2025]);
}

#[test]
fn the_same_year_cannot_be_registered_twice() {
    let conn = setup_seed_db();
    create_year_impl(&conn, 2026, None, None).unwrap();
    let err = create_year_impl(&conn, 2026, None, None).unwrap_err();
    assert!(err.contains("이미 있는 학년도입니다"), "{err}");
    assert!(err.contains("2026"), "{err}");
}

#[test]
fn a_year_before_1900_is_refused() {
    let conn = setup_seed_db();
    let err = create_year_impl(&conn, 1899, None, None).unwrap_err();
    assert!(err.contains("학년도가 올바르지 않습니다"), "{err}");
    assert!(years(&conn).is_empty());
}

/// 형식이 깨진 날짜가 들어가면 통계가 그 학년도를 세지 못하고, 그때는 이미 기록이
/// 쌓인 뒤다. 다른 커맨드가 전부 확인하는데 여기만 빠져 있었다.
#[test]
fn the_start_and_end_dates_must_be_iso() {
    let conn = setup_seed_db();

    for bad in ["2026.03.01.", "3월 1일", "2026-3-1일"] {
        let err = create_year_impl(&conn, 2026, Some(bad), None).unwrap_err();
        assert!(err.contains("날짜 형식"), "{bad}: {err}");
        let err = create_year_impl(&conn, 2026, None, Some(bad)).unwrap_err();
        assert!(err.contains("날짜 형식"), "{bad}: {err}");
    }
    assert!(years(&conn).is_empty(), "거절된 학년도가 새어 들어갔다");

    // 빈 칸은 "아직 설정하지 않았다"는 뜻이라 그대로 통과한다.
    create_year_impl(&conn, 2026, None, Some("")).unwrap();
    let id = year_id(&conn);
    assert!(update_year_impl(&conn, id, Some("오늘"), None)
        .unwrap_err()
        .contains("날짜 형식"));
    update_year_impl(&conn, id, Some("2026-03-02"), Some("2027-02-28")).unwrap();

    let back = &get_years_impl(&conn).unwrap()[0];
    assert_eq!(back.starts_on.as_deref(), Some("2026-03-02"));
    assert_eq!(back.ends_on.as_deref(), Some("2027-02-28"));
}

/// 없는 id에 `Ok(())`를 돌려주면 화면은 저장된 줄 알고 넘어간다.
#[test]
fn updating_a_missing_year_is_reported_instead_of_passing_silently() {
    let conn = setup_seed_db();
    let err = update_year_impl(&conn, 9999, Some("2026-03-01"), None).unwrap_err();
    assert!(err.contains("학년도를 찾을 수 없습니다"), "{err}");
    assert!(err.contains("9999"), "{err}");
}

// ── 계층 ──────────────────────────────────────────────────────

/// **학년도 → 학교 → 담당 학급 · 강좌.** 학년도를 지우면 그 아래가 전부 함께 사라진다.
///
/// 이것이 `ON DELETE CASCADE`로 연결되어 있다는 사실이 곧 "학년도가 바뀌면 학교 설정도
/// 처음부터"라는 결정의 근거다 — 지난해 줄이 올해 목록에 남지 않는다.
///
/// **커맨드를 거쳐 지운다.** 스키마만 확인하면 커맨드가 엉뚱한 행을 지우거나 아무것도
/// 지우지 않아도 이 시험은 통과한다.
#[test]
fn deleting_a_year_takes_its_schools_classes_and_students_with_it() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let student = enroll(&conn, class, school, 1, "김하나");
    let (reason, r#type) = axes(&conn, "질병", "결석");
    stamp_span_impl(
        &conn,
        &StampInput {
            class_id: class,
            student_id: student,
            date: "2026-09-01".to_string(),
            reason_id: reason,
            type_id: r#type,
            slots: vec![],
        },
    )
    .unwrap();

    // 다음 학년도는 그대로 남아야 한다 — 지우는 것은 한 해뿐이다.
    let next_year = insert_year(&conn, TEST_YEAR + 1);
    let next_school = create_school_impl(&conn, next_year, "나다고등학교", 7, 7, true).unwrap();

    delete_year_impl(&conn, id_of(&conn, TEST_YEAR)).unwrap();

    assert_eq!(count(&conn, "school"), 1, "다음 학년도의 학교만 남는다");
    assert_eq!(count(&conn, "teaching_class"), 0);
    assert_eq!(count(&conn, "student"), 0);
    assert_eq!(count(&conn, "class_member"), 0);
    assert_eq!(count(&conn, "absence_span"), 0);
    assert_eq!(
        count(&conn, "span_tag"),
        2,
        "다음 학년도 학교의 기본 태그는 남는다"
    );

    assert_eq!(get_schools_impl(&conn, next_year).unwrap().len(), 1);
    assert!(get_teaching_classes_impl(&conn, next_school, None)
        .unwrap()
        .is_empty());
}

/// **학년도는 날짜 울타리가 아니다.** 2026학년도에 2027년 3월 출결을 넣을 수 있다.
///
/// 3월 초에는 지난 학년도 기록을 아직 정리하는 중이고, 학년도가 언제 끝나는지는
/// 학교마다 다르다. 막는 검증을 넣으면 그 며칠을 어디에도 적을 수 없다.
#[test]
fn a_date_outside_the_year_window_is_still_accepted() {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    let student = enroll(&conn, class, school, 1, "김하나");
    let (reason, r#type) = axes(&conn, "질병", "결석");

    // academic_year는 2026-03-01 ~ 2027-02-28로 만들어져 있다.
    for date in ["2027-03-05", "2025-12-24"] {
        let done = stamp_span_impl(
            &conn,
            &StampInput {
                class_id: class,
                student_id: student,
                date: date.to_string(),
                reason_id: reason,
                type_id: r#type,
                slots: vec![],
            },
        )
        .unwrap();
        assert_eq!(done.action, "added", "{date}가 막혔다");
    }
}

// ── 삭제 ──────────────────────────────────────────────────────
//
// Welcome에서 만든 학년도는 초안 없이 곧장 DB에 들어간다. 오타로 만든 2027학년도를
// 지우는 길이 없으면 목록에 영원히 남는다. 되돌릴 수 없는 쓰기라 규칙이 셋이다 —
// 없는 것은 알리고, 마지막 하나는 거절하고, 지운 뒤에는 가리키던 자리를 비운다.

/// 아무것도 없는 학년도는 그대로 사라지고 다른 학년도는 건드리지 않는다.
#[test]
fn an_empty_year_is_deleted_and_the_others_stay() {
    let conn = setup_seed_db();
    create_year_impl(&conn, 2026, None, None).unwrap();
    create_year_impl(&conn, 2027, None, None).unwrap();
    create_year_impl(&conn, 2028, None, None).unwrap();

    delete_year_impl(&conn, id_of(&conn, 2027)).unwrap();

    assert_eq!(years(&conn), vec![2028, 2026]);
}

/// **마지막 하나는 지우지 않는다.** 학년도가 0개가 되면 학교도 담당 학급 · 강좌도
/// 만들 수 없어 교사가 아무것도 할 수 없는 화면에 갇힌다.
///
/// 거절 문구가 이유와 다음에 할 일을 말해야 한다 — "지울 수 없습니다"만 보면 교사는
/// 앱이 고장 난 줄 안다.
#[test]
fn the_last_year_is_refused_with_the_reason() {
    let conn = setup_test_db();

    let err = delete_year_impl(&conn, id_of(&conn, TEST_YEAR)).unwrap_err();
    assert!(err.contains("마지막 학년도는 지울 수 없습니다"), "{err}");
    assert!(
        err.contains("담당 학급 · 강좌"),
        "거절 문구가 이유를 말하지 않는다: {err}"
    );
    assert!(
        err.contains("새 학년도를 먼저 만든"),
        "다음에 할 일을 말하지 않는다: {err}"
    );

    assert_eq!(years(&conn), vec![TEST_YEAR]);
    assert_eq!(count(&conn, "school"), 1, "거절된 삭제가 학교를 건드렸다");

    // 하나를 더 만들면 그때는 지워진다 — 막은 것은 그 학년도가 아니라 "마지막"이다.
    insert_year(&conn, TEST_YEAR + 1);
    delete_year_impl(&conn, id_of(&conn, TEST_YEAR)).unwrap();
    assert_eq!(years(&conn), vec![TEST_YEAR + 1]);
}

/// 없는 학년도를 지우면 알린다. 조용히 성공하면 화면은 지워진 줄 알고 목록에서 뺀다.
#[test]
fn deleting_a_missing_year_is_reported_instead_of_passing_silently() {
    let conn = setup_test_db();
    insert_year(&conn, TEST_YEAR + 1);

    let err = delete_year_impl(&conn, 9999).unwrap_err();
    assert!(err.contains("학년도를 찾을 수 없습니다"), "{err}");
    assert!(err.contains("9999"), "{err}");
    assert_eq!(years(&conn).len(), 2, "없는 번호가 남의 줄을 지웠다");
}

/// 남은 것이 하나뿐일 때 없는 번호를 넘기면 **"찾을 수 없습니다"**여야 한다.
///
/// "마지막 학년도"라고 답하면 교사는 지우려던 줄이 그대로인 이유를 잘못 읽는다.
#[test]
fn a_missing_id_is_reported_even_when_only_one_year_is_left() {
    let conn = setup_test_db();
    let err = delete_year_impl(&conn, 9999).unwrap_err();
    assert!(err.contains("학년도를 찾을 수 없습니다"), "{err}");
}

/// **무엇이 함께 사라지는가를 목록이 들고 온다.** 삭제가 되돌릴 수 없으므로 화면은
/// 묻기 전에 이 수를 보여줘야 한다.
///
/// 담임 출결과 교과 차시를 한 수로 합치지 않는다 — 두 기록은 표부터 다르다.
#[test]
fn the_year_row_carries_what_would_be_deleted_with_it() {
    let conn = setup_seed_db();
    let filled = fill_year(&conn, TEST_YEAR);
    // 순회 교사. 한 학년도에 학교가 둘일 수 있다.
    let other = insert_school(&conn, filled, "나다고등학교");
    insert_student(&conn, other, 1, "이둘");
    insert_year(&conn, TEST_YEAR + 1);

    let row = item(&conn, TEST_YEAR);
    assert_eq!(row.school_count, 2);
    assert_eq!(row.class_count, 2, "담임 학급 + 교과 강좌");
    assert_eq!(row.student_count, 2);
    assert_eq!(row.span_count, 1);
    assert_eq!(row.session_count, 1);

    // 빈 학년도는 전부 0이다. 수가 다른 학년도에서 새어 오지 않는다.
    let empty = item(&conn, TEST_YEAR + 1);
    assert_eq!(
        (
            empty.school_count,
            empty.class_count,
            empty.student_count,
            empty.span_count,
            empty.session_count
        ),
        (0, 0, 0, 0, 0)
    );
}

/// **마감한 학교도 센다.** 지울 때는 그것도 똑같이 사라지기 때문이다.
///
/// `get_schools`가 내린 학교를 빼는 것과 기준이 다른 것이 맞다 — 저쪽은 선택할 대상이고
/// 이쪽은 사라질 대상이다. 여기서 빼면 "학교 0개"라고 알린 뒤 학교가 지워진다.
#[test]
fn a_retired_school_still_counts_because_it_is_deleted_too() {
    let conn = setup_seed_db();
    let year = fill_year(&conn, TEST_YEAR);
    let school: i64 = conn
        .query_row("SELECT id FROM school WHERE year_id = ?1", [year], |r| {
            r.get(0)
        })
        .unwrap();

    retire_school_impl(&conn, school).unwrap();

    assert!(
        get_schools_impl(&conn, year).unwrap().is_empty(),
        "내린 학교가 목록에 남았다"
    );
    let row = item(&conn, TEST_YEAR);
    assert_eq!(row.school_count, 1, "내린 학교를 세지 않았다");
    assert_eq!(row.class_count, 2, "그 학교의 담당도 함께 사라진다");
    assert_eq!(row.span_count, 1);
}

/// **지금 보고 있는 학년도를 지우면 범위 열쇠를 비운다.**
///
/// 대신 볼 학년도를 Rust가 선택하지 않는다 — 열쇠가 없으면 화면이 목록의 첫 학년도로
/// 되돌린다. 학교 · 학급 열쇠도 함께 비운다. 그 행들은 CASCADE로 이미 사라졌는데
/// 학년도만 비우면 다음 실행에서 없는 학교 · 학급 번호가 복원된다.
#[test]
fn deleting_the_year_in_view_clears_the_scope_keys() {
    let conn = setup_seed_db();
    let gone = fill_year(&conn, TEST_YEAR);
    insert_year(&conn, TEST_YEAR + 1);

    set_config_impl(&conn, "yearId", &gone.to_string()).unwrap();
    set_config_impl(&conn, "schoolId", "7").unwrap();
    set_config_impl(&conn, "homeroomClassId", "11").unwrap();
    set_config_impl(&conn, "subjectClassId", "12").unwrap();
    set_config_impl(&conn, "mode", "subject").unwrap();

    delete_year_impl(&conn, gone).unwrap();

    for key in ["yearId", "schoolId", "homeroomClassId", "subjectClassId"] {
        assert_eq!(
            get_config_impl(&conn, key).unwrap(),
            None,
            "{key}가 없는 행을 가리킨 채 남았다"
        );
    }
    // 모드는 행을 가리키는 값이 아니라 화면 취향이다. 지울 이유가 없다.
    assert_eq!(
        get_config_impl(&conn, "mode").unwrap().as_deref(),
        Some("subject")
    );
}

/// **다른 학년도를 지울 때는 열쇠에 손대지 않는다.** 그 열쇠들은 남아 있는 학년도의
/// 자리를 가리키고 있고, 지우면 교사가 보던 화면이 이유 없이 처음으로 돌아간다.
#[test]
fn deleting_another_year_leaves_the_scope_keys_alone() {
    let conn = setup_seed_db();
    let kept = fill_year(&conn, TEST_YEAR);
    let gone = fill_year(&conn, TEST_YEAR + 1);

    set_config_impl(&conn, "yearId", &kept.to_string()).unwrap();
    set_config_impl(&conn, "schoolId", "7").unwrap();

    delete_year_impl(&conn, gone).unwrap();

    assert_eq!(
        get_config_impl(&conn, "yearId").unwrap().as_deref(),
        Some(kept.to_string().as_str())
    );
    assert_eq!(
        get_config_impl(&conn, "schoolId").unwrap().as_deref(),
        Some("7")
    );
}

/// 거절된 삭제는 아무것도 바꾸지 않는다. 열쇠를 비우는 것은 실제로 지운 뒤의 일이다.
#[test]
fn a_refused_delete_changes_nothing() {
    let conn = setup_seed_db();
    let only = fill_year(&conn, TEST_YEAR);
    set_config_impl(&conn, "yearId", &only.to_string()).unwrap();

    delete_year_impl(&conn, only).unwrap_err();

    assert_eq!(
        get_config_impl(&conn, "yearId").unwrap().as_deref(),
        Some(only.to_string().as_str())
    );
    let row = item(&conn, TEST_YEAR);
    assert_eq!(row.school_count, 1);
    assert_eq!(row.span_count, 1);
    assert_eq!(row.session_count, 1);
}
