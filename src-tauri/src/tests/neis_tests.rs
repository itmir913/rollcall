//! 나이스 가져오기 — 차분과 적용.
//!
//! 학생 이름은 전부 가짜다. 실제 파일의 이름은 개인정보라 저장소에 남기지 않는다.
//!
//! 여기서 지키려는 것은 셋이다.
//!   · 같은 것은 그대로 두고, 없는 것은 추가하고, 다른 것은 교사가 고른다.
//!   · **앱에만 있는 기록을 지우지 않는다.** 나이스에 아직 안 넣은 것이 이 앱의 존재 이유다.
//!   · 고르지 않은 것은 손대지 않는다.

use super::*;
use crate::commands::attendance::stamp_span_impl;
use crate::commands::neis::{apply_neis_import_impl, preview_neis_import_impl};
use crate::types::{NeisImportChoice, NeisRowInput, StampInput};
use rusqlite::Connection;

const TODAY: &str = "2026-09-11";
const DATE: &str = "2026-09-01";

/// 담임 학급 하나를 갖춘 메모리 DB. 범위가 `classId` 하나이므로 시험도 그것부터 만든다.
fn fixture() -> (Connection, i64, i64) {
    let conn = setup_test_db();
    let school = school_id(&conn);
    let class = homeroom(&conn, school);
    (conn, school, class)
}

fn row(number: i64, code: &str, reason: &str, kind: &str, start: Option<&str>, end: Option<&str>)
    -> NeisRowInput {
    NeisRowInput {
        number,
        date: DATE.to_string(),
        code_label: Some(code.to_string()),
        reason_label: Some(reason.to_string()),
        type_label: Some(kind.to_string()),
        start_slot: start.map(str::to_string),
        end_slot: end.map(str::to_string),
        detail: Some("몸살".to_string()),
    }
}

/// 질병 결석 하루 종일. 나이스가 `조회,1교시,…,종례,`로 주는 것과 같은 구간이다.
fn sick_absence(number: i64) -> NeisRowInput {
    row(number, "질병결석", "질병", "결석", Some("조회"), Some("종례"))
}

/// 같은 줄을 다른 날짜로. 나이스 파일 하나에 여러 날이 섞여 오는 것이 보통이다.
fn on_date(mut row: NeisRowInput, date: &str) -> NeisRowInput {
    row.date = date.to_string();
    row
}

#[allow(clippy::too_many_arguments)]
fn stamp_on(
    conn: &Connection,
    class_id: i64,
    student_id: i64,
    date: &str,
    reason: &str,
    kind: &str,
    slots: &[&str],
) {
    let (reason_id, type_id) = axes(conn, reason, kind);
    stamp_span_impl(
        conn,
        &StampInput {
            class_id,
            student_id,
            date: date.to_string(),
            reason_id,
            type_id,
            slots: slots.iter().map(|s| s.to_string()).collect(),
        },
    )
    .unwrap();
}

fn stamp(conn: &Connection, class_id: i64, student_id: i64, reason: &str, kind: &str, slots: &[&str]) {
    stamp_on(conn, class_id, student_id, DATE, reason, kind, slots);
}

/// 시드의 구분 × 종류 코드 하나. 별칭표가 이 행을 가리킨다.
fn code_id(conn: &Connection, reason: &str, kind: &str) -> i64 {
    conn.query_row(
        "SELECT c.id FROM attendance_code c
           JOIN attendance_reason r ON r.id = c.reason_id
           JOIN attendance_type t ON t.id = c.type_id
          WHERE r.label = ?1 AND t.label = ?2",
        rusqlite::params![reason, kind],
        |r| r.get(0),
    )
    .unwrap()
}

fn nothing() -> NeisImportChoice {
    NeisImportChoice {
        add: vec![],
        replace: vec![],
        mark_neis: false,
    }
}

