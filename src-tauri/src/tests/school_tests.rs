use crate::commands::school::*;
use crate::tests::*;
use crate::types::{OffDayItem, QuotaRuleItem};

const TODAY: &str = "2026-09-11";

fn off(date: &str, label: Option<&str>) -> OffDayItem {
    OffDayItem {
        id: 0,
        date: date.to_string(),
        label: label.map(|s| s.to_string()),
    }
}

fn new_rule(name: &str, tag: Option<i64>, period: &str, limit_n: i64, unit: &str) -> QuotaRuleItem {
    QuotaRuleItem {
        id: 0,
        name: name.to_string(),
        tag_id: tag,
        tag_name: None,
        reason_id: None,
        type_id: None,
        period: period.to_string(),
        limit_n,
        unit: unit.to_string(),
        sort_order: 0,
        valid_from: String::new(),
        valid_to: None,
    }
}

/// 마감된 행이 남아 있는지 직접 확인한다. 커맨드는 유효한 것만 돌려주므로
/// 목록만 봐서는 "지웠다"와 "마감했다"를 구별할 수 없다.
fn tag_valid_to(conn: &rusqlite::Connection, tag_id: i64) -> Option<String> {
    conn.query_row(
        "SELECT valid_to FROM span_tag WHERE id = ?1",
        rusqlite::params![tag_id],
        |r| r.get(0),
    )
    .unwrap()
}

fn count(conn: &rusqlite::Connection, table: &str) -> i64 {
    count_where(conn, &format!("SELECT COUNT(*) FROM {table}"))
}

