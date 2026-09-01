use crate::commands::attendance::*;
use crate::tests::*;

// ── 두 축이 다 정해진 보통의 기록 ─────────────────────────────

#[test]
fn absence_is_stored_as_a_fully_open_span() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");

    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, None, None, Some("몸살")).unwrap();

    let spans = get_spans_on_impl(&conn, "2026-08-26").unwrap();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].span_text, "* ~ *");
    assert_eq!(spans[0].code_label.as_deref(), Some("질병결석"));
    assert!(spans[0].complete);
}

#[test]
fn early_leave_is_open_on_the_right_only() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 5, "박민수");
    let (r, t) = axes(&conn, "질병", "조퇴");

    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, Some("5"), None, Some("복통")).unwrap();
    assert_eq!(get_spans_on_impl(&conn, "2026-08-26").unwrap()[0].span_text, "5 ~ *");
}

#[test]
fn two_spans_in_one_day_are_allowed() {
    // 3교시 무단결과 + 6교시 질병조퇴. 저장 구조가 이걸 못 담으면 설계가 막힌다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 1, "김철수");

    let (r1, t1) = axes(&conn, "미인정", "결과");
    add_spans_impl(&conn, &[sid], "2026-08-26", r1, t1, Some("3"), Some("3"), None).unwrap();
    let (r2, t2) = axes(&conn, "질병", "조퇴");
    add_spans_impl(&conn, &[sid], "2026-08-26", r2, t2, Some("6"), None, Some("복통")).unwrap();

    assert_eq!(get_spans_on_impl(&conn, "2026-08-26").unwrap().len(), 2);

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

// ── 미완성 기록: 이 앱의 정상 상태 ────────────────────────────

#[test]
fn a_student_can_be_recorded_with_no_axes_at_all() {
    // 학교에 안 왔는데 연락이 닿지 않는다. 날짜와 학생만 남긴다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");

    add_spans_impl(&conn, &[sid], "2026-08-26", None, None, None, None, None).unwrap();

    let spans = get_spans_on_impl(&conn, "2026-08-26").unwrap();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].reason_id, None);
    assert_eq!(spans[0].type_id, None);
    assert_eq!(spans[0].code_label, None);
    assert!(!spans[0].complete);
}

#[test]
fn one_axis_alone_is_a_valid_intermediate_state() {
    // 구분만 눌렀고 종류는 아직. code_id 하나짜리 설계로는 담을 수 없는 상태다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let r = Some(reason_id(&conn, "질병"));

    add_spans_impl(&conn, &[sid], "2026-08-26", r, None, None, None, None).unwrap();

    let spans = get_spans_on_impl(&conn, "2026-08-26").unwrap();
    assert_eq!(spans[0].reason_label.as_deref(), Some("질병"));
    assert_eq!(spans[0].type_label, None);
    assert!(!spans[0].complete);
}

#[test]
fn an_incomplete_record_gets_no_reason_draft() {
    // 빈 사유 행이 있으면 나이스에 낼 것이 있는 것처럼 보인다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_spans_impl(&conn, &[sid], "2026-08-26", None, None, None, None, None).unwrap();

    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daily_reason WHERE student_id = ?1",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 0);
}

#[test]
fn an_incomplete_record_still_creates_its_check_rows() {
    // 서류는 구분이 정해지기 전에도 받아야 한다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_spans_impl(&conn, &[sid], "2026-08-26", None, None, None, None, None).unwrap();

    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM daily_check WHERE student_id = ?1",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 2);
}

#[test]
fn filling_in_the_axes_later_creates_the_reason_draft() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_spans_impl(&conn, &[sid], "2026-08-26", None, None, None, None, None).unwrap();
    let span = get_spans_on_impl(&conn, "2026-08-26").unwrap()[0].id;

    let (r, t) = axes(&conn, "질병", "조퇴");
    update_span_impl(&conn, span, r, t, Some("5"), None, Some("복통")).unwrap();

    let reason: String = conn
        .query_row(
            "SELECT reason FROM daily_reason WHERE student_id = ?1 AND date = '2026-08-26'",
            rusqlite::params![sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reason, "복통으로 5교시부터 질병조퇴");
    assert!(get_spans_on_impl(&conn, "2026-08-26").unwrap()[0].complete);
}

