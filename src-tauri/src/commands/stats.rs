//! 통계 — 한도 집계.
//!
//! **세는 것은 태그로 센다.** 체험학습은 `출석인정 결석`으로도 `출석인정 조퇴`로도
//! 나가므로 구분 × 종류 조합으로 세면 빠뜨린다. 반대로 `출석인정 조퇴`를 전부 세면
//! 교외 대회와 병원까지 섞인다. 그래서 규정(`quota_rule`)이 무엇을 셀지는 **태그**가
//! 결정하고, 구분과 종류는 **선택적 추가 조건**이다.
//!
//! **이 화면은 판정하지 않는다.** 한도를 넘겼다는 이유로 입력을 막지도, 기록을
//! 고치지도 않는다. 세어서 보여줄 뿐이다. 그래서 이 모듈은 읽기만 하고
//! 트랜잭션을 열지 않는다.
//!
//! 태그가 빠진 구간은 조용히 넘기지 않고 `QuotaReport.untagged`에 모아 돌려준다.
//! 넘기면 학년말에야 발견된다.
//!
//! **집계 창 밖도 같은 규칙을 받는다.** 한도는 학년도 창으로 세지만(연 20일의 '연'이
//! 학년도다) 학년도는 기준 연도일 뿐 날짜 울타리가 아니라, 그 창 밖에 기록된 것이
//! 실제로 있다. 세지 못한 그 건수를 `QuotaReport.outside`로 함께 돌려준다.

use crate::commands::attendance::{load_spans_on, max_slot_of};
use crate::commands::class::homeroom_scope;
use crate::commands::with_conn;
use crate::due::{academic_year_of, format_date, parse_date};
use crate::state::DbState;
use crate::types::*;
use chrono::{Datelike, Duration, Local, NaiveDate};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension, ToSql};
use std::collections::{BTreeSet, HashMap};
use tauri::State;

/// `near` 판정 기준 — 쓴 값이 한도의 80% **이상**이면 near다.
/// 20일 한도라면 16일부터, 5일 한도라면 4일부터. 정수 곱으로만 비교해
/// 부동소수 오차로 경계가 흔들리지 않게 한다.
const NEAR_PERCENT: i64 = 80;

/// 규정이 태그만 지정하고 구분·종류 조건이 없을 때, 태그 누락으로 볼 구간의 구분.
///
/// 시드가 넣는 기본 라벨이다. 태그 없는 **모든** 구간을 담으면 질병 결석까지 섞여
/// 목록이 쓸모없어지므로 여기서 한 번 좁힌다. 학교가 이 라벨을 바꾸면 목록은
/// 비어서 나온다 — 세는 결과에는 영향이 없다.
///
/// **알려진 빈틈**: 시드의 `생리통 월 1회`처럼 태그만 지정한 규정 중 실제 기록이
/// 질병 조퇴로 남는 것은, 태그가 빠져도 이 목록에 뜨지 않는다. 스키마에 "출석인정"을
/// 가리키는 표시가 없어 라벨 문자열 말고는 좁힐 방법이 없기 때문이다. 제대로 고치려면
/// 규정이 태그 누락 후보 조건을 직접 들고 있거나(`quota_rule`에 열 추가),
/// `attendance_reason`이 그 뜻을 표시해야 한다. 둘 다 스키마 변경이다.
const RECOGNIZED_REASON: &str = "출석인정";

// ── 기간과 단위 ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Period {
    Year,
    Month,
}

impl Period {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "year" => Ok(Period::Year),
            "month" => Ok(Period::Month),
            other => Err(format!("알 수 없는 한도 기간입니다: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    /// 날짜 단위. 같은 학생이 하루에 두 건이어도 1일이다.
    Day,
    /// 건수 단위. 구간 하나가 1이다.
    Count,
}

impl Unit {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "day" => Ok(Unit::Day),
            "count" => Ok(Unit::Count),
            other => Err(format!("알 수 없는 한도 단위입니다: {other}")),
        }
    }
}