fn count_where(conn: &rusqlite::Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

// ── 학교 ──────────────────────────────────────────────────────

#[test]
fn a_school_carries_its_own_settings() {
    // 최대 교시와 제출 기한은 앱 상수도 전역 설정도 아니라 학교가 들고 있는 값이다.
    let conn = setup_test_db();
    let schools = get_schools_impl(&conn, year_id(&conn)).unwrap();
    assert_eq!(schools.len(), 1);
    assert_eq!(schools[0].name, TEST_SCHOOL);
    assert_eq!(schools[0].max_slot, 7);
    assert_eq!(schools[0].due_days, 7);
    assert!(schools[0].due_skip_offdays);
    assert!(schools[0].active);
    assert_eq!(schools[0].year_id, year_id(&conn));
}

/// **학교는 학년도 안에 있다.** 전역으로 두면 해가 바뀌어도 지난해 학교가 목록에 남고,
/// 그 학교의 최대 교시를 고치면 지난해 화면까지 소급해 바뀐다.
#[test]
fn a_school_belongs_to_one_academic_year() {
    let conn = setup_test_db();
    let this_year = year_id(&conn);
    let next_year = insert_year(&conn, TEST_YEAR + 1);

    // 순회 교사 — 2027학년도에는 두 학교다.
    create_school_impl(&conn, next_year, "나다고등학교", 6, 3, false).unwrap();
    create_school_impl(&conn, next_year, "라마고등학교", 7, 7, true).unwrap();

    let names = |y| {
        get_schools_impl(&conn, y)
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(names(this_year), vec![TEST_SCHOOL]);
    assert_eq!(names(next_year), vec!["나다고등학교", "라마고등학교"]);

    // 같은 이름이라도 학년도가 다르면 다른 학교다.
    assert!(create_school_impl(&conn, next_year, TEST_SCHOOL, 7, 7, true).is_ok());
}

/// 같은 학년도에 같은 이름의 학교는 하나다. **내린 학교는 세지 않는다** —
/// 잘못 만든 학교를 마감하고 같은 이름으로 다시 만드는 일이 있다.
#[test]
fn one_year_holds_one_school_of_each_name() {
    let conn = setup_test_db();
    let y = year_id(&conn);

    let err = create_school_impl(&conn, y, TEST_SCHOOL, 7, 7, true).unwrap_err();
    assert!(err.contains("이미 있는 학교입니다"), "{err}");

    retire_school_impl(&conn, school_id(&conn)).unwrap();
    assert!(create_school_impl(&conn, y, TEST_SCHOOL, 7, 7, true).is_ok());
}

#[test]
fn an_unknown_school_is_reported_in_korean() {
    let conn = setup_test_db();
    let err = get_school_impl(&conn, 9999).unwrap_err();
    assert!(err.contains("학교를 찾을 수 없습니다"), "{err}");
    assert!(err.contains("9999"), "{err}");
}

#[test]
fn max_slot_outside_one_to_nine_is_refused() {
    let conn = setup_test_db();
    let id = school_id(&conn);

    for bad in [0, -1, 10] {
        let err = update_school_impl(&conn, id, TEST_SCHOOL, bad, 7, true).unwrap_err();
        assert!(err.contains("최대 교시"), "{err}");
    }

    update_school_impl(&conn, id, TEST_SCHOOL, 9, 7, true).unwrap();
    assert_eq!(get_school_impl(&conn, id).unwrap().max_slot, 9);

    // 거부된 값이 새어 들어가지 않았다.
    let _ = update_school_impl(&conn, id, TEST_SCHOOL, 10, 7, true);
    assert_eq!(get_school_impl(&conn, id).unwrap().max_slot, 9);
}

#[test]
fn school_name_cannot_be_blank_and_due_days_cannot_be_negative() {
    let conn = setup_test_db();
    let id = school_id(&conn);

    assert!(update_school_impl(&conn, id, "   ", 7, 7, true)
        .unwrap_err()
        .contains("학교 이름"));
    assert!(update_school_impl(&conn, id, TEST_SCHOOL, 7, -1, true)
        .unwrap_err()
        .contains("제출 기한"));
}

#[test]
fn updating_a_missing_school_is_reported_instead_of_passing_silently() {
    let conn = setup_test_db();
    let err = update_school_impl(&conn, 9999, "가나고등학교", 7, 7, true).unwrap_err();
    assert!(err.contains("학교를 찾을 수 없습니다"), "{err}");
}

#[test]
fn every_school_setting_survives_a_round_trip() {
    // 제출 기한과 주말 건너뛰기는 기한일 계산으로 곧장 들어간다. 두 값이 서로 바뀐 채
    // 저장돼도 화면에서는 한동안 티가 나지 않고, 그때는 이미 기한일이 박힌 기록이 쌓인 뒤다.
    // SQLite는 열의 형을 강제하지 않으므로 자리가 어긋나도 오류가 나지 않는다.
    let conn = setup_test_db();
    let id = school_id(&conn);
    update_school_impl(&conn, id, "  가나고등학교  ", 6, 3, false).unwrap();

    let back = get_school_impl(&conn, id).unwrap();
    assert_eq!(back.name, "가나고등학교");
    assert_eq!(back.max_slot, 6);
    assert_eq!(back.due_days, 3);
    assert!(!back.due_skip_offdays);
    assert!(back.active);
}

#[test]
fn a_travelling_teacher_can_register_a_second_school() {
    // 학교 단위 값(최대 교시 · 제출 기한)이 school 행에 있는 이유가 이것이다.
    // 두 학교가 같은 목록에 나란히 표시되고, 각자의 설정을 따로 들고 있어야 한다.
    let conn = setup_test_db();
    let y = year_id(&conn);

    let id = create_school_impl(&conn, y, "  나다고등학교  ", 6, 3, true).unwrap();
    let schools = get_schools_impl(&conn, y).unwrap();
    assert_eq!(schools.len(), 2);
    // 뒤에 붙는다.
    assert_eq!(schools.last().unwrap().id, id);

    let made = get_school_impl(&conn, id).unwrap();
    assert_eq!(made.name, "나다고등학교");
    assert_eq!(made.max_slot, 6);
    assert_eq!(made.due_days, 3);
    assert!(made.active);
    // 먼저 있던 학교의 설정은 그대로다.
    assert_eq!(get_school_impl(&conn, school_id(&conn)).unwrap().max_slot, 7);
}

/// **기본 태그와 한도 규정은 학교를 만들 때 함께 들어간다.**
///
/// 시드에 두면 한 번만 돌므로 둘째 학교가 빈 목록으로 시작한다. 그 상태에서는
/// 통계 화면에 셀 것이 하나도 없고, 교사는 무엇을 먼저 만들어야 하는지 알 수 없다.
#[test]
fn a_new_school_starts_with_the_default_tags_and_rules() {
    let conn = setup_test_db();
    let second = create_school_impl(&conn, year_id(&conn), "나다고등학교", 7, 7, true).unwrap();

    for school in [school_id(&conn), second] {
        let tags: Vec<String> = get_tags_impl(&conn, school)
            .unwrap()
            .into_iter()
            .map(|t| t.name)
            .collect();
        assert_eq!(tags, vec!["체험학습", "생리통"], "학교 {school}");

        let rules = get_quota_rules_impl(&conn, school).unwrap();
        let names: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["체험학습 연 20일", "생리통 월 1회"]);
        // 규정은 그 학교의 태그를 가리킨다. 남의 학교 태그를 세면 안 된다.
        assert_eq!(rules[0].tag_name.as_deref(), Some("체험학습"));
        assert_eq!(rules[1].tag_name.as_deref(), Some("생리통"));
        // 최초 집합에는 시작일이 없다 — 설치 전 날짜의 출결도 이 태그를 달 수 있어야 한다.
        assert!(rules.iter().all(|r| r.valid_from == "1900-01-01"));
    }

    let crossed = count_where(
        &conn,
        "SELECT COUNT(*) FROM quota_rule q
         JOIN span_tag t ON t.id = q.tag_id
         WHERE q.school_id <> t.school_id",
    );
    assert_eq!(crossed, 0, "규정과 태그는 같은 학교에 속한다");
}

#[test]
fn a_new_school_is_validated_the_same_way_an_edited_one_is() {
    // 확인을 나누어 두면 새로 만드는 길로만 최대 교시 10이 들어온다.
    let conn = setup_test_db();
    let y = year_id(&conn);

    assert!(create_school_impl(&conn, y, "   ", 7, 7, true)
        .unwrap_err()
        .contains("학교 이름"));
    assert!(create_school_impl(&conn, y, "라마고등학교", 10, 7, true)
        .unwrap_err()
        .contains("최대 교시"));
    assert!(create_school_impl(&conn, y, "라마고등학교", 7, -1, true)
        .unwrap_err()
        .contains("제출 기한"));

    assert_eq!(count(&conn, "school"), 1, "거절된 학교가 새어 들어갔다");
}

/// 거절된 학교가 태그만 남기고 사라지면 안 된다. 한 트랜잭션이라 통째로 되돌아간다.
#[test]
fn a_refused_school_leaves_no_tags_behind() {
    let conn = setup_test_db();
    let tags_before = count(&conn, "span_tag");
    let rules_before = count(&conn, "quota_rule");

    // 같은 학년도의 같은 이름 — 학교 INSERT에서 거절된다.
    assert!(create_school_impl(&conn, year_id(&conn), TEST_SCHOOL, 7, 7, true).is_err());

    assert_eq!(count(&conn, "span_tag"), tags_before);
    assert_eq!(count(&conn, "quota_rule"), rules_before);
    // 트랜잭션이 열린 채 남지 않았다.
    assert!(create_tag_impl(&conn, school_id(&conn), "병결").is_ok());
}

#[test]
fn a_retired_school_leaves_the_list_but_can_still_be_read() {
    let conn = setup_test_db();
    let id = school_id(&conn);
    retire_school_impl(&conn, id).unwrap();

    assert!(get_schools_impl(&conn, year_id(&conn)).unwrap().is_empty());
    assert!(!get_school_impl(&conn, id).unwrap().active);
    // 지운 것이 아니다 — 그 학교의 기록이 그대로 남아야 한다.
    assert_eq!(count(&conn, "school"), 1);

    // 두 번 마감하지 않는다.
    assert!(retire_school_impl(&conn, id)
        .unwrap_err()
        .contains("유효한 학교를 찾을 수 없습니다"));
}

// ── 휴업일 ────────────────────────────────────────────────────

#[test]
fn off_days_come_back_in_date_order_and_can_be_bounded() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    add_off_day_impl(&conn, s, &off("2026-10-09", Some("한글날"))).unwrap();
    add_off_day_impl(&conn, s, &off("2026-05-01", Some("재량휴업일"))).unwrap();
    add_off_day_impl(&conn, s, &off("2026-10-03", Some("개천절"))).unwrap();

    let all = get_off_days_impl(&conn, s, None, None).unwrap();
    let dates: Vec<&str> = all.iter().map(|d| d.date.as_str()).collect();
    assert_eq!(dates, vec!["2026-05-01", "2026-10-03", "2026-10-09"]);

    let autumn = get_off_days_impl(&conn, s, Some("2026-10-01"), Some("2026-10-05")).unwrap();
    assert_eq!(autumn.len(), 1);
    assert_eq!(autumn[0].label.as_deref(), Some("개천절"));

    // 한쪽만 준 경우도 걸린다.
    assert_eq!(
        get_off_days_impl(&conn, s, Some("2026-10-01"), None)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn adding_the_same_off_day_twice_updates_the_label_and_keeps_the_id() {
    // 같은 날을 다시 넣는 것은 실수가 아니라 이름을 고치는 동작이다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let first = add_off_day_impl(&conn, s, &off("2026-10-03", Some("개천절"))).unwrap();
    let second = add_off_day_impl(&conn, s, &off("2026-10-03", Some("개천절 대체휴업"))).unwrap();

    assert_eq!(first, second);
    let days = get_off_days_impl(&conn, s, None, None).unwrap();
    assert_eq!(days.len(), 1);
    assert_eq!(days[0].label.as_deref(), Some("개천절 대체휴업"));
}

#[test]
fn an_off_day_date_must_be_iso() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let err = add_off_day_impl(&conn, s, &off("2026.10.03.(토)", None)).unwrap_err();
    assert!(err.contains("날짜 형식"), "{err}");
    assert!(get_off_days_impl(&conn, s, None, None).unwrap().is_empty());

    assert!(get_off_days_impl(&conn, s, Some("10월"), None)
        .unwrap_err()
        .contains("날짜 형식"));
}

#[test]
fn removing_a_missing_off_day_is_reported() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let id = add_off_day_impl(&conn, s, &off("2026-10-03", None)).unwrap();
    remove_off_day_impl(&conn, id).unwrap();
    assert!(get_off_days_impl(&conn, s, None, None).unwrap().is_empty());

    let err = remove_off_day_impl(&conn, id).unwrap_err();
    assert!(err.contains("휴업일을 찾을 수 없습니다"), "{err}");
}