#[test]
fn incomplete_list_ignores_the_date_so_forgotten_records_surface() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김철수");
    let b = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");

    add_spans_impl(&conn, &[a], "2026-08-20", None, None, None, None, None).unwrap();
    add_spans_impl(&conn, &[b], "2026-08-26", r, t, None, None, None).unwrap();

    let rows = get_incomplete_impl(&conn, year, 3, 6).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].student_id, a);
}

// ── 도장 찍기 ─────────────────────────────────────────────────

#[test]
fn one_stamp_covers_many_students() {
    // 구분·종류를 고른 뒤 학생을 눌러 나가는 입력 방식이 이 경로를 쓴다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김철수");
    let b = insert_student(&conn, year, 2, "이영희");
    let c = insert_student(&conn, year, 3, "박민수");
    let (r, t) = axes(&conn, "출석인정", "결석");

    let n = add_spans_impl(&conn, &[a, b, c], "2026-08-26", r, t, None, None, Some("체험학습"))
        .unwrap();
    assert_eq!(n, 3);
    assert_eq!(get_spans_on_impl(&conn, "2026-08-26").unwrap().len(), 3);
}

#[test]
fn a_stamp_needs_at_least_one_student() {
    let conn = setup_test_db();
    assert!(add_spans_impl(&conn, &[], "2026-08-26", None, None, None, None, None).is_err());
}

#[test]
fn a_failing_stamp_rolls_back_every_student() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let a = insert_student(&conn, year, 1, "김철수");
    let (r, t) = axes(&conn, "질병", "결과");

    // 역순 구간은 저장 전에 걸린다.
    assert!(add_spans_impl(&conn, &[a], "2026-08-26", r, t, Some("6"), Some("3"), None).is_err());
    assert!(get_spans_on_impl(&conn, "2026-08-26").unwrap().is_empty());

    // 롤백 후 다음 쓰기가 정상 동작해야 한다.
    add_spans_impl(&conn, &[a], "2026-08-26", r, t, Some("3"), Some("6"), None).unwrap();
}

// ── 사유 ──────────────────────────────────────────────────────

#[test]
fn teacher_edited_reason_survives_a_second_span() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");

    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, None, None, Some("몸살")).unwrap();
    set_daily_reason_impl(&conn, sid, "2026-08-26", None, None, "교사가 직접 쓴 문구").unwrap();
    let (r2, t2) = axes(&conn, "질병", "조퇴");
    add_spans_impl(&conn, &[sid], "2026-08-26", r2, t2, Some("6"), None, None).unwrap();

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
fn phrase_preview_matches_what_gets_saved() {
    let conn = setup_test_db();
    let (r, t) = axes(&conn, "질병", "조퇴");
    let out = render_phrase_impl(&conn, r, t, Some("몸살"), Some("5"), None, None).unwrap();
    assert_eq!(out, "몸살로 5교시부터 질병조퇴");
}

#[test]
fn no_phrase_without_both_axes() {
    let conn = setup_test_db();
    let r = Some(reason_id(&conn, "질병"));
    assert_eq!(render_phrase_impl(&conn, r, None, Some("몸살"), None, None, None).unwrap(), "");
    assert_eq!(render_phrase_impl(&conn, None, None, None, None, None, None).unwrap(), "");
}

// ── 삭제 ──────────────────────────────────────────────────────

#[test]
fn deleting_the_last_span_clears_reason_and_checks() {
    // 빈 행은 출석이다. 체크가 남으면 사라진 결석의 서류가 미제출 목록에 계속 뜬다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");

    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, None, None, None).unwrap();
    let span = get_spans_on_impl(&conn, "2026-08-26").unwrap()[0].id;
    delete_span_impl(&conn, span).unwrap();

    for table in ["daily_check", "daily_reason"] {
        let n: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE student_id = ?1"),
                rusqlite::params![sid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0, "{table}이 남았다");
    }
}

#[test]
fn deleting_one_of_two_spans_keeps_reason_and_checks() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");

    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, None, None, None).unwrap();
    let first = get_spans_on_impl(&conn, "2026-08-26").unwrap()[0].id;
    let (r2, t2) = axes(&conn, "질병", "조퇴");
    add_spans_impl(&conn, &[sid], "2026-08-26", r2, t2, Some("6"), None, None).unwrap();
    delete_span_impl(&conn, first).unwrap();

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
fn deleting_a_missing_span_is_not_an_error() {
    let conn = setup_test_db();
    assert!(delete_span_impl(&conn, 9999).is_ok());
}

