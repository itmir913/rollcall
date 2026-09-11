//! `due.rs` — 마감일 계산과 날짜 유틸.
//!
//! 기한은 학교 설정 두 값으로 정해진다 — `due_days`와 `due_skip_offdays`.
//! 건너뛸 날은 주말과 **교사가 등록한 휴업일**이다. 공휴일 API를 부르지 않으므로
//! 개교기념일·재량휴업일은 `off_day` 행으로만 들어온다.

use crate::due::*;
use chrono::NaiveDate;
use std::collections::HashSet;

fn d(s: &str) -> NaiveDate {
    parse_date(s).unwrap()
}

fn off(dates: &[&str]) -> HashSet<NaiveDate> {
    dates.iter().map(|s| d(s)).collect()
}

fn none() -> HashSet<NaiveDate> {
    HashSet::new()
}

// ─── 마감일 ────────────────────────────────────────────────────

#[test]
fn due_date_skips_the_weekend() {
    // 2026-09-10은 목요일. 금(1) → 토·일 건너뜀 → 월(2) → 화(3)
    assert_eq!(due_date(d("2026-09-10"), 3, true, &none()), d("2026-09-15"));
}

#[test]
fn due_date_skips_registered_off_days_too() {
    // 월요일(09-14)을 재량휴업일로 등록하면 마감이 하루 더 밀린다.
    assert_eq!(
        due_date(d("2026-09-10"), 3, true, &off(&["2026-09-14"])),
        d("2026-09-16")
    );
}

#[test]
fn due_date_never_lands_on_an_off_day() {
    // 금요일부터 등록된 휴업일이면 다음 근무일까지 밀린다.
    let holidays = off(&["2026-09-11"]);
    let due = due_date(d("2026-09-10"), 1, true, &holidays);
    assert_eq!(due, d("2026-09-14"));
    assert!(!is_off_day(due, &holidays));
}

#[test]
fn due_date_uses_the_plain_calendar_when_skip_is_off() {
    // 설정이 꺼져 있으면 주말도 휴업일도 세지 않고 그대로 더한다.
    assert_eq!(
        due_date(d("2026-09-11"), 1, false, &off(&["2026-09-12"])),
        d("2026-09-12")
    );
    assert_eq!(
        due_date(d("2026-09-10"), 5, false, &none()),
        d("2026-09-15")
    );
}

#[test]
fn zero_due_days_means_the_base_day_itself() {
    // 교사가 0을 넣었으면 그날이 마감이다. 토요일이어도 옮기지 않는다 —
    // 교사가 정한 값을 프로그램이 조정하지 않는다.
    assert_eq!(due_date(d("2026-09-12"), 0, true, &none()), d("2026-09-12"));
    assert_eq!(
        due_date(d("2026-09-12"), 0, false, &none()),
        d("2026-09-12")
    );
    assert_eq!(
        due_date(d("2026-09-12"), 0, true, &off(&["2026-09-12"])),
        d("2026-09-12")
    );
}

#[test]
fn due_date_crossing_a_month_boundary() {
    // 2026-08-31은 월요일. 화~금이 4일이고 주말을 건너뛴 다음 월요일이 5일째다.
    assert_eq!(due_date(d("2026-08-31"), 5, true, &none()), d("2026-09-07"));
}

#[test]
fn due_date_terminates_even_if_every_day_is_an_off_day() {
    // 휴업일을 아무리 많이 등록해도 1년 안에서 끝낸다. 무한 루프가 나면
    // 마감일을 계산하는 화면이 통째로 멈춘다.
    let base = d("2026-09-10");
    let all: HashSet<NaiveDate> = (0..400).map(|n| base + chrono::Duration::days(n)).collect();
    let due = due_date(base, 5, true, &all);
    assert_eq!(due, base + chrono::Duration::days(366));
}

// ─── 셀 수 있는 날 ─────────────────────────────────────────────

#[test]
fn open_days_exclude_the_weekend() {
    let days = open_days_between(d("2026-09-11"), d("2026-09-15"), &none());
    assert_eq!(
        days.iter().map(|x| format_date(*x)).collect::<Vec<_>>(),
        vec!["2026-09-11", "2026-09-14", "2026-09-15"]
    );
}

#[test]
fn open_days_exclude_registered_off_days() {
    let days = open_days_between(d("2026-09-11"), d("2026-09-15"), &off(&["2026-09-14"]));
    assert_eq!(
        days.iter().map(|x| format_date(*x)).collect::<Vec<_>>(),
        vec!["2026-09-11", "2026-09-15"]
    );
}