// ── 태그 ──────────────────────────────────────────────────────

#[test]
fn tags_are_school_settings_not_globals() {
    let conn = setup_test_db();
    let names: Vec<String> = get_tags_impl(&conn, school_id(&conn))
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(names, vec!["체험학습", "생리통"]);

    // 다른 학교의 태그는 섞이지 않는다.
    assert!(get_tags_impl(&conn, 9999).unwrap().is_empty());
}

#[test]
fn a_new_tag_is_valid_from_the_beginning_so_past_records_can_carry_it() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let id = create_tag_impl(&conn, s, "  교외대회  ").unwrap();

    let tags = get_tags_impl(&conn, s).unwrap();
    let made = tags.iter().find(|t| t.id == id).unwrap();
    assert_eq!(made.name, "교외대회");
    assert_eq!(made.valid_from, "1900-01-01");
    // 뒤에 붙는다.
    assert_eq!(tags.last().unwrap().id, id);
}

#[test]
fn a_tag_name_cannot_be_blank_or_repeat_a_live_one() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    assert!(create_tag_impl(&conn, s, "   ").unwrap_err().contains("비어"));

    let err = create_tag_impl(&conn, s, "체험학습").unwrap_err();
    assert!(err.contains("이미"), "영문 원문이 새어나왔다: {err}");

    // 마감된 이름은 다시 쓸 수 있다.
    retire_tag_impl(&conn, tag_id(&conn, "생리통"), TODAY).unwrap();
    assert!(create_tag_impl(&conn, s, "생리통").is_ok());
}

