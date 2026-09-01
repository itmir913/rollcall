use crate::commands::attendance::*;
use crate::tests::*;

#[test]
fn absence_is_stored_as_a_fully_open_span() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let code = code_id(&conn, "질병결석");

    add_span_impl(&conn, sid, "2026-08-26", code, None, None, Some("몸살")).unwrap();

    let spans = get_spans_on_impl(&conn, "2026-08-26").unwrap();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].span_text, "* ~ *");
    assert_eq!(spans[0].code_label, "질병결석");
}

#[test]
fn early_leave_is_open_on_the_right_only() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 5, "박민수");
    let code = code_id(&conn, "질병조퇴");

    add_span_impl(&conn, sid, "2026-08-26", code, Some("5"), None, Some("복통")).unwrap();
    let spans = get_spans_on_impl(&conn, "2026-08-26").unwrap();
    assert_eq!(spans[0].span_text, "5 ~ *");
}

#[test]
fn two_spans_in_one_day_are_allowed() {
    // 3교시 무단결과 + 6교시 질병조퇴. 저장 구조가 이걸 못 담으면 설계가 막힌다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 1, "김철수");

    add_span_impl(
        &conn,
        sid,
        "2026-08-26",
        code_id(&conn, "미인정결과"),
        Some("3"),
        Some("3"),
        None,
    )
    .unwrap();
    add_span_impl(
        &conn,
        sid,
        "2026-08-26",
        code_id(&conn, "질병조퇴"),
        Some("6"),
        None,
        Some("복통"),
    )
    .unwrap();

    let spans = get_spans_on_impl(&conn, "2026-08-26").unwrap();
    assert_eq!(spans.len(), 2);

    // 사유는 하루에 한 줄뿐이다. 두 번째 구간이 첫 초안을 덮지 않는다.
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daily_reason WHERE student_id = ?1",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

#[test]
fn first_span_drafts_the_reason() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    add_span_impl(
        &conn,
        sid,
        "2026-08-26",
        code_id(&conn, "질병조퇴"),
        Some("5"),
        None,
        Some("복통"),
    )
    .unwrap();

    let reason: String = conn
        .query_row(
            "SELECT reason FROM daily_reason WHERE student_id = ?1 AND date = '2026-08-26'",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reason, "복통으로 5교시부터 질병조퇴");
}

#[test]
fn teacher_edited_reason_survives_a_second_span() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    add_span_impl(&conn, sid, "2026-08-26", code_id(&conn, "질병결석"), None, None, Some("몸살"))
        .unwrap();
    set_daily_reason_impl(&conn, sid, "2026-08-26", None, "교사가 직접 쓴 문구").unwrap();
    add_span_impl(
        &conn,
        sid,
        "2026-08-26",
        code_id(&conn, "질병조퇴"),
        Some("6"),
        None,
        None,
    )
    .unwrap();

    let reason: String = conn
        .query_row(
            "SELECT reason FROM daily_reason WHERE student_id = ?1 AND date = '2026-08-26'",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reason, "교사가 직접 쓴 문구");
}

#[test]
fn saving_a_span_creates_the_check_rows() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    add_span_impl(&conn, sid, "2026-08-26", code_id(&conn, "질병결석"), None, None, None).unwrap();

    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daily_check WHERE student_id = ?1 AND date = '2026-08-26'",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 2); // 시드의 활성 항목 수만큼
}

#[test]
fn deleting_the_last_span_clears_reason_and_checks() {
    // 빈 행은 출석이다. 체크가 남으면 사라진 결석의 서류가 미제출 목록에 계속 뜬다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    let span = add_span_impl(&conn, sid, "2026-08-26", code_id(&conn, "질병결석"), None, None, None)
        .unwrap();
    delete_span_impl(&conn, span).unwrap();

    let checks: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daily_check WHERE student_id = ?1",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    let reasons: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daily_reason WHERE student_id = ?1",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(checks, 0);
    assert_eq!(reasons, 0);
}

