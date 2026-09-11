//! 프런트엔드와 주고받는 자료형.
//!
//! Rust는 snake_case, 프런트는 camelCase다. `#[serde(rename_all = "camelCase")]`가
//! 그 경계를 맡는다. 여기 있는 구조체가 곧 화면과의 계약이고, 화면은 이 형태 말고
//! 다른 조합을 만들지 않는다 — 조합 규칙이 프런트로 새면 그게 비즈니스 로직이 된다.

use serde::{Deserialize, Serialize};

// ── 학교 ──────────────────────────────────────────────────────

/// 학교 단위 설정. 최대 교시와 제출 기한이 여기 있다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchoolItem {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    pub max_slot: i64,
    pub due_days: i64,
    pub due_skip_offdays: bool,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default = "default_true")]
    pub active: bool,
}

fn default_true() -> bool {
    true
}

/// 마감을 셀 때 건너뛸 날. 학사일정이 아니라 그 목록일 뿐이다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OffDayItem {
    #[serde(default)]
    pub id: i64,
    pub date: String,
    #[serde(default)]
    pub label: Option<String>,
}

// ── 학년도 ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcademicYearItem {
    pub id: i64,
    pub year: i64,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
}

// ── 학생 ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentItem {
    pub id: i64,
    pub school_id: i64,
    pub year_id: i64,
    pub grade: i64,
    pub class_no: i64,
    pub number: i64,
    pub name: String,
    pub enrolled_from: String,
    pub enrolled_to: Option<String>,
}

/// 명렬표 한 줄. 파일에서 읽은 것이 이 형태로 수렴한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterEntry {
    #[serde(default)]
    pub grade: Option<i64>,
    #[serde(default)]
    pub class_no: Option<i64>,
    pub number: i64,
    pub name: String,
}

/// 재가져오기 미리보기 한 줄.
///
/// 재가져오기는 교체가 아니라 차분이다. 사라진 번호는 삭제하지 않고 전출 처리한다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RosterDiffRow {
    pub number: i64,
    pub incoming_name: Option<String>,
    pub current_name: Option<String>,
    pub student_id: Option<i64>,
    /// added | unchanged | renamed | withdrawn
    pub action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterApplyResult {
    pub added: i64,
    pub renamed: i64,
    pub withdrawn: i64,
}

/// 명렬표에서 읽어낸 학급. 첫 실행에서 "우리 반"을 되묻지 않기 위한 것이다.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RosterClass {
    pub grade: Option<i64>,
    pub class_no: Option<i64>,
    pub mixed: bool,
}

// ── 연락처 ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactItem {
    #[serde(default)]
    pub id: i64,
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
}

// ── 출결 축 ───────────────────────────────────────────────────

/// 축 1 — 질병 · 미인정 · 기타 · 출석인정
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasonItem {
    pub id: i64,
    pub label: String,
    pub shortcut: Option<String>,
    pub sort_order: i64,
    pub valid_from: String,
    pub valid_to: Option<String>,
}

/// 축 2 — 지각 · 조퇴 · 결석 · 결과
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeItem {
    pub id: i64,
    pub label: String,
    /// none | start | end | multi — 그 종류가 물어야 하는 기간이 어느 쪽인지.
    pub slot_prompt: String,
    pub shortcut: Option<String>,
    pub sort_order: i64,
    pub valid_from: String,
    pub valid_to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceCodeItem {
    pub id: i64,
    pub reason_id: i64,
    pub type_id: i64,
    pub reason_label: String,
    pub type_label: String,
    pub label: String,
    pub phrase_pattern: Option<String>,
    pub sort_order: i64,
    pub valid_from: String,
    pub valid_to: Option<String>,
}

// ── 태그 ──────────────────────────────────────────────────────

/// 세는 대상. 한 구간에 하나만 붙는다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagItem {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub valid_from: String,
    #[serde(default)]
    pub valid_to: Option<String>,
}

// ── 한도 규정 ─────────────────────────────────────────────────

/// "어떤 태그를 · 어느 기간에 · 몇 번까지". 입력을 막지 않는다 — 세어서 알려줄 뿐이다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaRuleItem {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub tag_id: Option<i64>,
    #[serde(default)]
    pub tag_name: Option<String>,
    #[serde(default)]
    pub reason_id: Option<i64>,
    #[serde(default)]
    pub type_id: Option<i64>,
    /// year | semester | month
    pub period: String,
    pub limit_n: i64,
    /// day(하루에 두 건이어도 1일) | count(건수)
    pub unit: String,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub valid_from: String,
    #[serde(default)]
    pub valid_to: Option<String>,
}