#[test]
fn renaming_a_tag_leaves_the_old_row_behind() {
    // UPDATE로 고치면 그 태그를 가진 과거 구간의 이름까지 소급 변경된다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let old = tag_id(&conn, "체험학습");

    let new = rename_tag_impl(&conn, old, "학교장허가 체험학습", TODAY).unwrap();
    assert_ne!(new, old);

    let names: Vec<String> = get_tags_impl(&conn, s)
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(names, vec!["학교장허가 체험학습", "생리통"]);

    // 옛 행은 지워지지 않고 마감됐다.
    assert_eq!(tag_valid_to(&conn, old).as_deref(), Some(TODAY));
    assert_eq!(tag_valid_to(&conn, new), None);
    assert_eq!(count(&conn, "span_tag"), 3);
}

#[test]
fn renaming_a_tag_moves_its_quota_rule_onto_the_new_tag() {
    // 남겨두면 목록에서 사라진 태그를 가리키는 규정이 계속 유효한 것으로 조회된다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let old = tag_id(&conn, "체험학습");
    let new = rename_tag_impl(&conn, old, "학교장허가 체험학습", TODAY).unwrap();

    let rules = get_quota_rules_impl(&conn, s).unwrap();
    let moved = rules
        .iter()
        .find(|r| r.name == "체험학습 연 20일")
        .expect("규정이 사라졌다");
    assert_eq!(moved.tag_id, Some(new));
    assert_eq!(moved.tag_name.as_deref(), Some("학교장허가 체험학습"));
    assert_eq!(moved.valid_from, TODAY);
    assert_eq!(moved.limit_n, 20);

    // 옛 규정 행도 남는다 — 그때의 태그를 가리킨 채로.
    assert_eq!(count(&conn, "quota_rule"), 3);
}