#[test]
fn 앱에_없는_기록은_추가로_잡힌다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");

    let rows = vec![sick_absence(5)];
    let out = preview_neis_import_impl(&conn, class, &rows, TODAY).unwrap();

    assert_eq!(out.add, 1);
    assert_eq!(out.same, 0);
    assert_eq!(out.items[0].verdict, "add");
    assert_eq!(out.items[0].their_axis, "질병 결석");
    assert_eq!(out.items[0].their_span, "하루 종일");
}

#[test]
fn 똑같은_기록은_같음이고_손대지_않는다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    stamp(&conn, class, student, "질병", "결석", &[]);

    let rows = vec![sick_absence(5)];
    let out = preview_neis_import_impl(&conn, class, &rows, TODAY).unwrap();

    assert_eq!(out.same, 1);
    assert_eq!(out.add, 0);
    assert_eq!(out.only_mine, 0);
}

#[test]
fn 내용이_다르면_다름이고_교사가_고른다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    stamp(&conn, class, student, "미인정", "결석", &[]);

    let rows = vec![sick_absence(5)];
    let out = preview_neis_import_impl(&conn, class, &rows, TODAY).unwrap();

    assert_eq!(out.differ, 1);
    let item = &out.items[0];
    assert_eq!(item.my_axis.as_deref(), Some("미인정 결석"));
    assert_eq!(item.their_axis, "질병 결석");
    assert!(item.span_id.is_some(), "고칠 대상을 알려줘야 고를 수 있다");
}

#[test]
fn 고르지_않으면_아무것도_바뀌지_않는다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    stamp(&conn, class, student, "미인정", "결석", &[]);

    let rows = vec![sick_absence(5)];
    let out = apply_neis_import_impl(&conn, class, &rows, &nothing(), TODAY).unwrap();

    assert_eq!((out.added, out.replaced, out.marked), (0, 0, 0));
    let axis: String = conn
        .query_row(
            "SELECT r.label FROM absence_span s JOIN attendance_reason r ON r.id = s.reason_id",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(axis, "미인정", "고르지 않은 것을 바꾸지 않는다");
}

#[test]
fn 고른_것만_추가한다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");
    enroll(&conn, class, school, 6, "학생6");

    let rows = vec![sick_absence(5), sick_absence(6)];
    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    let out = apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();

    assert_eq!(out.added, 1);
    // **어느 학생인지까지 본다.** 건수만 세면 엉뚱한 학생에게 넣어도 통과한다.
    let number: i64 = conn
        .query_row(
            "SELECT st.number FROM absence_span s JOIN student st ON st.id = s.student_id",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(number, 5);
}

#[test]
fn 가져온_건은_나이스_등재로_들어간다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");

    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();

    let (done, on): (bool, Option<String>) = conn
        .query_row("SELECT neis_done, neis_done_on FROM absence_span", [], |r| {
            Ok((r.get::<_, i64>(0)? != 0, r.get(1)?))
        })
        .unwrap();
    assert!(done, "나이스 파일에서 왔으므로 이미 등재된 건이다");
    assert_eq!(on.as_deref(), Some(TODAY));
}

#[test]
fn 나이스에_있는_것으로_확인되면_등재_표시를_해준다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    stamp(&conn, class, student, "질병", "결석", &[]);

    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![],
        replace: vec![],
        mark_neis: true,
    };
    let out = apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();

    assert_eq!(out.marked, 1);
    let done: bool = conn
        .query_row("SELECT neis_done FROM absence_span", [], |r| {
            Ok(r.get::<_, i64>(0)? != 0)
        })
        .unwrap();
    assert!(done);
}

#[test]
fn 다름을_고르면_내_기록이_나이스_쪽으로_바뀐다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    stamp(&conn, class, student, "미인정", "결석", &[]);

    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![],
        replace: vec![0],
        mark_neis: false,
    };
    let out = apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();

    assert_eq!(out.replaced, 1);
    let (axis, memo): (String, String) = conn
        .query_row(
            "SELECT r.label, s.memo FROM absence_span s
               JOIN attendance_reason r ON r.id = s.reason_id",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(axis, "질병");
    assert_eq!(memo, "몸살", "비어 있던 메모는 나이스의 사유로 채운다");
}