// ── 어제 것 그대로 ────────────────────────────────────────────

#[test]
fn copy_previous_repeats_the_last_recorded_day() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");

    add_spans_impl(&conn, &[sid], "2026-08-25", r, t, None, None, Some("몸살")).unwrap();
    assert_eq!(copy_previous_impl(&conn, sid, "2026-08-26").unwrap(), 1);
    assert_eq!(
        get_spans_on_impl(&conn, "2026-08-26").unwrap()[0].symptom.as_deref(),
        Some("몸살")
    );
}

#[test]
fn copy_previous_carries_an_incomplete_record_as_is() {
    // 어제도 아직 못 정했으면 오늘도 미정으로 복사된다. 프로그램이 판정하지 않는다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    add_spans_impl(&conn, &[sid], "2026-08-25", None, None, None, None, None).unwrap();

    copy_previous_impl(&conn, sid, "2026-08-26").unwrap();
    assert!(!get_spans_on_impl(&conn, "2026-08-26").unwrap()[0].complete);
}

#[test]
fn copy_previous_skips_gaps() {
    // 어제가 주말이면 그 전 기록일을 가져온다. 캘린더를 보지 않는다.
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");

    add_spans_impl(&conn, &[sid], "2026-08-21", r, t, None, None, Some("몸살")).unwrap();
    copy_previous_impl(&conn, sid, "2026-08-24").unwrap();
    assert_eq!(get_spans_on_impl(&conn, "2026-08-24").unwrap().len(), 1);
}

#[test]
fn copy_previous_carries_the_edited_reason() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");

    add_spans_impl(&conn, &[sid], "2026-08-25", r, t, None, None, Some("몸살")).unwrap();
    set_daily_reason_impl(&conn, sid, "2026-08-25", None, None, "장기 입원").unwrap();
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
    let (r, t) = axes(&conn, "출석인정", "결석");

    bulk_apply_impl(&conn, &[sid], &["2026-08-25".into()], r, t, None, None, Some("체험학습"))
        .unwrap();
    copy_previous_impl(&conn, sid, "2026-08-26").unwrap();

    assert_eq!(get_spans_on_impl(&conn, "2026-08-26").unwrap()[0].group_id, None);
}

#[test]
fn copy_previous_refuses_when_there_is_nothing_or_something_already() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let sid = insert_student(&conn, year, 2, "이영희");
    assert!(copy_previous_impl(&conn, sid, "2026-08-26").is_err());

    let (r, t) = axes(&conn, "질병", "결석");
    add_spans_impl(&conn, &[sid], "2026-08-25", r, t, None, None, None).unwrap();
    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, None, None, None).unwrap();
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
    let (r, t) = axes(&conn, "질병", "결석");
    add_spans_impl(&conn, &[sid], "2026-08-31", r, t, None, None, None).unwrap();

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
    let (r, t) = axes(&conn, "출석인정", "결석");

    let result = bulk_apply_impl(
        &conn,
        &[sid],
        &["2026-08-28".into(), "2026-08-31".into(), "2026-09-01".into()],
        r,
        t,
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
    let (r, t) = axes(&conn, "출석인정", "결석");

    bulk_apply_impl(
        &conn,
        &[sid],
        &["2026-08-28".into(), "2026-08-31".into()],
        r,
        t,
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
    assert!(bulk_apply_impl(&conn, &[], &["2026-08-28".into()], None, None, None, None, None)
        .is_err());
    assert!(bulk_apply_impl(&conn, &[1], &[], None, None, None, None, None).is_err());
}

// ── 격자 ──────────────────────────────────────────────────────

#[test]
fn grid_has_a_row_per_enrolled_student_even_when_empty() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 1, "김철수");
    let sid = insert_student(&conn, year, 2, "이영희");
    let (r, t) = axes(&conn, "질병", "결석");
    add_spans_impl(&conn, &[sid], "2026-08-26", r, t, None, None, None).unwrap();

    let grid = get_day_grid_impl(&conn, year, 3, 6, "2026-08-26").unwrap();
    assert_eq!(grid.rows.len(), 2);
    assert!(grid.rows[0].spans.is_empty()); // 빈 행 = 출석
    assert_eq!(grid.rows[1].spans.len(), 1);
    assert_eq!(grid.items.len(), 2);
}
