//! 학년도 — 이 앱 계층의 뿌리다.
//!
//! 여기서 붙드는 것은 셋이다.
//!   · **학년도가 바뀌면 학교 설정도 처음부터다.** 지난해 학교는 지난해에 남는다.
//!   · **`starts_on` · `ends_on`은 날짜 울타리가 아니다.** 학년도가 2026이어도
//!     2027년 3월 출결을 막지 않는다.
//!   · **없는 학년도를 고치면 알린다.** 조용히 성공하면 화면은 저장된 줄 알고 넘어간다.

use crate::commands::attendance::stamp_span_impl;
use crate::commands::class::get_teaching_classes_impl;
use crate::commands::school::{create_school_impl, get_schools_impl};
use crate::commands::year::*;
use crate::tests::*;
use crate::types::StampInput;

fn years(conn: &rusqlite::Connection) -> Vec<i64> {
    get_years_impl(conn).unwrap().into_iter().map(|y| y.year).collect()
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

    // 빈 칸은 "아직 안 정했다"는 뜻이라 그대로 통과한다.
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

/// **학년도 → 학교 → 맡은 것.** 학년도를 지우면 그 아래가 전부 함께 사라진다.
///
/// 이것이 `ON DELETE CASCADE`로 연결되어 있다는 사실이 곧 "학년도가 바뀌면 학교 설정도
/// 처음부터"라는 결정의 근거다 — 지난해 줄이 올해 목록에 남지 않는다.
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

    conn.execute(
        "DELETE FROM academic_year WHERE id = ?1",
        rusqlite::params![year_id(&conn)],
    )
    .unwrap();

    let count = |table: &str| -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    };
    assert_eq!(count("school"), 1, "다음 학년도의 학교만 남는다");
    assert_eq!(count("teaching_class"), 0);
    assert_eq!(count("student"), 0);
    assert_eq!(count("class_member"), 0);
    assert_eq!(count("absence_span"), 0);
    assert_eq!(count("span_tag"), 2, "다음 학년도 학교의 기본 태그는 남는다");

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
