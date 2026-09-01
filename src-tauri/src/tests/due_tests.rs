use crate::due::*;
use chrono::NaiveDate;

fn d(s: &str) -> NaiveDate {
    parse_date(s).unwrap()
}

#[test]
fn calendar_days_include_weekend() {
    // 2026-08-26은 수요일
    assert_eq!(due_date(d("2026-08-26"), 5, true), d("2026-08-31"));
}

#[test]
fn business_days_skip_weekend() {
    // 수 → 목금(2) → 월화수(5)
    assert_eq!(due_date(d("2026-08-26"), 5, false), d("2026-09-02"));
}

#[test]
fn zero_due_days_means_same_day() {
    // 교사가 0을 넣었으면 그날이 마감이다. 주말이어도 옮기지 않는다.
    assert_eq!(due_date(d("2026-08-29"), 0, false), d("2026-08-29"));
}

#[test]
fn business_day_count_starting_on_friday() {
    // 금요일 + 1영업일 = 월요일
    assert_eq!(due_date(d("2026-08-28"), 1, false), d("2026-08-31"));
}

#[test]
fn weekend_detection() {
    assert!(is_weekend(d("2026-08-29"))); // 토
    assert!(is_weekend(d("2026-08-30"))); // 일
    assert!(!is_weekend(d("2026-08-31"))); // 월
}

#[test]
fn weekdays_between_excludes_weekend() {
    let days = weekdays_between(d("2026-08-28"), d("2026-09-01"));
    assert_eq!(
        days.iter().map(|x| format_date(*x)).collect::<Vec<_>>(),
        vec!["2026-08-28", "2026-08-31", "2026-09-01"]
    );
}

#[test]
fn weekdays_between_single_day() {
    assert_eq!(weekdays_between(d("2026-08-26"), d("2026-08-26")).len(), 1);
    // 주말 하루만 고르면 후보가 없다.
    assert!(weekdays_between(d("2026-08-29"), d("2026-08-29")).is_empty());
}

#[test]
fn overdue_is_negative_before_due() {
    assert_eq!(days_overdue(d("2026-09-02"), d("2026-08-31")), -2);
    assert_eq!(days_overdue(d("2026-09-02"), d("2026-09-02")), 0);
    assert_eq!(days_overdue(d("2026-09-02"), d("2026-09-05")), 3);
}

#[test]
fn stored_dates_are_iso_and_display_is_korean() {
    assert_eq!(format_date(d("2026-07-15")), "2026-07-15");
    assert_eq!(format_korean(d("2026-07-15")), "2026.07.15.(수)");
}

#[test]
fn bad_date_is_rejected_with_korean_message() {
    let err = parse_date("2026/07/15").unwrap_err();
    assert!(err.contains("날짜 형식"));
}
