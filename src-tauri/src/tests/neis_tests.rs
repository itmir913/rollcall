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
use crate::commands::neis::{apply_neis_import_impl, preview_neis_import_impl, Scope};
use crate::types::{NeisImportChoice, NeisRowInput, StampInput};
use rusqlite::Connection;

const TODAY: &str = "2026-09-11";
const DATE: &str = "2026-09-01";

fn scope(conn: &Connection, year_id: i64) -> Scope {
    Scope {
        school_id: school_id(conn),
        year_id,
        grade: 3,
        class_no: 6,
    }
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

fn stamp(conn: &Connection, student_id: i64, reason: &str, kind: &str, slots: &[&str]) {
    let (reason_id, type_id) = axes(conn, reason, kind);
    stamp_span_impl(
        conn,
        &StampInput {
            student_id,
            date: DATE.to_string(),
            reason_id,
            type_id,
            slots: slots.iter().map(|s| s.to_string()).collect(),
        },
    )
    .unwrap();
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
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 5, "학생5");

    let rows = vec![sick_absence(5)];
    let out = preview_neis_import_impl(&conn, scope(&conn, year), &rows, TODAY).unwrap();

    assert_eq!(out.add, 1);
    assert_eq!(out.same, 0);
    assert_eq!(out.items[0].verdict, "add");
    assert_eq!(out.items[0].their_axis, "질병 결석");
    assert_eq!(out.items[0].their_span, "하루 종일");
}

#[test]
fn 똑같은_기록은_같음이고_손대지_않는다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 5, "학생5");
    stamp(&conn, student, "질병", "결석", &[]);

    let rows = vec![sick_absence(5)];
    let out = preview_neis_import_impl(&conn, scope(&conn, year), &rows, TODAY).unwrap();

    assert_eq!(out.same, 1);
    assert_eq!(out.add, 0);
    assert_eq!(out.only_mine, 0);
}

#[test]
fn 내용이_다르면_다름이고_교사가_고른다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 5, "학생5");
    stamp(&conn, student, "미인정", "결석", &[]);

    let rows = vec![sick_absence(5)];
    let out = preview_neis_import_impl(&conn, scope(&conn, year), &rows, TODAY).unwrap();

    assert_eq!(out.differ, 1);
    let item = &out.items[0];
    assert_eq!(item.my_axis.as_deref(), Some("미인정 결석"));
    assert_eq!(item.their_axis, "질병 결석");
    assert!(item.span_id.is_some(), "고칠 대상을 알려줘야 고를 수 있다");
}

#[test]
fn 고르지_않으면_아무것도_바뀌지_않는다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 5, "학생5");
    stamp(&conn, student, "미인정", "결석", &[]);

    let rows = vec![sick_absence(5)];
    let out = apply_neis_import_impl(&conn, scope(&conn, year), &rows, &nothing(), TODAY).unwrap();

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
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 5, "학생5");
    insert_student(&conn, year, 6, "학생6");

    let rows = vec![sick_absence(5), sick_absence(6)];
    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    let out = apply_neis_import_impl(&conn, scope(&conn, year), &rows, &choice, TODAY).unwrap();

    assert_eq!(out.added, 1);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn 가져온_건은_나이스_등재로_들어간다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 5, "학생5");

    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    apply_neis_import_impl(&conn, scope(&conn, year), &rows, &choice, TODAY).unwrap();

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
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 5, "학생5");
    stamp(&conn, student, "질병", "결석", &[]);

    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![],
        replace: vec![],
        mark_neis: true,
    };
    let out = apply_neis_import_impl(&conn, scope(&conn, year), &rows, &choice, TODAY).unwrap();

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
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 5, "학생5");
    stamp(&conn, student, "미인정", "결석", &[]);

    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![],
        replace: vec![0],
        mark_neis: false,
    };
    let out = apply_neis_import_impl(&conn, scope(&conn, year), &rows, &choice, TODAY).unwrap();

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
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 5, "학생5");
    stamp(&conn, student, "미인정", "결석", &[]);
    conn.execute("UPDATE absence_span SET memo = '학부모 통화함'", [])
        .unwrap();

    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![],
        replace: vec![0],
        mark_neis: false,
    };
    apply_neis_import_impl(&conn, scope(&conn, year), &rows, &choice, TODAY).unwrap();

    let memo: String = conn
        .query_row("SELECT memo FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(memo, "학부모 통화함");
}

#[test]
fn 앱에만_있는_기록은_세기만_하고_지우지_않는다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student5 = insert_student(&conn, year, 5, "학생5");
    let student6 = insert_student(&conn, year, 6, "학생6");
    stamp(&conn, student5, "질병", "결석", &[]);
    stamp(&conn, student6, "질병", "조퇴", &["3"]);

    // 파일에는 5번만 있다.
    let rows = vec![sick_absence(5)];
    let choice = NeisImportChoice {
        add: vec![],
        replace: vec![],
        mark_neis: true,
    };
    let out = preview_neis_import_impl(&conn, scope(&conn, year), &rows, TODAY).unwrap();
    assert_eq!(out.only_mine, 1, "나이스에 아직 안 넣은 것이 하나 있다");

    apply_neis_import_impl(&conn, scope(&conn, year), &rows, &choice, TODAY).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM absence_span", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "파일에 없다는 것이 삭제 근거가 될 수 없다");
}

