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
    pub enrolled_from: String,
    pub enrolled_to: Option<String>,
}

/// 명렬표 한 줄. 붙여넣기·CSV·직접 입력이 모두 이 형태로 수렴한다.
///
/// 학년·반이 비어 있으면 화면이 고른 학급을 쓴다. 나이스 명렬표는 보통
/// (학년, 반, 번호, 성명) 네 열이지만, 두 열만 복사해 오는 경우도 흔하다.
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

/// 명렬표에서 읽어낸 학급. 첫 실행에서 "우리 반"을 되묻지 않기 위한 것이다.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RosterClass {
    pub grade: Option<i64>,
    pub class_no: Option<i64>,
    /// 명렬표에 학급이 둘 이상 섞여 있는지. 그러면 교사가 골라야 한다.
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

/// 축 1 — 질병 · 출석인정 · 미인정 · 기타
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

/// 축 2 — 결석 · 지각 · 조퇴 · 결과
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeItem {
    pub id: i64,
    pub label: String,
    /// none | start | end | both — 교사에게 물어야 하는 교시가 어느 쪽인지.
    pub slot_prompt: String,
    pub shortcut: Option<String>,
    pub sort_order: i64,
    pub valid_from: String,
    pub valid_to: Option<String>,
}

/// 두 축이 모두 정해졌을 때만 존재하는 쌍. 문구 패턴이 여기 붙는다.
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

// ── 부재 구간 ─────────────────────────────────────────────────

/// 두 축은 각각 비어 있을 수 있다. 비어 있으면 "아직 안 정했다"는 뜻이다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanItem {
    pub id: i64,
    pub student_id: i64,
    pub date: String,
    pub reason_id: Option<i64>,
    pub type_id: Option<i64>,
    pub reason_label: Option<String>,
    pub type_label: Option<String>,
    /// 두 축이 다 정해졌을 때의 코드 라벨. 아니면 None.
    pub code_label: Option<String>,
    pub start_slot: Option<String>,
    pub end_slot: Option<String>,
    pub symptom: Option<String>,
    pub group_id: Option<String>,
    /// `"* ~ *"` 같은 요약 표기
    pub span_text: String,
    /// 두 축이 모두 채워졌는가. false면 나이스에 낼 수 없는 미완성 기록이다.
    pub complete: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyReasonItem {
    pub reason_id: Option<i64>,
    pub type_id: Option<i64>,
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
    /// 대표 연락처 하나(sort_order가 가장 앞선 것). 없으면 None.
    pub contact_label: Option<String>,
    pub contact_value: Option<String>,
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

// ── 대시보드 ──────────────────────────────────────────────────

/// HOME이 한 번에 받아 가는 요약. 화면이 여러 커맨드를 조합하지 않게 한다.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeSummary {
    pub date: String,
    pub date_label: String,
    pub enrolled: i64,
    /// 오늘 구간이 하나라도 있는 학생 수
    pub recorded: i64,
    /// 두 축 중 하나라도 비어 있는 구간의 수. 채워 넣어야 할 것들이다.
    pub incomplete: i64,
    pub pending: Vec<PendingSummary>,
    /// 마감이 지난 미제출 건수
    pub overdue: i64,
}