#[test]
fn 교사가_쓴_메모를_덮지_않는다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    stamp(&conn, class, student, "미인정", "결석", &[]);
    conn.execute("UPDATE absence_span SET memo = '학부모 통화함'", [])
        .unwrap();

    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![],
        replace: vec![0],
        mark_neis: false,
    };
    apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();

    let memo: String = conn
        .query_row("SELECT memo FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(memo, "학부모 통화함");
}

#[test]
fn 앱에만_있는_기록은_세기만_하고_지우지_않는다() {
    let (conn, school, class) = fixture();
    let student5 = enroll(&conn, class, school, 5, "학생5");
    let student6 = enroll(&conn, class, school, 6, "학생6");
    stamp(&conn, class, student5, "질병", "결석", &[]);
    stamp(&conn, class, student6, "질병", "조퇴", &["3"]);

    // 파일에는 5번만 있다.
    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![],
        replace: vec![],
        mark_neis: true,
    };
    let out = preview_neis_import_impl(&conn, class, &rows, TODAY).unwrap();
    assert_eq!(out.only_mine, 1, "나이스에 아직 안 넣은 것이 하나 있다");

    apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "파일에 없다는 것이 삭제 근거가 될 수 없다");
}

#[test]
fn 하루_두_구간은_여럿을_여럿과_비교한다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    // 1교시까지 지각 + 6교시부터 조퇴. 나이스 실파일에 있던 하루 2구간이다.
    stamp(&conn, class, student, "질병", "지각", &["1"]);
    stamp(&conn, class, student, "질병", "조퇴", &["6"]);

    // **파일 순서를 뒤집어 둔다.** 저장 순서와 같으면 순서대로만 연결하는 짝짓기로도
    // 통과해 버려서, 이 테스트가 주장하는 것을 실제로는 확인하지 못한다.
    let rows = vec![
        row(5, "질병조퇴", "질병", "조퇴", Some("6"), Some("종례")),
        row(5, "질병지각", "질병", "지각", Some("조회"), Some("1")),
    ];
    let out = preview_neis_import_impl(&conn, class, &rows, TODAY).unwrap();

    assert_eq!(out.same, 2, "하나씩 비교하면 둘 다 다름으로 잡힌다");
    assert_eq!(out.differ, 0);
    assert_eq!(out.only_mine, 0);
}

#[test]
fn 결시교시가_조회_하나뿐인_지각도_들어간다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 18, "학생18");

    // 나이스 실파일의 `질병지각 · 결시교시 조회,`. 화면의 교시 고르개는 지각에서
    // 조회를 열지 않지만, 파일에서 온 값은 그대로 저장한다 — 우리가 못 고르는 것과
    // 나이스에 그렇게 적혀 있는 것은 다른 문제다.
    let rows = vec![row(18, "질병지각", "질병", "지각", Some("조회"), Some("조회"))];
    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();

    let (start, end): (String, String) = conn
        .query_row("SELECT start_slot, end_slot FROM absence_span", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!((start.as_str(), end.as_str()), ("조회", "조회"));
}

#[test]
fn 명렬표에_없는_번호는_조용히_넘기지_않는다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");

    let rows = vec![sick_absence(99)];
    let out = preview_neis_import_impl(&conn, class, &rows, TODAY).unwrap();

    assert_eq!(out.unreadable, 1);
    assert_eq!(out.items[0].verdict, "unreadable");
    assert!(out.items[0].why.as_deref().unwrap().contains("명렬표"));
}

#[test]
fn 모르는_출결_표기는_지어내지_않는다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");

    let mut bad = sick_absence(5);
    bad.code_label = Some("공결".to_string());
    bad.reason_label = None;
    bad.type_label = None;

    let out = preview_neis_import_impl(&conn, class, &[bad], TODAY).unwrap();
    assert_eq!(out.unreadable, 1);
    assert!(out.items[0].why.as_deref().unwrap().contains("공결"));
}