#[test]
fn deleting_one_of_two_spans_keeps_reason_and_checks() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    let first =
        add_span_impl(&conn, sid, "2026-08-26", code_id(&conn, "질병결석"), None, None, None)
            .unwrap();
    add_span_impl(
        &conn,
        sid,
        "2026-08-26",
        code_id(&conn, "질병조퇴"),
        Some("6"),
        None,
        None,
    )
    .unwrap();
    delete_span_impl(&conn, first).unwrap();

    let reasons: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daily_reason WHERE student_id = ?1",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reasons, 1);
}

#[test]
fn deleting_a_missing_span_is_not_an_error() {
    let conn = setup_test_db();
    assert!(delete_span_impl(&conn, 9999).is_ok());
}

#[test]
fn reversed_span_is_rejected_before_insert() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    let err = add_span_impl(
        &conn,
        sid,
        "2026-08-26",
        code_id(&conn, "질병결과"),
        Some("6"),
        Some("3"),
        None,
    )
    .unwrap_err();
    assert!(err.contains("시작 교시"));

    // 롤백 후 다음 쓰기가 정상 동작해야 한다.
    add_span_impl(&conn, sid, "2026-08-26", code_id(&conn, "질병결석"), None, None, None).unwrap();
}

// ── 어제 것 그대로 ────────────────────────────────────────────

#[test]
fn copy_previous_repeats_the_last_recorded_day() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    add_span_impl(&conn, sid, "2026-08-25", code_id(&conn, "질병결석"), None, None, Some("몸살"))
        .unwrap();
    let copied = copy_previous_impl(&conn, sid, "2026-08-26").unwrap();
    assert_eq!(copied, 1);

    let spans = get_spans_on_impl(&conn, "2026-08-26").unwrap();
    assert_eq!(spans[0].symptom.as_deref(), Some("몸살"));
}

#[test]
fn copy_previous_skips_gaps() {
    // 어제가 주말이면 그 전 기록일을 가져온다. 캘린더를 보지 않는다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    add_span_impl(&conn, sid, "2026-08-21", code_id(&conn, "질병결석"), None, None, Some("몸살"))
        .unwrap();
    copy_previous_impl(&conn, sid, "2026-08-24").unwrap();
    assert_eq!(get_spans_on_impl(&conn, "2026-08-24").unwrap().len(), 1);
}

#[test]
fn copy_previous_carries_the_edited_reason() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    add_span_impl(&conn, sid, "2026-08-25", code_id(&conn, "질병결석"), None, None, Some("몸살"))
        .unwrap();
    set_daily_reason_impl(&conn, sid, "2026-08-25", None, "장기 입원").unwrap();
    copy_previous_impl(&conn, sid, "2026-08-26").unwrap();

    let reason: String = conn
        .query_row(
            "SELECT reason FROM daily_reason WHERE student_id = ?1 AND date = '2026-08-26'",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reason, "장기 입원");
}

#[test]
fn copy_previous_does_not_inherit_the_group() {
    // 어제의 체험학습 묶음에 오늘이 끼면, 서류 한 장이 오늘까지 덮는 것으로 보인다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    bulk_apply_impl(
        &conn,
        &[sid],
        &["2026-08-25".to_string()],
        code_id(&conn, "출석인정결석"),
        None,
        None,
        Some("교외체험학습"),
    )
    .unwrap();
    copy_previous_impl(&conn, sid, "2026-08-26").unwrap();

    let spans = get_spans_on_impl(&conn, "2026-08-26").unwrap();
    assert_eq!(spans[0].group_id, None);
}

#[test]
fn copy_previous_refuses_when_nothing_to_copy() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    assert!(copy_previous_impl(&conn, sid, "2026-08-26").is_err());
}

#[test]
fn copy_previous_refuses_to_overwrite_today() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_span_impl(&conn, sid, "2026-08-25", code_id(&conn, "질병결석"), None, None, None).unwrap();
    add_span_impl(&conn, sid, "2026-08-26", code_id(&conn, "질병결석"), None, None, None).unwrap();
    assert!(copy_previous_impl(&conn, sid, "2026-08-26").is_err());
}