#[test]
fn a_rename_that_collides_rolls_back_and_leaves_the_old_tag_live() {
    // 클로저 밖에서 `?`를 쓰면 ROLLBACK을 건너뛴다. 여기서 그것을 잡는다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    create_tag_impl(&conn, s, "특별휴가").unwrap();
    let old = tag_id(&conn, "체험학습");

    let err = rename_tag_impl(&conn, old, "특별휴가", TODAY).unwrap_err();
    assert!(err.contains("이미"), "{err}");

    assert_eq!(tag_valid_to(&conn, old), None);
    let names: Vec<String> = get_tags_impl(&conn, s)
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(names, vec!["체험학습", "생리통", "특별휴가"]);
    // 규정도 그대로다.
    assert_eq!(count(&conn, "quota_rule"), 2);

    // 트랜잭션이 열린 채 남지 않았다.
    assert!(create_tag_impl(&conn, s, "병결").is_ok());
}

#[test]
fn renaming_a_tag_to_the_same_name_leaves_the_lineage_alone() {
    // 교사가 이름 칸을 열었다가 그대로 저장하는 일은 흔하다. 그때마다 마감 후 추가를
    // 하면 과거 구간이 가리키는 태그 id만 분리되고 얻는 것이 없다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let tags_before = count(&conn, "span_tag");
    let rules_before = count(&conn, "quota_rule");
    let old = tag_id(&conn, "체험학습");

    // 앞뒤 공백만 다른 것도 같은 이름이다.
    let same = rename_tag_impl(&conn, old, "  체험학습  ", TODAY).unwrap();

    assert_eq!(same, old);
    assert_eq!(tag_valid_to(&conn, old), None);
    assert_eq!(count(&conn, "span_tag"), tags_before);
    assert_eq!(count(&conn, "quota_rule"), rules_before);
    assert_eq!(
        get_quota_rules_impl(&conn, s).unwrap()[0].tag_id,
        Some(old)
    );
}

