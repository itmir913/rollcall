use serde::{Deserialize, Serialize};

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
    pub year_id: i64,
    pub grade: i64,
    pub class_no: i64,
    pub number: i64,
    pub name: String,
    pub guardian_phone: Option<String>,
    pub enrolled_from: String,
    pub enrolled_to: Option<String>,
}

/// 붙여넣기·CSV·직접 입력이 모두 이 형태로 수렴한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterEntry {
    pub number: i64,
    pub name: String,
    #[serde(default)]
    pub guardian_phone: Option<String>,
}

/// 재가져오기 미리보기 한 줄.
///
/// 재가져오기는 교체가 아니라 차분이다. 사라진 번호는 삭제하지 않고 전출 처리한다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RosterDiffRow {
    pub number: i64,
    /// 명단에 있는 이름. 전출 후보에는 없다.
    pub incoming_name: Option<String>,
    /// DB에 있는 이름. 신규에는 없다.
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

// ── 출결 코드 ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceCodeItem {
    pub id: i64,
    pub reason: String,
    #[serde(rename = "type")]
    pub code_type: String,
    pub label: String,
    pub phrase_pattern: Option<String>,
    pub default_start: Option<String>,
    pub default_end: Option<String>,
    pub shortcut: Option<String>,
    pub sort_order: i64,
    pub valid_from: String,
    pub valid_to: Option<String>,
    /// 유형에서 파생한 값. 프론트가 다시 계산하지 않도록 함께 내린다.
    /// none | start | end | both
    pub slot_prompt: String,
}

// ── 부재 구간 ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanItem {
    pub id: i64,
    pub student_id: i64,
    pub date: String,
    pub code_id: i64,
    pub code_label: String,
    pub code_type: String,
    pub start_slot: Option<String>,
    pub end_slot: Option<String>,
    pub symptom: Option<String>,
    pub group_id: Option<String>,
    /// `"* ~ *"` 같은 요약 표기
    pub span_text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyReasonItem {
    pub code_id: Option<i64>,
    pub reason: String,
}

// ── 체크 ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CheckItemDef {
    pub id: i64,
    pub name: String,
    pub due_days: Option<i64>,
    pub include_weekend: bool,
    pub default_done: bool,
    pub sort_order: i64,
    pub active: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyCheckItem {
    pub item_id: i64,
    pub done: bool,
    pub due_date: Option<String>,
    pub done_at: Option<String>,
}

// ── 일일 격자 ─────────────────────────────────────────────────

/// 격자 한 행. 재학 중인 학생은 구간이 없어도 행이 나온다.
/// 빈 행은 출석을 뜻하며 저장되지 않는다.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayRow {
    pub student_id: i64,
    pub number: i64,
    pub name: String,
    pub spans: Vec<SpanItem>,
    pub reason: Option<DailyReasonItem>,
    pub checks: Vec<DailyCheckItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayGrid {
    pub date: String,
    pub rows: Vec<DayRow>,
    pub items: Vec<CheckItemDef>,
}

// ── 기간 일괄 입력 ────────────────────────────────────────────

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

// ── 미제출자 ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingRow {
    pub student_id: i64,
    pub grade: i64,
    pub class_no: i64,
    pub number: i64,
    pub name: String,
    pub guardian_phone: Option<String>,
    pub date: String,
    pub item_id: i64,
    pub item_name: String,
    pub due_date: Option<String>,
    /// 마감 경과일. 마감이 없으면 None.
    pub days_overdue: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingSummary {
    pub item_id: i64,
    pub item_name: String,
    pub count: i64,
}