// ── 기간 일괄 입력 ────────────────────────────────────────────

#[test]
fn bulk_preview_excludes_weekend() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    let days = bulk_preview_impl(&conn, &[sid], "2026-08-28", "2026-09-01").unwrap();
    assert_eq!(
        days.iter().map(|d| d.date.as_str()).collect::<Vec<_>>(),
        vec!["2026-08-28", "2026-08-31", "2026-09-01"]
    );
    assert_eq!(days[0].label, "2026.08.28.(금)");
}

#[test]
fn bulk_preview_flags_days_that_already_have_records() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_span_impl(&conn, sid, "2026-08-31", code_id(&conn, "질병결석"), None, None, None).unwrap();

    let days = bulk_preview_impl(&conn, &[sid], "2026-08-28", "2026-09-01").unwrap();
    assert!(!days[0].has_existing);
    assert!(days[1].has_existing);
}

#[test]
fn bulk_preview_rejects_reversed_range() {
    let conn = setup_test_db();
    assert!(bulk_preview_impl(&conn, &[], "2026-09-01", "2026-08-28").is_err());
}

#[test]
fn bulk_apply_shares_one_group_id() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    let result = bulk_apply_impl(
        &conn,
        &[sid],
        &[
            "2026-08-28".to_string(),
            "2026-08-31".to_string(),
            "2026-09-01".to_string(),
        ],
        code_id(&conn, "출석인정결석"),
        None,
        None,
        Some("교외체험학습"),
    )
    .unwrap();
    assert_eq!(result.days, 3);

    let n: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT group_id) FROM absence_span WHERE student_id = ?1",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

#[test]
fn bulk_apply_uses_the_last_day_for_every_due_date() {
    // 서류 한 장이 기간 전체를 덮는다. 날짜마다 다른 마감일이 뜨면 목록이 시끄럽다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    bulk_apply_impl(
        &conn,
        &[sid],
        &["2026-08-28".to_string(), "2026-08-31".to_string()],
        code_id(&conn, "출석인정결석"),
        None,
        None,
        None,
    )
    .unwrap();

    let dues: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT due_date FROM daily_check
                 WHERE student_id = ?1 AND due_date IS NOT NULL",
            )
            .unwrap();
        stmt.query_map(rusqlite::params![sid], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<String>, _>>()
            .unwrap()
    };
    // 마지막 날(월) + 5영업일
    assert_eq!(dues, vec!["2026-09-07".to_string()]);
}

#[test]
fn bulk_apply_requires_students_and_dates() {
    let conn = setup_test_db();
    let code = code_id(&conn, "출석인정결석");
    assert!(bulk_apply_impl(&conn, &[], &["2026-08-28".into()], code, None, None, None).is_err());
    assert!(bulk_apply_impl(&conn, &[1], &[], code, None, None, None).is_err());
}

// ── 격자 ──────────────────────────────────────────────────────

#[test]
fn grid_has_a_row_per_enrolled_student_even_when_empty() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");
    let sid = insert_student(&conn, year, 2, "이영희");
    add_span_impl(&conn, sid, "2026-08-26", code_id(&conn, "질병결석"), None, None, None).unwrap();

    let grid = get_day_grid_impl(&conn, year, 3, 6, "2026-08-26").unwrap();
    assert_eq!(grid.rows.len(), 2);
    assert!(grid.rows[0].spans.is_empty()); // 빈 행 = 출석
    assert_eq!(grid.rows[1].spans.len(), 1);
    assert_eq!(grid.items.len(), 2);
}

#[test]
fn phrase_preview_matches_what_gets_saved() {
    let conn = setup_test_db();
    let out = render_phrase_impl(
        &conn,
        code_id(&conn, "질병조퇴"),
        Some("몸살"),
        Some("5"),
        None,
    )
    .unwrap();
    assert_eq!(out, "몸살로 5교시부터 질병조퇴");
}