/// 세는 칸 하나. 기간 단위에 따라 학년도 하나 · 학기 둘 · 달 열둘이 된다.
struct Bucket {
    key: String,
    label: String,
}

// ── 날짜 범위 ─────────────────────────────────────────────────

fn month_start(year: i32, month: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, 1).expect("달의 첫날은 언제나 존재한다")
}

fn next_month_start(d: NaiveDate) -> NaiveDate {
    if d.month() == 12 {
        month_start(d.year() + 1, 1)
    } else {
        month_start(d.year(), d.month() + 1)
    }
}

/// 학년도 전체 범위. 3월 1일 ~ 이듬해 2월 말이다.
///
/// `academic_year` 행의 `starts_on`/`ends_on`을 쓰지 않는 이유는, 그 두 값이 비어
/// 있을 수 있고 윤년의 2월 말을 정확히 담고 있다는 보장도 없기 때문이다.
/// 집계 범위는 `due::academic_year_of`와 같은 규칙으로 계산한다.
fn academic_year_range(year: i64) -> Result<(NaiveDate, NaiveDate), String> {
    let y = i32::try_from(year).map_err(|_| format!("학년도 값이 올바르지 않습니다: {year}"))?;
    let start = NaiveDate::from_ymd_opt(y, 3, 1)
        .ok_or_else(|| format!("학년도 값이 올바르지 않습니다: {year}"))?;
    let next = NaiveDate::from_ymd_opt(y + 1, 3, 1)
        .ok_or_else(|| format!("학년도 값이 올바르지 않습니다: {year}"))?;
    Ok((start, next - Duration::days(1)))
}

/// 날짜가 속한 칸의 열쇠. `buckets_for`가 만드는 열쇠와 반드시 같은 규칙이어야 한다.
fn bucket_key(period: Period, d: NaiveDate) -> String {
    match period {
        Period::Year => academic_year_of(d).to_string(),
        Period::Month => format!("{:04}-{:02}", d.year(), d.month()),
    }
}

/// 집계 구간 안에 놓이는 칸 목록. 달 단위면 3월부터 2월 순서로 나온다.
fn buckets_for(period: Period, from: NaiveDate, to: NaiveDate) -> Vec<Bucket> {
    if from > to {
        return Vec::new();
    }
    match period {
        Period::Year => {
            let ay = academic_year_of(from);
            vec![Bucket {
                key: ay.to_string(),
                label: format!("{ay}학년도"),
            }]
        }
        Period::Month => {
            let mut out = Vec::new();
            let mut cursor = month_start(from.year(), from.month());
            while cursor <= to {
                out.push(Bucket {
                    key: format!("{:04}-{:02}", cursor.year(), cursor.month()),
                    label: format!("{}월", cursor.month()),
                });
                cursor = next_month_start(cursor);
            }
            out
        }
    }
}

// ── DB 읽기 ───────────────────────────────────────────────────

fn year_of(conn: &Connection, year_id: i64) -> Result<i64, String> {
    conn.query_row("SELECT year FROM academic_year WHERE id = ?1", [year_id], |r| {
        r.get(0)
    })
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("학년도를 찾을 수 없습니다: {year_id}"))
}

struct ClassStudent {
    id: i64,
    /// 그 학생의 학적. 내보내기가 학급의 학년 · 반이 아니라 이것을 쓴다.
    grade: i64,
    class_no: i64,
    number: i64,
    name: String,
}