// ── 부재 구간 ─────────────────────────────────────────────────

/// 출결 한 건. 화면의 한 줄이 이것이다.
///
/// 두 축은 각각 비어 있을 수 있다(미정). 기간도 양쪽이 열릴 수 있다.
/// 서류와 나이스는 이 구간에 달린 두 개의 불리언이다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanItem {
    pub id: i64,
    pub student_id: i64,
    pub number: i64,
    pub name: String,
    pub date: String,
    pub date_label: String,
    pub reason_id: Option<i64>,
    pub type_id: Option<i64>,
    pub reason_label: Option<String>,
    pub type_label: Option<String>,
    /// 두 축이 다 정해졌을 때의 코드 라벨. 아니면 None.
    pub code_label: Option<String>,
    pub start_slot: Option<String>,
    pub end_slot: Option<String>,
    pub slot_prompt: Option<String>,
    /// `"조회부터 2교시까지"` 같은 사람이 읽는 표기. Rust가 만든다.
    pub span_text: String,
    pub tag_id: Option<i64>,
    pub tag_name: Option<String>,
    pub memo: String,
    pub doc_done: bool,
    pub doc_due: Option<String>,
    pub doc_done_on: Option<String>,
    /// 마감 경과일. 마감이 없거나 이미 받았으면 None.
    pub days_overdue: Option<i64>,
    pub neis_done: bool,
    pub neis_done_on: Option<String>,
    pub group_id: Option<String>,
    /// 두 축이 모두 채워졌는가. false면 나이스에 낼 수 없는 미완성 기록이다.
    pub complete: bool,
    /// 같은 날 같은 학생의 다른 구간과 시간이 겹치는가. 막지 않고 표시만 한다.
    pub overlapping: bool,
}

/// 찍기 요청. 같은 조합이 이미 있으면 **취소**된다.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StampInput {
    pub student_id: i64,
    pub date: String,
    #[serde(default)]
    pub reason_id: Option<i64>,
    #[serde(default)]
    pub type_id: Option<i64>,
    /// 고른 교시들. 결과처럼 여럿일 수 있고, 비어 있으면 기간 미정이다.
    /// 이어진 교시는 Rust가 한 구간으로 묶는다.
    #[serde(default)]
    pub slots: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StampResult {
    /// added | cancelled
    pub action: String,
    pub span_ids: Vec<i64>,
}

/// 구간 하나의 축과 기간을 고친다. 사유·태그는 별도 커맨드다.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanEdit {
    pub span_id: i64,
    #[serde(default)]
    pub reason_id: Option<i64>,
    #[serde(default)]
    pub type_id: Option<i64>,
    #[serde(default)]
    pub slots: Vec<String>,
}

// ── 화면별 묶음 ───────────────────────────────────────────────

/// 오늘의 출결 격자 한 행. 재학 중이면 구간이 없어도 행이 나온다.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayRow {
    pub student_id: i64,
    pub number: i64,
    pub name: String,
    pub spans: Vec<SpanItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayGrid {
    pub date: String,
    pub date_label: String,
    pub max_slot: i64,
    pub rows: Vec<DayRow>,
    pub spans: Vec<SpanItem>,
}

/// 출결 기록 · NEIS 미등재가 쓰는 날짜 묶음. 하루가 카드 하나다.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayGroup {
    pub date: String,
    pub date_label: String,
    pub enrolled: i64,
    pub spans: Vec<SpanItem>,
}

/// 개요가 한 번에 받아 가는 요약. 화면이 커맨드 여러 개를 조합하지 않게 한다.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeSummary {
    pub date: String,
    pub date_label: String,
    pub enrolled: i64,
    /// 오늘 구간이 하나라도 있는 학생 수
    pub recorded: i64,
    /// 두 축 중 하나라도 비어 있는 구간의 수
    pub incomplete: i64,
    pub doc_pending: i64,
    pub doc_overdue: i64,
    pub neis_pending: i64,
    /// 개요에 보여줄 몇 줄. 나머지는 각 화면이 전부 보여준다.
    pub doc_rows: Vec<SpanItem>,
    pub neis_rows: Vec<SpanItem>,
}

