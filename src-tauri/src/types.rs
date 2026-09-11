//! 프런트엔드와 주고받는 자료형.
//!
//! Rust는 snake_case, 프런트는 camelCase다. `#[serde(rename_all = "camelCase")]`가
//! 그 경계를 맡는다. 여기 있는 구조체가 곧 화면과의 계약이고, 화면은 이 형태 말고
//! 다른 조합을 만들지 않는다 — 조합 규칙이 프런트로 새면 그게 비즈니스 로직이 된다.

use serde::{Deserialize, Serialize};

// ── 학교 ──────────────────────────────────────────────────────

/// 학교 단위 설정. 최대 교시와 제출 기한이 여기 있다.
///
/// **학교는 학년도 안에 있다.** `year_id`가 그 자리를 가리킨다 — 2026학년도의 A학교와
/// 2027학년도의 A학교는 다른 행이고, 최대 교시 · 제출 기한도 따로 든다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchoolItem {
    pub id: i64,
    pub year_id: i64,
    pub name: String,
    pub max_slot: i64,
    pub due_days: i64,
    pub due_skip_offdays: bool,
    pub sort_order: i64,
    pub active: bool,
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

/// 학년도 한 줄. **뒤의 다섯 수는 "이 학년도를 지우면 무엇이 함께 사라지는가"다.**
///
/// 삭제는 `ON DELETE CASCADE`로 그 아래를 통째로 지우는, 되돌릴 수 없는 쓰기다.
/// 확인 대화상자가 그 양을 **묻기 전에** 보여줘야 하는데, 수를 따로 받아 오는 커맨드를
/// 두면 화면이 대화상자를 여는 순간 한 번 더 기다린다.
///
/// **담임 출결과 교과 차시를 한 수로 합치지 않는다.** 두 기록은 표부터 다르고, 합친
/// 수는 "출결 300건"이 담임 것인지 교과 것인지 말해 주지 못한다.
///
/// 마감한 것(`school.active = 0` · `teaching_class.valid_to`)도 센다 —
/// 지울 때는 그것들도 똑같이 사라지기 때문이다. 목록 조회와 세는 기준이 다른 것이 맞다.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcademicYearItem {
    pub id: i64,
    pub year: i64,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    /// 그 학년도의 학교 수.
    pub school_count: i64,
    /// 담당 학급 · 강좌 수(`teaching_class`). 담임과 교과를 함께 센다.
    pub class_count: i64,
    /// 학적 수(`student`). 명단에서 제외한 학생도 학적은 남아 있으므로 함께 센다.
    pub student_count: i64,
    /// 담임 출결 건수(`absence_span`).
    pub span_count: i64,
    /// 교과 차시 수(`subject_session`).
    pub session_count: i64,
}

// ── 담당 학급 · 강좌 ──────────────────────────────────────────

/// 담임 학급 하나 또는 교과 강좌 하나. **화면의 범위가 이 행의 `id`다.**
///
/// `grade` · `class_no`는 담임일 때만 채워진다. 교과 강좌는 여러 반에서 모이므로
/// 가리킬 반이 없다.
///
/// **학년도를 따로 들지 않는다.** 학교가 이미 학년도를 안다 —
/// 학년도 → 학교 → 담당 학급 · 강좌가 이 앱의 계층이다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeachingClassItem {
    pub id: i64,
    pub school_id: i64,
    /// homeroom | subject — 기록하는 것이 달라 화면이 나뉜다.
    pub role: String,
    pub name: String,
    pub grade: Option<i64>,
    pub class_no: Option<i64>,
    /// 화면에서 함께 묶어 보일 이름표. `프로그래밍A · B · C`를 묶는다.
    /// **분반 표가 아니다** — 강좌는 저마다 독립한 행이고 이것은 이름일 뿐이다.
    pub group_tag_id: Option<i64>,
    pub group_tag_name: Option<String>,
    pub sort_order: i64,
    pub valid_from: String,
    pub valid_to: Option<String>,
    /// 지금 명단에 있는 인원. 이동 화면이 "어느 쪽이 내가 찾던 강좌인지"를
    /// 이름만으로 구별하지 못할 때 이 숫자가 구별한다.
    pub member_count: i64,
}

/// 강좌 묶음 이름표 하나. **`TagItem`(출결 태그)과 다른 것이다** —
/// 그쪽은 한도를 세는 대상이고 이쪽은 화면에서 강좌를 묶는 이름이다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassTagItem {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    /// 이 이름표를 단 강좌 수. 지우기 전에 무엇이 풀리는지 보여준다.
    pub class_count: i64,
}

// ── 학생 ──────────────────────────────────────────────────────