/// 그 학급의 명단 전부. **명단에서 빠진 학생도 뺀다고 판정하지 않는다** —
/// 빠지기 전에 쓴 날짜가 통계에서 조용히 사라지면 안 되기 때문이다.
/// 그래서 `left_on`도 `enrolled_to`도 보지 않고 한 번이라도 명단이었던 학생을 전부 센다.
fn class_students(conn: &Connection, class_id: i64) -> Result<Vec<ClassStudent>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT st.id, st.grade, st.class_no, st.number, st.name
               FROM class_member m JOIN student st ON st.id = m.student_id
              WHERE m.class_id = ?1
              ORDER BY st.number, st.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![class_id], |r| {
            Ok(ClassStudent {
                id: r.get(0)?,
                grade: r.get(1)?,
                class_no: r.get(2)?,
                number: r.get(3)?,
                name: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 마감되지 않은 한도 규정. `valid_to`가 채워진 규정은 지난 규정이라 세지 않는다.
fn load_rules(
    conn: &Connection,
    school_id: i64,
    rule_id: Option<i64>,
) -> Result<Vec<QuotaRuleItem>, String> {
    let mut sql = String::from(
        "SELECT q.id, q.name, q.tag_id, tg.name, q.reason_id, q.type_id,
                q.period, q.limit_n, q.unit, q.sort_order, q.valid_from, q.valid_to
         FROM quota_rule q
         LEFT JOIN span_tag tg ON tg.id = q.tag_id
         WHERE q.school_id = ? AND q.valid_to IS NULL",
    );
    let mut args: Vec<Value> = vec![Value::Integer(school_id)];
    if let Some(id) = rule_id {
        sql.push_str(" AND q.id = ?");
        args.push(Value::Integer(id));
    }
    sql.push_str(" ORDER BY q.sort_order, q.id");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params_from_iter(args.iter()), |r| {
            Ok(QuotaRuleItem {
                id: r.get(0)?,
                name: r.get(1)?,
                tag_id: r.get(2)?,
                tag_name: r.get(3)?,
                reason_id: r.get(4)?,
                type_id: r.get(5)?,
                period: r.get(6)?,
                limit_n: r.get(7)?,
                unit: r.get(8)?,
                sort_order: r.get(9)?,
                valid_from: r.get(10)?,
                valid_to: r.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // 지정한 규정이 없거나 이미 마감됐다면 빈 목록으로 넘기지 않고 알린다.
    if rows.is_empty() {
        if let Some(id) = rule_id {
            return Err(format!("한도 규정을 찾을 수 없습니다: {id}"));
        }
    }
    Ok(rows)
}

/// 규정이 세는 대상을 좁히는 조건. 별칭은 `sp`이고 날짜 조건은 부르는 쪽이 붙인다.
///
/// 태그가 있으면 그 태그가 붙은 것만, 구분·종류가 채워져 있으면 AND로 더 좁힌다.
/// 셋 다 비어 있으면 그 학급의 모든 구간이다. **창 안을 세는 질의와 창 밖을 세는
/// 질의가 같은 조건을 써야 한다** — 한쪽만 고치면 "세지 않은 건수"가 틀린 값이 된다.
fn rule_narrowing(rule: &QuotaRuleItem) -> (String, Vec<Value>) {
    let mut sql = String::new();
    let mut args: Vec<Value> = Vec::new();
    if let Some(tag) = rule.tag_id {
        sql.push_str(" AND sp.tag_id = ?");
        args.push(Value::Integer(tag));
    }
    if let Some(reason) = rule.reason_id {
        sql.push_str(" AND sp.reason_id = ?");
        args.push(Value::Integer(reason));
    }
    if let Some(type_id) = rule.type_id {
        sql.push_str(" AND sp.type_id = ?");
        args.push(Value::Integer(type_id));
    }
    (sql, args)
}

/// 규정이 세는 구간.
fn counted_spans(
    conn: &Connection,
    class_id: i64,
    from: &str,
    to: &str,
    rule: &QuotaRuleItem,
) -> Result<Vec<(i64, String)>, String> {
    let (narrowing, extra) = rule_narrowing(rule);
    let mut sql = String::from(
        "SELECT sp.student_id, sp.date
         FROM absence_span sp
         WHERE sp.class_id = ?
           AND sp.date >= ? AND sp.date <= ?",
    );
    let mut args: Vec<Value> = vec![
        Value::Integer(class_id),
        Value::Text(from.to_string()),
        Value::Text(to.to_string()),
    ];
    sql.push_str(&narrowing);
    args.extend(extra);
    sql.push_str(" ORDER BY sp.date, sp.id");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params_from_iter(args.iter()), |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn recognized_reason_id(conn: &Connection) -> Result<Option<i64>, String> {
    conn.query_row(
        "SELECT id FROM attendance_reason WHERE label = ?1 AND valid_to IS NULL",
        [RECOGNIZED_REASON],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

// ── 태그가 빠진 구간 ──────────────────────────────────────────

/// 태그가 비어 있어 세지 못한 구간.
///
/// 규정이 태그를 지정한 경우에만 뜻이 있다. 규정에 구분·종류 조건이 있으면 그 조건에
/// 맞으면서 태그만 빠진 구간을, 조건이 없으면 출석인정 구간 중 태그 없는 것을 모은다.
///
/// 목록을 읽는 일은 `attendance::load_spans_on`이 한다. 같은 질의를 여기서 다시
/// 만들면 구간 표기(`span_text`)와 겹침 판정이 화면마다 달라진다 — 같은 기록이
/// 출결 기록에서는 `조회부터 3교시까지`, 통계에서는 다른 문장으로 보인다.
/// 별칭은 그 헬퍼가 정한 것을 그대로 쓴다. 구간이 `s`, 학생이 `st`다.
/// 태그 누락 후보를 좁히는 두 축.
struct UntaggedFilter {
    reason_id: Option<i64>,
    type_id: Option<i64>,
}

/// 태그 누락 후보를 좁히는 조건. `None`이면 그 개념이 없는 규정이다.
///
/// 태그를 지정하지 않은 규정은 모든 구간을 세므로 "빠진 태그"라는 개념이 없다.
/// 구분·종류 조건이 없는 규정은 좁힐 기준이 없어 기본 구분 라벨로 좁힌다 — 그 라벨이
/// 없는 DB라면 태그 없는 **모든** 구간이 남아 질병 결석까지 섞이므로 `None`이다.
fn untagged_narrowing(
    conn: &Connection,
    rule: &QuotaRuleItem,
) -> Result<Option<UntaggedFilter>, String> {
    if rule.tag_id.is_none() {
        return Ok(None);
    }
    if rule.reason_id.is_none() && rule.type_id.is_none() {
        return Ok(recognized_reason_id(conn)?.map(|id| UntaggedFilter {
            reason_id: Some(id),
            type_id: None,
        }));
    }
    Ok(Some(UntaggedFilter {
        reason_id: rule.reason_id,
        type_id: rule.type_id,
    }))
}

fn untagged_spans(
    conn: &Connection,
    class_id: i64,
    from: &str,
    to: &str,
    rule: &QuotaRuleItem,
    today: NaiveDate,
) -> Result<Vec<SpanItem>, String> {
    let Some(UntaggedFilter { reason_id, type_id }) = untagged_narrowing(conn, rule)? else {
        return Ok(Vec::new());
    };

    let mut where_sql = String::from(
        "WHERE s.class_id = ?1
           AND s.date >= ?2 AND s.date <= ?3
           AND s.tag_id IS NULL",
    );
    let mut params: Vec<&dyn ToSql> = vec![&class_id, &from, &to];

    let mut next = 4;
    if reason_id.is_some() {
        where_sql.push_str(&format!(" AND s.reason_id = ?{next}"));
        params.push(&reason_id);
        next += 1;
    }
    if type_id.is_some() {
        where_sql.push_str(&format!(" AND s.type_id = ?{next}"));
        params.push(&type_id);
    }
    where_sql.push_str(" ORDER BY s.date, st.number, s.id");

    load_spans_on(conn, &where_sql, &params, today)
}

// ── 집계 창 밖 ────────────────────────────────────────────────

/// 집계 창 **밖**의 구간을 센다. 조건은 부르는 쪽이 준다.
fn count_outside(
    conn: &Connection,
    class_id: i64,
    from: &str,
    to: &str,
    narrowing: &str,
    extra: Vec<Value>,
) -> Result<i64, String> {
    let sql = format!(
        "SELECT COUNT(*) FROM absence_span sp
          WHERE sp.class_id = ?
            AND (sp.date < ? OR sp.date > ?){narrowing}"
    );
    let mut args: Vec<Value> = vec![
        Value::Integer(class_id),
        Value::Text(from.to_string()),
        Value::Text(to.to_string()),
    ];
    args.extend(extra);
    conn.query_row(&sql, params_from_iter(args.iter()), |r| r.get(0))
        .map_err(|e| e.to_string())
}

/// 이 규정이 봤어야 하지만 집계 창 밖이라 세지 못한 구간 수.
///
/// **한도를 학년도 창으로 세는 것 자체는 유지한다** — 연 20일의 '연'이 학년도다.
/// 다만 학년도는 기준 연도일 뿐 날짜 울타리가 아니라, 2026학년도 학급에 2027-03-05를
/// 입력하는 일이 실제로 있다. 그 건은 한도에서도 태그 누락 목록에서도 빠지는데, 세어서
/// 알리지 않으면 교사에게 "덜 썼다"고 거짓으로 말하게 된다.
///
/// 세는 것과 태그 누락 후보는 서로 겹치지 않는다 — 앞은 규정의 태그가 붙은 것이고
/// 뒤는 태그가 비어 있는 것이라, 두 수를 그대로 더하면 된다.
fn outside_window_count(
    conn: &Connection,
    class_id: i64,
    from: &str,
    to: &str,
    rule: &QuotaRuleItem,
) -> Result<i64, String> {
    let (narrowing, extra) = rule_narrowing(rule);
    let mut total = count_outside(conn, class_id, from, to, &narrowing, extra)?;

    if let Some(UntaggedFilter { reason_id, type_id }) = untagged_narrowing(conn, rule)? {
        let mut sql = String::from(" AND sp.tag_id IS NULL");
        let mut args: Vec<Value> = Vec::new();
        if let Some(reason) = reason_id {
            sql.push_str(" AND sp.reason_id = ?");
            args.push(Value::Integer(reason));
        }
        if let Some(kind) = type_id {
            sql.push_str(" AND sp.type_id = ?");
            args.push(Value::Integer(kind));
        }
        total += count_outside(conn, class_id, from, to, &sql, args)?;
    }
    Ok(total)
}

// ── 집계 ──────────────────────────────────────────────────────

/// `used`를 한도와 비교하면 어느 상태인가.
///
///   over — 한도에 **도달했거나** 넘었다(`used >= limit_n`). 20일 한도에서 20일째면
///          더 쓸 수 없으므로 도달을 넘김으로 본다.
///   near — over가 아니면서 한도의 80% 이상.
fn state_of(used: i64, limit_n: i64) -> String {
    if used >= limit_n {
        "over".to_string()
    } else if used * 100 >= limit_n * NEAR_PERCENT {
        "near".to_string()
    } else {
        "ok".to_string()
    }
}

pub fn get_quota_reports_impl(
    conn: &Connection,
    class_id: i64,
    rule_id: Option<i64>,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<QuotaReport>, String> {
    // 규정과 학년도는 학급에서 얻는다. 학교가 없으면 여기서 걸린다 — 최대 교시를
    // 여기서 들고 있지 않는 이유는, 구간을 읽는 헬퍼가 각 행의 학생이 속한 학교에서
    // 그 값을 읽기 때문이다.
    let scope = homeroom_scope(conn, class_id)?;
    let school_id = scope.school_id;
    max_slot_of(conn, school_id)?;
    let (year_from, year_to) = academic_year_range(year_of(conn, scope.year_id)?)?;

    // 집계 구간은 학년도 범위와 교사가 고른 구간의 교집합이다. 학년도 밖을 세지 않는다.
    let win_from = match from {
        Some(v) => parse_date(v)?.max(year_from),
        None => year_from,
    };
    let win_to = match to {
        Some(v) => parse_date(v)?.min(year_to),
        None => year_to,
    };
    let win_from_text = format_date(win_from);
    let win_to_text = format_date(win_to);

    let students = class_students(conn, scope.id)?;
    let rules = load_rules(conn, school_id, rule_id)?;
    // 오늘 날짜는 태그 누락 목록의 서류 경과일 표시에만 쓴다.
    let today = Local::now().date_naive();

    let mut out = Vec::with_capacity(rules.len());
    for rule in rules {
        let period = Period::parse(&rule.period)?;
        let unit = Unit::parse(&rule.unit)?;
        let buckets = buckets_for(period, win_from, win_to);

        let hits = counted_spans(conn, scope.id, &win_from_text, &win_to_text, &rule)?;

        // 날짜 단위는 (학생, 칸)마다 날짜 집합을, 건수 단위는 건수를 센다.
        let mut day_hits: HashMap<(i64, String), BTreeSet<String>> = HashMap::new();
        let mut count_hits: HashMap<(i64, String), i64> = HashMap::new();
        let mut all_dates: HashMap<i64, BTreeSet<String>> = HashMap::new();

        for (student_id, date) in hits {
            let d = parse_date(&date)?;
            let key = bucket_key(period, d);
            match unit {
                Unit::Day => {
                    day_hits
                        .entry((student_id, key))
                        .or_default()
                        .insert(date.clone());
                }
                Unit::Count => {
                    *count_hits.entry((student_id, key)).or_insert(0) += 1;
                }
            }
            all_dates.entry(student_id).or_default().insert(date);
        }

        let count_in = |student_id: i64, key: &str| -> i64 {
            match unit {
                Unit::Day => day_hits
                    .get(&(student_id, key.to_string()))
                    .map_or(0, |set| set.len() as i64),
                Unit::Count => count_hits
                    .get(&(student_id, key.to_string()))
                    .copied()
                    .unwrap_or(0),
            }
        };

        let mut rows = Vec::with_capacity(students.len());
        let mut near_count = 0;
        let mut over_count = 0;
        let mut used_total = 0;

        for s in &students {
            // 기간이 학년도 하나면 칸도 하나다. 학기·달이면 **가장 많이 쓴 칸**이
            // 그 학생의 used다 — 어느 달이든 한도를 넘겼으면 넘긴 것이다.
            let used = buckets
                .iter()
                .map(|b| count_in(s.id, &b.key))
                .max()
                .unwrap_or(0);

            let dates = match unit {
                Unit::Day => all_dates
                    .get(&s.id)
                    .map(|set| set.iter().cloned().collect())
                    .unwrap_or_default(),
                // 건수 단위에서는 날짜 칸을 쓰지 않는다.
                Unit::Count => Vec::new(),
            };

            let row_buckets = if period == Period::Month {
                buckets
                    .iter()
                    .map(|b| QuotaBucket {
                        key: b.key.clone(),
                        label: b.label.clone(),
                        count: count_in(s.id, &b.key),
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let state = state_of(used, rule.limit_n);
            match state.as_str() {
                "over" => over_count += 1,
                "near" => near_count += 1,
                _ => {}
            }
            used_total += used;

            rows.push(QuotaRow {
                student_id: s.id,
                grade: s.grade,
                class_no: s.class_no,
                number: s.number,
                name: s.name.clone(),
                used,
                limit_n: rule.limit_n,
                dates,
                buckets: row_buckets,
                state,
            });
        }

        let untagged = untagged_spans(conn, scope.id, &win_from_text, &win_to_text, &rule, today)?;
        // 창 밖은 조용히 넘기지 않는다. 화면이 이 수로 "세지 않은 것이 있다"를 알린다.
        let outside =
            outside_window_count(conn, scope.id, &win_from_text, &win_to_text, &rule)?;

        out.push(QuotaReport {
            rule,
            rows,
            used_total,
            near_count,
            over_count,
            untagged,
            window_from: win_from_text.clone(),
            window_to: win_to_text.clone(),
            outside,
        });
    }

    Ok(out)
}

// ── 커맨드 ────────────────────────────────────────────────────

#[tauri::command]
pub fn get_quota_reports(
    db: State<DbState>,
    class_id: i64,
    rule_id: Option<i64>,
    from: Option<String>,
    to: Option<String>,
) -> Result<Vec<QuotaReport>, String> {
    with_conn(&db, |conn| {
        get_quota_reports_impl(conn, class_id, rule_id, from.as_deref(), to.as_deref())
    })
}