// ── 통계 (한도) ───────────────────────────────────────────────

/// 한도 표 한 줄. 학생 하나가 그 규정을 얼마나 썼는가.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaRow {
    pub student_id: i64,
    pub number: i64,
    pub name: String,
    pub used: i64,
    pub limit_n: i64,
    /// 쓴 날짜들(단위가 day일 때). 화면의 칸 하나가 이 날짜 하나다.
    pub dates: Vec<String>,
    /// 기간 단위가 month일 때 달별 사용 수. `("2026-06", 2)`
    pub buckets: Vec<QuotaBucket>,
    /// ok | near | over
    pub state: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaBucket {
    pub key: String,
    pub label: String,
    pub count: i64,
}

/// 규정 하나의 집계 결과.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaReport {
    pub rule: QuotaRuleItem,
    pub rows: Vec<QuotaRow>,
    pub used_total: i64,
    pub near_count: i64,
    pub over_count: i64,
    /// 태그가 비어 있어 세지 못한 구간. 조용히 넘기지 않는다.
    pub untagged: Vec<SpanItem>,
}

// ── 여러 날 일괄 입력 ─────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkPreviewDay {
    pub date: String,
    pub label: String,
    /// 이미 그날 구간이 있는 학생인지. 있으면 교사가 미리보기에서 뺄 수 있다.
    pub has_existing: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkApplyResult {
    pub group_id: String,
    pub days: i64,
}

// ── 나이스 가져오기 ───────────────────────────────────────────
//
// 파일 서식은 프런트(`services/neisFile.js`)가 맡는다. 여기부터가 업무 규칙이다 —
// 명렬표 가져오기와 같은 경계다. 그래서 이 구조체들은 파일이 아니라 **읽어 낸 줄**을 받는다.

/// 나이스 파일에서 읽은 출결 한 건.
///
/// 구분 · 종류는 이미 나뉘어 온다. 나누지 못한 경우 `code_label`만 채워져 오고,
/// Rust가 `code_alias`로 한 번 더 찾아본다.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeisRowInput {
    pub number: i64,
    pub date: String,
    pub code_label: Option<String>,
    pub reason_label: Option<String>,
    pub type_label: Option<String>,
    pub start_slot: Option<String>,
    pub end_slot: Option<String>,
    pub detail: Option<String>,
}

/// 차분 한 건. **무엇을 할지는 교사가 고른다.**
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeisDiffItem {
    /// 미리보기와 적용을 잇는 자리표. 파일에서 읽은 줄의 순번이다.
    pub key: usize,
    /// same · add · differ · unreadable
    pub verdict: String,
    pub number: i64,
    pub name: String,
    pub date: String,
    pub date_label: String,
    /// 나이스 쪽
    pub their_axis: String,
    pub their_span: String,
    pub detail: Option<String>,
    /// 앱 쪽. 짝이 없으면 비어 있다.
    pub span_id: Option<i64>,
    pub my_axis: Option<String>,
    pub my_span: Option<String>,
    /// 이미 나이스 등재로 표시해 둔 건인지. same일 때만 뜻이 있다.
    pub neis_done: bool,
    /// 읽지 못한 이유. verdict가 unreadable일 때만 채워진다.
    pub why: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeisImportPreview {
    pub items: Vec<NeisDiffItem>,
    pub same: i64,
    pub add: i64,
    pub differ: i64,
    pub unreadable: i64,
    /// 파일의 기간 안에 있는데 나이스에는 없는 내 기록. **지우지 않는다** — 세어서 알린다.
    pub only_mine: i64,
    pub from: String,
    pub to: String,
}

/// 교사가 고른 것. 여기 없는 것은 손대지 않는다.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeisImportChoice {
    pub add: Vec<usize>,
    pub replace: Vec<usize>,
    /// 나이스에 있는 것으로 확인된 건을 NEIS 등재로 표시할지.
    pub mark_neis: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeisImportResult {
    pub added: i64,
    pub replaced: i64,
    pub marked: i64,
    /// 교사가 골랐는데 적용되지 않은 건수. 미리보기 이후에 그 기록이 바뀐 경우다.
    pub skipped: i64,
}