#[test]
fn 학교마다_다른_표기는_별칭표가_흡수한다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");
    conn.execute(
        "INSERT INTO code_alias (code_id, raw) VALUES (?1, '인정결석')",
        rusqlite::params![code_id(&conn, "출석인정", "결석")],
    )
    .unwrap();

    let mut aliased = sick_absence(5);
    aliased.code_label = Some("인정결석".to_string());
    aliased.reason_label = None;
    aliased.type_label = None;

    let out = preview_neis_import_impl(&conn, class, &[aliased], TODAY).unwrap();
    assert_eq!(out.unreadable, 0);
    assert_eq!(out.items[0].their_axis, "출석인정 결석");
}

#[test]
fn 다른_반의_기록은_섞이지_않는다() {
    let (conn, school, class) = fixture();
    // **같은 학생을 두 학급에 넣는다.** 다른 학생을 쓰면 학적으로 거르던 옛 질의로도
    // 통과해, 이 테스트가 주장하는 것을 확인하지 못한다.
    let student = enroll(&conn, class, school, 5, "학생5");
    let next_door = insert_class(&conn, school, "homeroom", "3학년 7반", Some(3), Some(7));
    join_class(&conn, next_door, student);
    // 옆 학급에 같은 학생 · 같은 날 기록. 학생으로 거르면 '이미 있음'으로 잡힌다.
    stamp(&conn, next_door, student, "질병", "결석", &[]);

    let out = preview_neis_import_impl(&conn, class, &[sick_absence(5)], TODAY).unwrap();
    assert_eq!(out.add, 1, "이 학급에는 없는 기록이다");
    assert_eq!(out.same, 0, "옆 학급의 같은 학생 기록을 내 것으로 보지 않는다");
    assert_eq!(out.only_mine, 0, "옆 학급 기록을 세지 않는다");
}

#[test]
fn 마감은_가져올_때_계산해_박는다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");

    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    apply_neis_import_impl(&conn, class, &[sick_absence(5)], &choice, TODAY).unwrap();

    let due: Option<String> = conn
        .query_row("SELECT doc_due FROM absence_span", [], |r| r.get(0))
        .unwrap();
    // 시드는 7일 · 주말 제외다. 2026-09-01(화)에서 수업일로 7일 뒤.
    assert_eq!(due.as_deref(), Some("2026-09-10"));
}

#[test]
fn 못_읽은_줄_하나가_나머지를_막지_않는다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");

    // 5번은 읽히고 99번은 명렬표에 없다. 교사는 읽힌 것만 골랐다.
    let rows = vec![sick_absence(5), sick_absence(99)];
    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    let out = apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();

    assert_eq!(out.added, 1, "모르는 줄 하나 때문에 전체가 막히면 안 된다");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn 하루_두_구간이_어긋나면_같은_종류끼리_짝짓는다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    // 조퇴를 먼저 입력해 id가 작다. 순서대로만 연결하면 조퇴에 지각이 붙는다.
    stamp(&conn, class, student, "질병", "조퇴", &["6"]);
    stamp(&conn, class, student, "질병", "지각", &["1"]);
    conn.execute("UPDATE absence_span SET memo = '병원' WHERE start_slot = '6'", [])
        .unwrap();

    // 파일 쪽은 두 구간 모두 기간이 조금씩 다르다 — 둘 다 '다름'으로 남는다.
    let rows = vec![
        row(5, "질병지각", "질병", "지각", Some("조회"), Some("2")),
        row(5, "질병조퇴", "질병", "조퇴", Some("5"), Some("종례")),
    ];
    let out = preview_neis_import_impl(&conn, class, &rows, TODAY).unwrap();
    assert_eq!(out.differ, 2);

    let late = out.items.iter().find(|i| i.their_axis == "질병 지각").unwrap();
    let early = out.items.iter().find(|i| i.their_axis == "질병 조퇴").unwrap();
    assert_eq!(late.my_axis.as_deref(), Some("질병 지각"));
    assert_eq!(early.my_axis.as_deref(), Some("질병 조퇴"));

    // 짝이 뒤바뀌면 교사가 둘 다 고쳤을 때 메모가 엉뚱한 기록으로 넘어간다.
    let choice = NeisImportChoice {
        add: vec![],
        replace: vec![0, 1],
        mark_neis: false,
    };
    apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();
    let memo: String = conn
        .query_row(
            "SELECT memo FROM absence_span s JOIN attendance_type t ON t.id = s.type_id
              WHERE t.label = '조퇴'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(memo, "병원", "메모는 그 구간에 그대로 남아야 한다");
}