#[test]
fn 하루_두_구간은_여럿을_여럿과_비교한다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    let student = insert_student(&conn, year, 5, "학생5");
    // 1교시까지 지각 + 6교시부터 조퇴. 나이스 실파일에 있던 하루 2구간이다.
    stamp(&conn, student, "질병", "지각", &["1"]);
    stamp(&conn, student, "질병", "조퇴", &["6"]);

    let rows = vec![
        row(5, "질병지각", "질병", "지각", Some("조회"), Some("1")),
        row(5, "질병조퇴", "질병", "조퇴", Some("6"), Some("종례")),
    ];
    let out = preview_neis_import_impl(&conn, scope(&conn, year), &rows, TODAY).unwrap();

    assert_eq!(out.same, 2, "하나씩 비교하면 둘 다 다름으로 잡힌다");
    assert_eq!(out.differ, 0);
    assert_eq!(out.only_mine, 0);
}

#[test]
fn 결시교시가_조회_하나뿐인_지각도_들어간다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 18, "학생18");

    // 나이스 실파일의 `질병지각 · 결시교시 조회,`. 화면의 교시 고르개는 지각에서
    // 조회를 열지 않지만, 파일에서 온 값은 그대로 저장한다 — 우리가 못 고르는 것과
    // 나이스에 그렇게 적혀 있는 것은 다른 문제다.
    let rows = vec![row(18, "질병지각", "질병", "지각", Some("조회"), Some("조회"))];
    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    apply_neis_import_impl(&conn, scope(&conn, year), &rows, &choice, TODAY).unwrap();

    let (start, end): (String, String) = conn
        .query_row("SELECT start_slot, end_slot FROM absence_span", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!((start.as_str(), end.as_str()), ("조회", "조회"));
}

#[test]
fn 명렬표에_없는_번호는_조용히_넘기지_않는다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 5, "학생5");

    let rows = vec![sick_absence(99)];
    let out = preview_neis_import_impl(&conn, scope(&conn, year), &rows, TODAY).unwrap();

    assert_eq!(out.unreadable, 1);
    assert_eq!(out.items[0].verdict, "unreadable");
    assert!(out.items[0].why.as_deref().unwrap().contains("명렬표"));
}

#[test]
fn 모르는_출결_표기는_지어내지_않는다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 5, "학생5");

    let mut bad = sick_absence(5);
    bad.code_label = Some("공결".to_string());
    bad.reason_label = None;
    bad.type_label = None;

    let out = preview_neis_import_impl(&conn, scope(&conn, year), &[bad], TODAY).unwrap();
    assert_eq!(out.unreadable, 1);
    assert!(out.items[0].why.as_deref().unwrap().contains("공결"));
}

#[test]
fn 학교마다_다른_표기는_별칭표가_흡수한다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 5, "학생5");
    let code: i64 = conn
        .query_row(
            "SELECT c.id FROM attendance_code c
               JOIN attendance_reason r ON r.id = c.reason_id
               JOIN attendance_type t ON t.id = c.type_id
              WHERE r.label = '출석인정' AND t.label = '결석'",
            [],
            |r| r.get(0),
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

    let out = preview_neis_import_impl(&conn, scope(&conn, year), &[aliased], TODAY).unwrap();
    assert_eq!(out.unreadable, 0);
    assert_eq!(out.items[0].their_axis, "출석인정 결석");
}

#[test]
fn 다른_반의_기록은_섞이지_않는다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 5, "학생5");
    // 같은 학교 다른 반의 같은 번호.
    conn.execute(
        "INSERT INTO student (school_id, year_id, grade, class_no, number, name, enrolled_from)
         VALUES (?1, ?2, 3, 7, 5, '다른반학생', '2026-03-02')",
        rusqlite::params![school_id(&conn), year],
    )
    .unwrap();
    let other: i64 = conn.last_insert_rowid();
    stamp(&conn, other, "질병", "결석", &[]);

    let out = preview_neis_import_impl(&conn, scope(&conn, year), &[sick_absence(5)], TODAY).unwrap();
    assert_eq!(out.add, 1, "6반에는 없는 기록이다");
    assert_eq!(out.only_mine, 0, "7반 기록을 세지 않는다");
}

#[test]
fn 마감은_가져올_때_계산해_박는다() {
    let conn = setup_test_db();
    let year = insert_year(&conn, 2026);
    insert_student(&conn, year, 5, "학생5");

    let choice = NeisImportChoice {
        add: vec![0],
        replace: vec![],
        mark_neis: false,
    };
    apply_neis_import_impl(&conn, scope(&conn, year), &[sick_absence(5)], &choice, TODAY).unwrap();

    let due: Option<String> = conn
        .query_row("SELECT doc_due FROM absence_span", [], |r| r.get(0))
        .unwrap();
    // 시드는 7일 · 주말 제외다. 2026-09-01(화)에서 수업일로 7일 뒤.
    assert_eq!(due.as_deref(), Some("2026-09-10"));
}