/// 학적 한 줄. **학년도를 따로 들지 않는다** — 학교가 이미 학년도를 안다.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentItem {
    pub id: i64,
    pub school_id: i64,
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
    /// 파일의 몇 번째 줄인가. **버린 줄이 자기를 가리키기 위한 값이다** —
    /// 교과에서 '4번 김하늘'이 두 줄 나란히 나타나면 어느 줄을 고칠지 말하지 못한다.
    #[serde(default)]
    pub line: Option<i64>,
}

/// 다시 열기 미리보기 한 줄.
///
/// 명렬표를 다시 여는 것은 교체가 아니라 차분이다. 사라진 번호는 삭제하지 않고 전출 처리한다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RosterDiffRow {
    /// 화면 목록의 열쇠이자 미리보기와 적용을 연결하는 자리표.
    /// 교과는 번호가 겹쳐 번호로 줄을 구별할 수 없다. (`NeisDiffItem.key`와 같은 뜻)
    #[serde(default)]
    pub key: usize,
    /// 그 줄이 가리키는 학적 자리. **담임 적용은 이 둘을 읽지 않는다** — 보여주기용이다.
    #[serde(default)]
    pub grade: Option<i64>,
    #[serde(default)]
    pub class_no: Option<i64>,
    #[serde(default)]
    pub line: Option<i64>,
    pub number: i64,
    pub incoming_name: Option<String>,
    pub current_name: Option<String>,
    pub student_id: Option<i64>,
    /// added | unchanged | renamed | withdrawn | linked | blocked
    pub action: String,
    /// 그 줄에 붙는 말. blocked의 이유 등. **앱이 이 문장을 다시 해석하지 않는다.**
    #[serde(default)]
    pub why: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterApplyResult {
    /// 명단에 들어온 줄.
    pub added: i64,
    /// 그중 학적을 새로 만든 수. 교과에서 인원과 같으면 파일의 반이 통째로 틀린 신호다.
    pub created: i64,
    pub renamed: i64,
    pub withdrawn: i64,
    /// 배치할 수 없어 넘긴 줄.
    pub blocked: i64,
    /// 담임이 자리를 넘겨받으며 마감한 학적. 되돌릴 수 없는 쓰기다.
    pub seat_closed: i64,
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

/// 연락처 한 줄. **학생마다 있는 번호가 다르다** — 본인만 있는 학생, 어머니만 있는
/// 학생, 둘 다 있는 학생. 그래서 칸이 아니라 줄이다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactItem {
    #[serde(default)]
    pub id: i64,
    /// 본인 · 부 · 모 · 조부모 · 시설 … 목록을 코드에 박지 않는다.
    #[serde(rename = "type")]
    pub kind: String,
    pub phone: String,
    /// "주간에는 받지 않음" 같은 것. 앱은 해석하지 않는다.
    #[serde(default)]
    pub memo: String,
    /// 먼저 걸 번호의 순서를 설정한다. 급한 순간에 고민하지 않게.
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
    /// year | month
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
    /// 그 학생의 **학적**이다. 학급의 학년 · 반이 아니다 — 명단에 반이 다른 학생이
    /// 들어올 수 있고, 문자 발송 시스템은 이 세 값으로 수신자를 찾는다.
    pub grade: i64,
    pub class_no: i64,
    pub number: i64,
    pub name: String,
    pub date: String,
    pub date_label: String,
    pub reason_id: Option<i64>,
    pub type_id: Option<i64>,
    pub reason_label: Option<String>,
    pub type_label: Option<String>,
    /// 두 축이 다 결정되었을 때의 코드 라벨. 아니면 None.
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

/// 입력 요청. 같은 조합이 이미 있으면 **취소**된다.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StampInput {
    /// 어느 담임 학급의 기록인가. **학생이 아니라 학급이 범위다** —
    /// 같은 학생이 내 교과 강좌에도 있을 수 있어, 학생만으로는 자리를 결정할 수 없다.
    pub class_id: i64,
    pub student_id: i64,
    pub date: String,
    #[serde(default)]
    pub reason_id: Option<i64>,
    #[serde(default)]
    pub type_id: Option<i64>,
    /// 선택한 교시들. 결과처럼 여럿일 수 있고, 비어 있으면 기간 미정이다.
    /// 연속한 교시는 Rust가 한 구간으로 묶는다.
    #[serde(default)]
    pub slots: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StampResult {
    /// added | cancelled | kept
    ///
    /// `kept`는 같은 조합이 이미 있는데 그 구간에 태그 · 사유 · 서류 · NEIS 표시가
    /// 남아 있어 **무르지 않은** 경우다. 그것까지 지우면 다시 입력해도 빈 구간만
    /// 돌아와, 교사가 적어 둔 것이 말없이 사라진다.
    pub action: String,
    pub span_ids: Vec<i64>,
    /// 화면이 그대로 보여줄 한 문장. `kept`일 때만 채워진다.
    pub message: Option<String>,
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

/// 집중 등재에서 한 명분으로 넘어오는 값. **그 구간의 최종 상태다.**
///
/// 모달은 복사본을 고치고 [저장]에서만 넘긴다. 그래서 축 · 기간 · 사유 · 태그를
/// 한 번에 받아 통째로 반영하고 나이스 등재로 표시한다.
///
/// 축과 기간은 `SpanEdit`을 그대로 전개해 쓴다. 같은 값을 다시 적으면 수정 모달이
/// 보내는 것과 이 화면이 보내는 것이 달라지고, 달라진 쪽만 고쳐진다.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusEntry {
    #[serde(flatten)]
    pub edit: SpanEdit,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub tag_id: Option<i64>,
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
    /// 그 학생의 학적. 파일 저장가 이 세 값으로 수신자를 찾게 한다.
    pub grade: i64,
    pub class_no: i64,
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
    /// 실제로 센 구간(ISO). 학년도 창과 교사가 선택한 구간의 교집합이다.
    pub window_from: String,
    pub window_to: String,
    /// **그 창 밖이라 세지 못한 구간 수.** 조용히 넘기지 않는다 —
    /// 학년도는 기준 연도일 뿐 날짜 울타리가 아니라, 2026학년도 학급에 2027-03-05를
    /// 입력하는 일이 실제로 있다. 그 건은 한도에도 태그 누락 목록에도 들어가지 않으므로
    /// 세어서 알리지 않으면 교사에게 "덜 썼다"고 거짓으로 말하게 된다.
    pub outside: i64,
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

// ── 교과 차시 ─────────────────────────────────────────────────
//
// 교과 교사가 기록하는 것은 `내 수업에 있었는가` 하나뿐이다. 구분 · 종류 · 기간도,
// 서류도, 나이스도 없다. 담임 쪽 자료형을 재사용하지 않는 이유가 이것이다.

/// 수업 한 칸. **이 행이 있다는 것이 곧 그 교시를 불렀다는 뜻이다.**
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectSessionItem {
    pub id: i64,
    pub date: String,
    /// `1` ~ `9`. 조회 · 종례는 들어올 수 없다 — 그 둘은 담임이 보는 하루의 양 끝이다.
    pub slot: String,
    pub memo: String,
    /// 그 차시에서 결석한 학생 수.
    pub absent_count: i64,
    /// 그 날짜의 명단 인원. 분모다 — `absent_count`만으로는 "몇 명 중"을 말할 수 없다.
    pub total: i64,
}

/// 차시 한 칸의 명단 한 줄. 교과가 기록하는 것은 `absent` 하나뿐이다.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectRollItem {
    pub student_id: i64,
    /// 그 학생의 **학적**이다. 교과 강좌는 반이 섞여 번호만으로는 줄을 구별할 수 없다.
    pub grade: i64,
    pub class_no: i64,
    pub number: i64,
    pub name: String,
    /// 그 차시에 빠졌는가.
    pub absent: bool,
    pub memo: String,
    /// 같은 날 그 학생에게 **담임으로 적어 둔** 출결을 사람이 읽는 한 문장으로.
    ///
    /// **읽기 전용 참고다.** 내가 담임인 학급의 기록만 보고, 이 화면에서 적은 것과
    /// 눈에 띄게 구별되어야 한다 — 교과 수업에서 빈 자리를 보았는데 아침에 담임으로
    /// 질병결석을 입력해 두었으면 그것을 알려주는 것이 기능이지 유출이 아니다.
    pub homeroom_note: Option<String>,
}

// ── 나이스 파일 열기 ───────────────────────────────────────────
//
// 파일 서식은 프런트(`services/neisFile.js`)가 맡는다. 여기부터가 업무 규칙이다 —
// 명렬표 파일 열기와 같은 경계다. 그래서 이 구조체들은 파일이 아니라 **읽어 낸 줄**을 받는다.

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

/// 차분 한 건. **무엇을 할지는 교사가 선택한다.**
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeisDiffItem {
    /// 미리보기와 적용을 연결하는 자리표. 파일에서 읽은 줄의 순번이다.
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

/// 교사가 선택한 것. 여기 없는 것은 손대지 않는다.
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
    /// 교사가 선택했는데 적용되지 않은 건수. 미리보기 이후에 그 기록이 바뀐 경우다.
    pub skipped: i64,
}