#[test]
fn open_days_for_a_single_day() {
    assert_eq!(
        open_days_between(d("2026-09-10"), d("2026-09-10"), &none()).len(),
        1
    );
    // 주말 하루만 고르면 후보가 없다.
    assert!(open_days_between(d("2026-09-12"), d("2026-09-12"), &none()).is_empty());
    // 등록된 휴업일 하루만 골라도 마찬가지다.
    assert!(open_days_between(d("2026-09-10"), d("2026-09-10"), &off(&["2026-09-10"])).is_empty());
}

#[test]
fn open_days_are_empty_when_the_range_is_reversed() {
    // 끝이 시작보다 앞이면 후보가 없다. 여기서 막지 않으면 호출한 쪽이 빈 목록을
    // 받지 못하고 무한히 돈다.
    assert!(open_days_between(d("2026-09-15"), d("2026-09-11"), &none()).is_empty());
}

// ─── 경과일 ────────────────────────────────────────────────────

#[test]
fn overdue_is_negative_before_the_due_date() {
    assert_eq!(days_overdue(d("2026-09-10"), d("2026-09-08")), -2);
}

#[test]
fn overdue_is_zero_on_the_due_date() {
    assert_eq!(days_overdue(d("2026-09-10"), d("2026-09-10")), 0);
}

#[test]
fn overdue_is_positive_after_the_due_date() {
    assert_eq!(days_overdue(d("2026-09-10"), d("2026-09-13")), 3);
}

// ─── 휴업일 판정 ───────────────────────────────────────────────

#[test]
fn weekend_detection() {
    assert!(is_weekend(d("2026-09-12"))); // 토
    assert!(is_weekend(d("2026-09-13"))); // 일
    assert!(!is_weekend(d("2026-09-14"))); // 월
}

#[test]
fn off_day_covers_both_the_weekend_and_the_register() {
    let holidays = off(&["2026-09-14"]);
    assert!(is_off_day(d("2026-09-12"), &holidays)); // 주말
    assert!(is_off_day(d("2026-09-14"), &holidays)); // 등록된 휴업일
    assert!(!is_off_day(d("2026-09-15"), &holidays)); // 평범한 화요일
}

// ─── 표기 ──────────────────────────────────────────────────────

#[test]
fn stored_dates_are_iso() {
    assert_eq!(format_date(d("2026-07-15")), "2026-07-15");
    assert_eq!(format_date(d("2026-01-05")), "2026-01-05");
}

#[test]
fn korean_label_includes_the_weekday() {
    assert_eq!(format_korean(d("2026-07-15")), "2026.07.15.(수)");
    assert_eq!(format_korean(d("2027-01-01")), "2027.01.01.(금)");
}

#[test]
fn korean_label_covers_every_weekday() {
    // 2026-09-14는 월요일. 요일 배열이 한 칸 밀리면 여기서 잡힌다.
    let week = [
        ("2026-09-14", "월"),
        ("2026-09-15", "화"),
        ("2026-09-16", "수"),
        ("2026-09-17", "목"),
        ("2026-09-18", "금"),
        ("2026-09-19", "토"),
        ("2026-09-20", "일"),
    ];
    for (iso, weekday) in week {
        assert_eq!(
            format_korean(d(iso)),
            format!("{}.({weekday})", iso.replace('-', "."))
        );
    }
}

#[test]
fn bad_date_is_rejected_with_a_korean_message() {
    let err = parse_date("2026/07/15").unwrap_err();
    assert!(err.contains("날짜 형식"), "실제 문구: {err}");
    assert!(err.contains("2026/07/15"), "실제 문구: {err}");
    assert!(parse_date("2026-02-30").is_err());
    assert!(parse_date("").is_err());
}

// ─── 학년도 · 학기 ─────────────────────────────────────────────

#[test]
fn academic_year_starts_in_march() {
    assert_eq!(academic_year_of(d("2026-03-01")), 2026);
    assert_eq!(academic_year_of(d("2026-12-31")), 2026);
}

#[test]
fn january_and_february_belong_to_the_previous_academic_year() {
    // 2027년 1월의 출결은 2026학년도 기록이다. 여기서 틀리면 학년도 필터가
    // 3학년 2학기 기록을 통째로 놓친다.
    assert_eq!(academic_year_of(d("2027-01-15")), 2026);
    assert_eq!(academic_year_of(d("2027-02-28")), 2026);
}