#[test]
fn renaming_a_missing_or_already_retired_tag_is_reported() {
    let conn = setup_test_db();
    let live = tag_id(&conn, "생리통");
    assert!(rename_tag_impl(&conn, 9999, "무엇", TODAY)
        .unwrap_err()
        .contains("유효한 태그를 찾을 수 없습니다"));
    assert!(rename_tag_impl(&conn, live, "  ", TODAY)
        .unwrap_err()
        .contains("비어"));

    retire_tag_impl(&conn, live, TODAY).unwrap();
    assert!(rename_tag_impl(&conn, live, "생리 조퇴", TODAY)
        .unwrap_err()
        .contains("유효한 태그를 찾을 수 없습니다"));
}

#[test]
fn retiring_a_tag_is_not_a_delete() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let before = count(&conn, "span_tag");
    let id = tag_id(&conn, "생리통");

    retire_tag_impl(&conn, id, TODAY).unwrap();

    assert_eq!(count(&conn, "span_tag"), before);
    assert_eq!(tag_valid_to(&conn, id).as_deref(), Some(TODAY));
    assert!(!get_tags_impl(&conn, s).unwrap().iter().any(|t| t.id == id));

    // 두 번 마감하지 않는다.
    assert!(retire_tag_impl(&conn, id, TODAY)
        .unwrap_err()
        .contains("유효한 태그를 찾을 수 없습니다"));
}

#[test]
fn retiring_a_tag_also_retires_the_rules_that_count_it() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    retire_tag_impl(&conn, tag_id(&conn, "생리통"), TODAY).unwrap();

    let names: Vec<String> = get_quota_rules_impl(&conn, s)
        .unwrap()
        .into_iter()
        .map(|r| r.name)
        .collect();
    assert_eq!(names, vec!["체험학습 연 20일"]);
    // 마감이지 삭제가 아니다.
    assert_eq!(count(&conn, "quota_rule"), 2);
}

// ── 한도 규정 ─────────────────────────────────────────────────

#[test]
fn quota_rules_carry_the_tag_name_they_count() {
    let conn = setup_test_db();
    let rules = get_quota_rules_impl(&conn, school_id(&conn)).unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].name, "체험학습 연 20일");
    assert_eq!(rules[0].tag_name.as_deref(), Some("체험학습"));
    assert_eq!(rules[0].period, "year");
    assert_eq!(rules[0].unit, "day");
    assert_eq!(rules[1].tag_name.as_deref(), Some("생리통"));
    assert_eq!(rules[1].unit, "count");
}

#[test]
fn a_rule_period_and_unit_must_be_values_the_app_knows() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let tag = Some(tag_id(&conn, "체험학습"));

    let err = create_quota_rule_impl(&conn, s, &new_rule("주 1회", tag, "week", 1, "day"))
        .unwrap_err();
    assert!(err.contains("year / month"), "{err}");

    let err = create_quota_rule_impl(&conn, s, &new_rule("연 3시간", tag, "year", 3, "hour"))
        .unwrap_err();
    assert!(err.contains("day / count"), "{err}");

    // 거부된 규정은 저장되지 않았다.
    assert_eq!(get_quota_rules_impl(&conn, s).unwrap().len(), 2);
}

#[test]
fn a_rule_limit_must_be_positive_and_named() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let tag = Some(tag_id(&conn, "체험학습"));

    for bad in [0, -3] {
        let err = create_quota_rule_impl(&conn, s, &new_rule("한도", tag, "year", bad, "day"))
            .unwrap_err();
        assert!(err.contains("한도는 1 이상"), "{err}");
    }
    let err = create_quota_rule_impl(&conn, s, &new_rule("  ", tag, "year", 3, "day")).unwrap_err();
    assert!(err.contains("비어"), "{err}");
}