#[test]
fn 최대_교시_밖의_교시는_읽지_못한_줄이다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");
    // 시드의 최대 교시는 7이다. 저장 경로는 전부 validate_span을 지나야 한다.
    let rows = vec![row(5, "질병결과", "질병", "결과", Some("8"), Some("8"))];

    let out = preview_neis_import_impl(&conn, class, &rows, TODAY).unwrap();
    assert_eq!(out.unreadable, 1);
    assert!(out.items[0].why.as_deref().unwrap().contains("8"));
}

#[test]
fn 자리를_채우지_않은_날짜도_같은_날로_본다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    stamp(&conn, class, student, "질병", "결석", &[]);

    // 파일이 `2026-9-1`로 주면, 원본 문자열로 기간을 재는 순간 내 기록을 하나도
    // 못 불러와 이미 있는 건이 "앱에 없음"으로 잡히고 중복이 들어간다.
    let mut loose = sick_absence(5);
    loose.date = "2026-9-1".to_string();

    let out = preview_neis_import_impl(&conn, class, &[loose], TODAY).unwrap();
    assert_eq!(out.same, 1);
    assert_eq!(out.add, 0);
    assert_eq!(out.from, "2026-09-01");
}

#[test]
fn 전출한_학생은_전출일부터_이_반이_아니다() {
    let (conn, school, class) = fixture();
    // 9월 5일에 전출했다. 경계는 `enrolled_from <= 날짜 < enrolled_to`다 —
    // 전출일 당일은 이미 다른 학교 학생이므로 그날 줄은 이 반의 것이 아니다.
    let leaving = insert_student_at(&conn, school, 3, 6, 7, "전출학생");
    join_class(&conn, class, leaving);
    conn.execute(
        "UPDATE student SET enrolled_to = '2026-09-05' WHERE id = ?1",
        rusqlite::params![leaving],
    )
    .unwrap();

    let rows = vec![
        on_date(sick_absence(7), "2026-09-04"),
        on_date(sick_absence(7), "2026-09-05"),
    ];
    let out = preview_neis_import_impl(&conn, class, &rows, TODAY).unwrap();

    assert_eq!(out.add, 1, "전출 전날은 아직 이 반 학생이다");
    assert_eq!(out.items[0].date, "2026-09-04");
    assert_eq!(out.unreadable, 1, "전출일부터는 이 반에 없는 번호다");
    assert!(out.items[1].why.as_deref().unwrap().contains("명렬표"));
}

#[test]
fn 같은_파일을_다시_가져와도_더_들어가지_않는다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");

    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    let first = apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();
    assert_eq!(first.added, 1);

    // 같은 파일을 한 번 더 올린 것. 두 번째는 '같음'이라 더할 것이 없고,
    // 교사가 고른 줄이 적용되지 않았다는 사실만 세어서 알린다.
    let again = apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();
    assert_eq!(again.added, 0);
    assert_eq!(again.skipped, 1);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "두 번 가져왔다고 같은 기록이 둘이 되면 안 된다");
}

