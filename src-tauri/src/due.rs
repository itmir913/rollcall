//! 마감일 계산과 날짜 유틸. 순수 함수다. DB를 모른다.
//!
//! 제출 기한은 학교 설정의 두 값으로 결정된다 — `due_days`(며칠)와
//! `due_skip_offdays`(주말·휴업일을 셀 것인가).
//!
//! **공휴일 API를 부르지 않는다.** 앱은 서버를 쓰지 않고, 개교기념일·재량휴업일은
//! 어차피 외부 달력에 없다. 대신 교사가 설정에서 휴업일을 직접 등록하고, 그 목록이
//! 여기로 넘어온다. 학사일정 테이블이 아니라 **마감 계산에서 건너뛸 날짜 목록**이다.
//!
//! 계산된 마감일은 `absence_span.doc_due`에 박아 둔다. 설정을 바꿔도 과거 기록의
//! 마감이 소급 변경되지 않아야 하고, 교사가 개별로 고칠 수 있어야 하기 때문이다.

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use std::collections::HashSet;

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

/// 그날을 세지 않는가. 주말이거나 등록된 휴업일이면 건너뛴다.
pub fn is_off_day(d: NaiveDate, off_days: &HashSet<NaiveDate>) -> bool {
    is_weekend(d) || off_days.contains(&d)
}

/// 기준일로부터 마감일.
///
/// `skip_off_days`가 true면 주말과 휴업일을 세지 않는다.
/// `due_days == 0`이면 기준일이 곧 마감일이다 — 주말이어도 옮기지 않는다.
/// 교사가 설정한 값을 프로그램이 조정하지 않는다.
pub fn due_date(
    base: NaiveDate,
    due_days: i64,
    skip_off_days: bool,
    off_days: &HashSet<NaiveDate>,
) -> NaiveDate {
    if !skip_off_days {
        return base + Duration::days(due_days);
    }
    let mut remaining = due_days;
    let mut cursor = base;
    // 무한 루프 방지 — 휴업일을 아무리 많이 등록해도 1년 안에서 끝낸다.
    let limit = base + Duration::days(366);
    while remaining > 0 && cursor < limit {
        cursor += Duration::days(1);
        if !is_off_day(cursor, off_days) {
            remaining -= 1;
        }
    }
    cursor
}

/// 기간 안에서 셀 수 있는 날 목록. 여러 날 일괄 입력의 기본 후보다.
///
/// 주말과 등록된 휴업일을 뺀다. 등록되지 않은 휴업일은 미리보기에서 교사가 지운다 —
/// 학사일정을 앱이 알 수 없다는 사실의 인정이다.
pub fn open_days_between(
    from: NaiveDate,
    to: NaiveDate,
    off_days: &HashSet<NaiveDate>,
) -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let mut cursor = from;
    while cursor <= to {
        if !is_off_day(cursor, off_days) {
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

/// 그 날짜가 속한 학년도. 1·2월은 전년도 학년도에 속한다.
pub fn academic_year_of(d: NaiveDate) -> i32 {
    if d.month() >= 3 {
        d.year()
    } else {
        d.year() - 1
    }
}