#[test]
fn a_rule_without_a_start_date_counts_from_the_beginning() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let tag = Some(tag_id(&conn, "생리통"));

    let id = create_quota_rule_impl(&conn, s, &new_rule("생리통 월 1회", tag, "month", 1, "count"))
        .unwrap();
    let rules = get_quota_rules_impl(&conn, s).unwrap();
    let made = rules.iter().find(|r| r.id == id).unwrap();
    assert_eq!(made.valid_from, "1900-01-01");
    // 뒤에 붙는다.
    assert_eq!(rules.last().unwrap().id, id);

    let mut dated = new_rule("교외대회 연 5일", None, "year", 5, "day");
    dated.valid_from = "2026-03-02".to_string();
    let id = create_quota_rule_impl(&conn, s, &dated).unwrap();
    let rules = get_quota_rules_impl(&conn, s).unwrap();
    let made = rules.iter().find(|r| r.id == id).unwrap();
    assert_eq!(made.valid_from, "2026-03-02");
    // 태그 없는 규정도 있을 수 있다 — 그 학교의 모든 구간을 센다.
    assert_eq!(made.tag_id, None);
    assert_eq!(made.tag_name, None);
}

#[test]
fn revising_a_rule_closes_the_old_row_and_adds_a_new_one() {
    // 한도는 학교마다 해마다 바뀐다. 지난해 규정이 남아야 그때 기록을 설명할 수 있다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let old_id = quota_rule_id(&conn, "체험학습 연 20일");
    let mut revised = get_quota_rules_impl(&conn, s)
        .unwrap()
        .into_iter()
        .find(|r| r.id == old_id)
        .unwrap();
    revised.name = "체험학습 연 15일".to_string();
    revised.limit_n = 15;

    let new_id = revise_quota_rule_impl(&conn, &revised, TODAY).unwrap();
    assert_ne!(new_id, old_id);

    let rules = get_quota_rules_impl(&conn, s).unwrap();
    assert_eq!(rules.len(), 2);
    let made = rules.iter().find(|r| r.id == new_id).unwrap();
    assert_eq!(made.limit_n, 15);
    assert_eq!(made.valid_from, TODAY);
    assert_eq!(made.sort_order, revised.sort_order);

    let old_valid_to: Option<String> = conn
        .query_row(
            "SELECT valid_to FROM quota_rule WHERE id = ?1",
            rusqlite::params![old_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(old_valid_to.as_deref(), Some(TODAY));
}

#[test]
fn revising_a_missing_rule_is_reported_and_closes_its_transaction() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let mut orphan = new_rule("없는 규정", None, "year", 5, "day");
    orphan.id = 9999;

    let err = revise_quota_rule_impl(&conn, &orphan, TODAY).unwrap_err();
    assert!(err.contains("한도 규정을 찾을 수 없습니다"), "{err}");
    assert_eq!(count(&conn, "quota_rule"), 2);

    // 트랜잭션이 열린 채 남았다면 다음 쓰기가 실패한다.
    assert!(create_tag_impl(&conn, s, "병결").is_ok());
}

#[test]
fn a_revision_is_validated_before_anything_is_closed() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let mut revised = get_quota_rules_impl(&conn, s).unwrap()[0].clone();
    revised.unit = "hour".to_string();

    assert!(revise_quota_rule_impl(&conn, &revised, TODAY)
        .unwrap_err()
        .contains("day / count"));
    assert_eq!(get_quota_rules_impl(&conn, s).unwrap().len(), 2);
    assert_eq!(count(&conn, "quota_rule"), 2);
}