#[test]
fn 여러_날짜_파일은_그_기간_안에서만_센다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    // 파일 기간 안에 있지만 파일에는 없는 기록 하나 + 기간 밖의 기록 하나.
    stamp_on(&conn, class, student, "2026-09-02", "질병", "결석", &[]);
    stamp_on(&conn, class, student, "2026-09-20", "질병", "결석", &[]);

    let rows = vec![
        on_date(sick_absence(5), "2026-09-01"),
        on_date(sick_absence(5), "2026-09-03"),
    ];
    let out = preview_neis_import_impl(&conn, class, &rows, TODAY).unwrap();

    assert_eq!(out.from, "2026-09-01");
    assert_eq!(out.to, "2026-09-03");
    assert_eq!(out.add, 2);
    assert_eq!(
        out.only_mine, 1,
        "기간 밖의 기록은 이 파일이 말하는 바가 아니다"
    );
}

#[test]
fn 다름을_고쳐도_태그와_서류와_마감은_그대로다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    stamp(&conn, class, student, "미인정", "결석", &[]);
    // 나이스는 태그도 서류도 마감도 모른다. 가져오기가 이것들을 건드리면 교사가
    // 받아 둔 서류와 학부모에게 말해 둔 날짜가 조용히 사라진다.
    let tag = tag_id(&conn, "체험학습");
    conn.execute(
        "UPDATE absence_span
            SET tag_id = ?1, doc_done = 1, doc_done_on = ?2, doc_due = '2026-09-30'",
        rusqlite::params![tag, TODAY],
    )
    .unwrap();

    let choice = NeisImportChoice {
        add: vec![],
        replace: vec![0],
        mark_neis: false,
    };
    let out =
        apply_neis_import_impl(&conn, class, &[sick_absence(5)], &choice, TODAY)
            .unwrap();
    assert_eq!(out.replaced, 1);

    let (kept, done, due): (Option<i64>, bool, Option<String>) = conn
        .query_row("SELECT tag_id, doc_done, doc_due FROM absence_span", [], |r| {
            Ok((r.get(0)?, r.get::<_, i64>(1)? != 0, r.get(2)?))
        })
        .unwrap();
    assert_eq!(kept, Some(tag), "한도는 태그로 센다. 태그가 날아가면 통계가 틀린다");
    assert!(done, "받아 둔 서류가 가져오기로 없던 일이 되면 안 된다");
    assert_eq!(due.as_deref(), Some("2026-09-30"), "마감은 만들 때 박은 값이다");
}

#[test]
fn 마감된_코드의_별칭은_그_뒤_날짜에_맞지_않는다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");
    // 코드는 마감 후 추가다. 마감한 코드의 별칭이 계속 맞으면 9월 줄이 작년 코드의
    // 두 축으로 들어가고, 그 구간은 그날 살아 있던 코드와 연결되지 않는다.
    let code = code_id(&conn, "출석인정", "결석");
    conn.execute(
        "UPDATE attendance_code SET valid_to = '2026-08-01' WHERE id = ?1",
        rusqlite::params![code],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO code_alias (code_id, raw) VALUES (?1, '인정결석')",
        rusqlite::params![code],
    )
    .unwrap();

    let mut aliased = sick_absence(5);
    aliased.code_label = Some("인정결석".to_string());
    aliased.reason_label = None;
    aliased.type_label = None;

    let out = preview_neis_import_impl(&conn, class, &[aliased], TODAY).unwrap();
    assert_eq!(out.unreadable, 1);
    assert!(out.items[0].why.as_deref().unwrap().contains("인정결석"));
}

#[test]
fn 별칭은_비어_있는_축만_채운다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");
    conn.execute(
        "INSERT INTO code_alias (code_id, raw) VALUES (?1, '인정결석')",
        rusqlite::params![code_id(&conn, "출석인정", "결석")],
    )
    .unwrap();

    // 파일이 구분만 나눠 왔다. 별칭이 두 축을 한꺼번에 덮으면 분명히 '질병'이라고
    // 적혀 있던 구분이 별칭표의 '출석인정'으로 조용히 바뀐다.
    let mut half = sick_absence(5);
    half.code_label = Some("인정결석".to_string());
    half.type_label = None;

    let out = preview_neis_import_impl(&conn, class, &[half], TODAY).unwrap();
    assert_eq!(out.unreadable, 0);
    assert_eq!(out.items[0].their_axis, "질병 결석");
}

