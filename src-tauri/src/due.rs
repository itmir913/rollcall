//! 마감일 계산과 날짜 유틸. 순수 함수다.
//!
//! `due_days`와 `include_weekend` 두 값으로만 계산한다. **공휴일 캘린더는 도입하지
//! 않는다.** 목적은 규정을 정확히 모델링하는 것이 아니라 "이 학생 서류 아직 안 냈다"를
//! 교사가 놓치지 않는 것이다. 하루 이틀 오차는 실무에 지장이 없고, 계산된 마감일은
//! `daily_check.due_date`에 저장되어 교사가 직접 고칠 수 있다. 그것이 탈출구다.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

pub const DATE_FMT: &str = "%Y-%m-%d";

pub fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, DATE_FMT).map_err(|_| format!("날짜 형식이 올바르지 않습니다: {s}"))
}

pub fn format_date(d: NaiveDate) -> String {
    d.format(DATE_FMT).to_string()
}

pub fn is_weekend(d: NaiveDate) -> bool {
    matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
}

/// 기준일로부터 마감일. `include_weekend`가 false면 주말을 세지 않는다.
///
/// `due_days == 0`이면 기준일이 곧 마감일이다(주말이어도 옮기지 않는다 —
/// 교사가 정한 값을 프로그램이 조정하지 않는다).
pub fn due_date(base: NaiveDate, due_days: i64, include_weekend: bool) -> NaiveDate {
    if include_weekend {
        return base + Duration::days(due_days);
    }
    let mut remaining = due_days;
    let mut cursor = base;
    while remaining > 0 {
        cursor += Duration::days(1);
        if !is_weekend(cursor) {
            remaining -= 1;
        }
    }
    cursor
}

/// 기간 안의 평일 목록. 기간 일괄 입력의 기본 후보다.
///
/// 재량휴업일·공휴일은 여기서 빼지 않는다. 미리보기에서 교사가 지운다.
/// 캘린더 테이블을 만들지 않기로 한 결정의 대가이자, 학교마다 다른 휴업일을
/// 프로그램이 알 수 없다는 사실의 인정이다.
pub fn weekdays_between(from: NaiveDate, to: NaiveDate) -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let mut cursor = from;
    while cursor <= to {
        if !is_weekend(cursor) {
            out.push(cursor);
        }
        cursor += Duration::days(1);
    }
    out
}

/// 마감일 기준 경과일. 음수면 아직 남았다.
pub fn days_overdue(due: NaiveDate, today: NaiveDate) -> i64 {
    (today - due).num_days()
}

const WEEKDAY_KO: [&str; 7] = ["월", "화", "수", "목", "금", "토", "일"];

/// 화면·내보내기 표기. 저장은 언제나 ISO다.
pub fn format_korean(d: NaiveDate) -> String {
    format!(
        "{}.{:02}.{:02}.({})",
        d.year(),
        d.month(),
        d.day(),
        WEEKDAY_KO[d.weekday().num_days_from_monday() as usize]
    )
}