#[test]
fn a_rule_cannot_count_a_tag_that_is_gone_or_belongs_to_another_school() {
    // 마감을 막아 놓고 생성으로 다시 만들 수 있으면 막은 것이 아니다. 태그 목록에
    // 없는 것을 세는 규정이 설정 화면에 남으면 교사는 그 줄을 설명할 수 없다.
    let conn = setup_test_db();
    let s = school_id(&conn);

    let dead = tag_id(&conn, "생리통");
    retire_tag_impl(&conn, dead, TODAY).unwrap();
    let err = create_quota_rule_impl(&conn, s, &new_rule("마감된 태그", Some(dead), "year", 5, "day"))
        .unwrap_err();
    assert!(err.contains("유효한 태그가 아닙니다"), "{err}");

    // 다른 학교의 태그도 막는다. FK는 span_tag의 존재만 볼 뿐 어느 학교의 것인지 보지 않는다.
    let other = insert_school(&conn, year_id(&conn), "옆 학교");
    let other_tag = create_tag_impl(&conn, other, "옆 학교 태그").unwrap();
    let err = create_quota_rule_impl(&conn, s, &new_rule("남의 태그", Some(other_tag), "year", 5, "day"))
        .unwrap_err();
    assert!(err.contains("유효한 태그가 아닙니다"), "{err}");

    // 아예 없는 태그도 같은 문장으로 막는다. FK까지 가지 않는다.
    let err = create_quota_rule_impl(&conn, s, &new_rule("유령", Some(9999), "year", 5, "day"))
        .unwrap_err();
    assert!(err.contains("유효한 태그가 아닙니다"), "{err}");

    // 태그 없는 규정은 그대로 통과한다 — 두 축만으로 세는 규정이 있을 수 있다.
    assert!(create_quota_rule_impl(&conn, s, &new_rule("전체 결석", None, "year", 5, "day")).is_ok());
    // 내 학교의 규정만 센다. 옆 학교는 만들 때 받은 기본 규정 둘을 따로 들고 있다.
    assert_eq!(get_quota_rules_impl(&conn, s).unwrap().len(), 2);
    assert_eq!(get_quota_rules_impl(&conn, other).unwrap().len(), 2);
}

#[test]
fn a_revision_onto_a_retired_tag_rolls_back_and_leaves_the_old_rule_live() {
    // 마감이 먼저 일어나므로, 뒤이은 삽입이 거부되면 반드시 되돌아와야 한다.
    let conn = setup_test_db();
    let s = school_id(&conn);
    let dead = tag_id(&conn, "생리통");
    retire_tag_impl(&conn, dead, TODAY).unwrap();

    let mut revised = get_quota_rules_impl(&conn, s).unwrap()[0].clone();
    let live_id = revised.id;
    revised.tag_id = Some(dead);

    let err = revise_quota_rule_impl(&conn, &revised, TODAY).unwrap_err();
    assert!(err.contains("유효한 태그가 아닙니다"), "{err}");

    let rules = get_quota_rules_impl(&conn, s).unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].id, live_id);
    assert_eq!(rules[0].tag_name.as_deref(), Some("체험학습"));

    // 트랜잭션이 열린 채 남지 않았다.
    assert!(create_tag_impl(&conn, s, "병결").is_ok());
}

#[test]
fn retiring_a_rule_is_not_a_delete() {
    let conn = setup_test_db();
    let s = school_id(&conn);
    let id = quota_rule_id(&conn, "생리통 월 1회");

    retire_quota_rule_impl(&conn, id, TODAY).unwrap();
    assert_eq!(count(&conn, "quota_rule"), 2);
    assert_eq!(get_quota_rules_impl(&conn, s).unwrap().len(), 1);

    assert!(retire_quota_rule_impl(&conn, id, TODAY)
        .unwrap_err()
        .contains("유효한 한도 규정을 찾을 수 없습니다"));
}

#[test]
fn dates_handed_to_the_rule_and_tag_commands_must_be_iso() {
    let conn = setup_test_db();
    let tag = tag_id(&conn, "생리통");
    let rule = quota_rule_id(&conn, "생리통 월 1회");

    assert!(rename_tag_impl(&conn, tag, "생리 조퇴", "오늘")
        .unwrap_err()
        .contains("날짜 형식"));
    assert!(retire_tag_impl(&conn, tag, "2026/09/11")
        .unwrap_err()
        .contains("날짜 형식"));
    assert!(retire_quota_rule_impl(&conn, rule, "")
        .unwrap_err()
        .contains("날짜 형식"));
    // 아무것도 마감되지 않았다.
    assert_eq!(tag_valid_to(&conn, tag), None);
    assert_eq!(get_quota_rules_impl(&conn, school_id(&conn)).unwrap().len(), 2);
}