#[test]
fn 한쪽_축만_맞은_줄은_절반을_버리지_않는다() {
    let (conn, school, class) = fixture();
    enroll(&conn, class, school, 5, "학생5");

    // 구분은 맞았고 종류를 못 찾았다. 이것을 "질병 미정"으로 넣으면 파일이 분명히
    // 말한 절반을 앱이 버린 것이 되고, 교사는 무엇이 빠졌는지 알 방법이 없다.
    let mut half = sick_absence(5);
    half.code_label = Some("질병공결".to_string());
    half.type_label = Some("공결".to_string());

    let out = preview_neis_import_impl(&conn, class, &[half], TODAY).unwrap();
    assert_eq!(out.unreadable, 1);
    let why = out.items[0].why.as_deref().unwrap();
    assert!(why.contains("종류"), "어느 쪽을 못 찾았는지 적는다: {why}");
    assert!(why.contains("공결"));
}

#[test]
fn 결시교시가_비어_온_결석도_하루_종일로_본다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    stamp(&conn, class, student, "질병", "결석", &[]);

    // 결석은 기간을 묻지 않는 종류라 앱이 언제나 조회~종례로 저장한다. 나이스 파일에
    // 결시교시가 비어 오는 줄이 있는데, 그대로 비교하면 화면에 양쪽 다 "하루 종일"로
    // 보이는 두 줄이 '다름'으로 남는다.
    let mut blank = sick_absence(5);
    blank.start_slot = None;
    blank.end_slot = None;

    let out = preview_neis_import_impl(&conn, class, &[blank], TODAY).unwrap();
    assert_eq!(out.same, 1);
    assert_eq!(out.differ, 0);
    assert_eq!(out.items[0].their_span, "하루 종일");
}

#[test]
fn 결시교시가_비어_온_결석도_조회부터_종례까지_저장한다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");

    let mut blank = sick_absence(5);
    blank.start_slot = None;
    blank.end_slot = None;
    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    apply_neis_import_impl(&conn, class, &[blank], &choice, TODAY).unwrap();

    let (start, end): (Option<String>, Option<String>) = conn
        .query_row("SELECT start_slot, end_slot FROM absence_span", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!((start.as_deref(), end.as_deref()), (Some("조회"), Some("종례")));

    // NULL로 들어가면 같은 조합을 다시 입력해도 그 건을 찾지 못해 똑같은 기록이
    // 하나 더 쌓인다. 찾았다면 추가가 아니다.
    //
    // 가져온 건에는 나이스의 사유가 메모로 들어가 있어 **지우지는 않는다**(`kept`) —
    // 교사가 적어 둔 것이 무르기로 사라지면 안 되기 때문이다. 여기서 보는 것은
    // 그 건을 찾았는가이고, 찾지 못했다면 `added`로 한 건이 더 생겼을 것이다.
    let (reason_id, type_id) = axes(&conn, "질병", "결석");
    let out = stamp_span_impl(
        &conn,
        &StampInput {
            class_id: class,
            student_id: student,
            date: DATE.to_string(),
            reason_id,
            type_id,
            slots: vec![],
        },
    )
    .unwrap();
    assert_eq!(out.action, "kept");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "같은 건을 찾았으므로 한 건도 더 쌓이지 않는다");
}

#[test]
fn 고른_것이_적용되지_않으면_세어서_알린다() {
    let (conn, school, class) = fixture();
    let student = enroll(&conn, class, school, 5, "학생5");
    stamp(&conn, class, student, "질병", "결석", &[]);

    // 미리보기에서는 '추가'였는데 그 사이에 같은 건이 생겨 '같음'이 된 상황이다.
    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    let out = apply_neis_import_impl(&conn, class, &rows, &choice, TODAY).unwrap();

    assert_eq!(out.added, 0);
    assert_eq!(out.skipped, 1, "조용히 넘기지 않는다");
}
