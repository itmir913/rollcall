-- ================================================================
-- 출결관리 스키마 v1
--
-- 설계 원칙(CLAUDE.md 참고)
--   · 슬롯을 펼치지 않는다 — 열린 구간은 NULL('?')이고 학사일정/시간표 테이블이 없다.
--   · 코드는 데이터다 — 구분·종류·태그·한도 규정은 행이지 enum이 아니다.
--   · 수정은 마감 후 추가다 — valid_to로 마감하고 새 행을 넣는다.
--   · **미완성 기록이 정상 상태다** — 구분과 종류는 각각 NULL일 수 있다.
--   · 학교 단위 설정(최대 교시, 제출 기한, 태그, 한도)은 school에 매단다.
--     순회 교사가 학교를 둘 이상 등록하는 날이 와도 자리를 옮기지 않는다.
-- ================================================================

-- ─── 학교 ──────────────────────────────────────────────────────
-- 지금은 행이 하나뿐이다. 그래도 자리를 학교에 만드는 이유는, 나중에 옮기려면
-- 이 값을 읽는 모든 곳을 다시 찾아야 하기 때문이다.
CREATE TABLE IF NOT EXISTS school
(
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    name             TEXT    NOT NULL,
    -- 하루의 마지막 교시. 조회와 종례는 설정 대상이 아니라 언제나 양 끝이다.
    max_slot         INTEGER NOT NULL DEFAULT 7 CHECK (max_slot BETWEEN 1 AND 9),
    -- 증빙 서류 제출 기한(결석일로부터). 0이면 결석일이 곧 마감이다.
    due_days         INTEGER NOT NULL DEFAULT 7 CHECK (due_days >= 0),
    -- 1이면 주말과 off_day에 등록된 휴업일을 세지 않는다.
    due_skip_offdays INTEGER NOT NULL DEFAULT 1 CHECK (due_skip_offdays IN (0, 1)),
    sort_order       INTEGER NOT NULL DEFAULT 0,
    active           INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1))
);

-- ─── 휴업일 ────────────────────────────────────────────────────
-- **학사일정 테이블이 아니다.** 제출 기한을 셀 때 건너뛸 날짜 목록일 뿐이고,
-- 교사가 설정 화면에서 직접 넣는다. 공휴일 API를 부르지 않는다 — 앱은 서버를
-- 쓰지 않고, 개교기념일·재량휴업일은 어차피 외부 달력에 없다.
CREATE TABLE IF NOT EXISTS off_day
(
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    school_id INTEGER NOT NULL REFERENCES school (id) ON DELETE CASCADE,
    date      TEXT    NOT NULL,
    label     TEXT,
    UNIQUE (school_id, date)
);

-- ─── 학년도 ────────────────────────────────────────────────────
-- 학년도는 학교에 매달지 않는다. 2026학년도는 어느 학교에서나 2026학년도다.
CREATE TABLE IF NOT EXISTS academic_year
(
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    year      INTEGER NOT NULL UNIQUE CHECK (year >= 1900),
    starts_on TEXT,
    ends_on   TEXT
);

-- ─── 학생 ──────────────────────────────────────────────────────
-- 명렬표가 (학년, 반, 번호, 이름)이므로 학급 정보는 학생이 들고 있다.
-- "우리 반"은 별도 테이블이 아니라 이 값들로 걸러낸 결과다.
-- 학교는 학생이 매단다 — 순회 교사는 같은 해에 여러 학교를 맡는다.
CREATE TABLE IF NOT EXISTS student
(
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    school_id     INTEGER NOT NULL REFERENCES school (id) ON DELETE CASCADE,
    year_id       INTEGER NOT NULL REFERENCES academic_year (id) ON DELETE CASCADE,
    grade         INTEGER NOT NULL CHECK (grade >= 1),
    class_no      INTEGER NOT NULL CHECK (class_no >= 1),
    number        INTEGER NOT NULL CHECK (number >= 1),
    name          TEXT    NOT NULL CHECK (name <> ''),
    enrolled_from TEXT    NOT NULL,
    enrolled_to   TEXT
);

-- 같은 학교·학년도·학급에서 재학 중인 번호는 하나뿐이다.
CREATE UNIQUE INDEX IF NOT EXISTS ux_student_active_number
    ON student (school_id, year_id, grade, class_no, number)
    WHERE enrolled_to IS NULL;

CREATE INDEX IF NOT EXISTS ix_student_class
    ON student (school_id, year_id, grade, class_no, number);

-- ─── 보호자 연락처 ─────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS contact
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    student_id INTEGER NOT NULL REFERENCES student (id) ON DELETE CASCADE,
    label      TEXT    NOT NULL,
    value      TEXT    NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS ix_contact_student ON contact (student_id, sort_order);

-- ─── 출결 구분 (축 1) ──────────────────────────────────────────
-- 질병 · 미인정 · 출석인정 · 기타
CREATE TABLE IF NOT EXISTS attendance_reason
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    label      TEXT    NOT NULL,
    shortcut   TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    valid_from TEXT    NOT NULL,
    valid_to   TEXT
);

-- ─── 출결 종류 (축 2) ──────────────────────────────────────────
-- 결석 · 지각 · 조퇴 · 결과
--
-- slot_prompt는 그 종류가 교사에게 물어야 하는 교시가 어느 쪽인지다.
--   none  = 하루 전체 (결석)      · start = 시작만 (조퇴)
--   end   = 끝만 (지각)           · multi = 여러 교시 (결과)
-- 종류가 데이터인 이상 이 속성도 데이터여야 한다. 코드에서 한국어 라벨을
-- 문자열로 비교하면 사용자가 종류를 추가하는 순간 틀린다.
CREATE TABLE IF NOT EXISTS attendance_type
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    label       TEXT    NOT NULL,
    slot_prompt TEXT    NOT NULL CHECK (slot_prompt IN ('none', 'start', 'end', 'multi')),
    shortcut    TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    valid_from  TEXT    NOT NULL,
    valid_to    TEXT
);

-- ─── 구분 × 종류 = 코드 ────────────────────────────────────────
-- 두 축이 모두 정해졌을 때만 존재한다. 문구 패턴과 나이스 표기가 여기 붙는다.
-- 고치지 않는다 — valid_to로 마감하고 새 행을 넣는다.
CREATE TABLE IF NOT EXISTS attendance_code
(
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    reason_id      INTEGER NOT NULL REFERENCES attendance_reason (id),
    type_id        INTEGER NOT NULL REFERENCES attendance_type (id),
    label          TEXT    NOT NULL, -- '질병조퇴'
    phrase_pattern TEXT,
    sort_order     INTEGER NOT NULL DEFAULT 0,
    valid_from     TEXT    NOT NULL,
    valid_to       TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_code_pair_active
    ON attendance_code (reason_id, type_id)
    WHERE valid_to IS NULL;

-- 나이스 표기 → 코드 매핑. 검증 기능과 학교별 표기 차이를 흡수한다.
CREATE TABLE IF NOT EXISTS code_alias
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    code_id INTEGER NOT NULL REFERENCES attendance_code (id) ON DELETE CASCADE,
    raw     TEXT    NOT NULL UNIQUE
);

-- ─── 태그 ──────────────────────────────────────────────────────
-- **세는 대상**이다. 체험학습은 출석인정 결석으로도 조퇴로도 나가므로 구분 × 종류
-- 조합으로는 셀 수 없다. 학교마다 부르는 이름이 다르고 한도 규정이 이 이름을
-- 가리키므로, 규정과 같은 곳(학교)에 매단다.
CREATE TABLE IF NOT EXISTS span_tag
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    school_id  INTEGER NOT NULL REFERENCES school (id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    valid_from TEXT    NOT NULL,
    valid_to   TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_tag_name_active
    ON span_tag (school_id, name)
    WHERE valid_to IS NULL;

-- ─── 부재 구간 (기록) ──────────────────────────────────────────
-- reason_id와 type_id는 **각각 NULL일 수 있다.** NULL은 "아직 안 정했다"는 뜻이다.
-- start_slot / end_slot의 NULL은 "열린 구간"이다. 둘 다 NULL이면 기간 미정이다.
--
-- 서류와 나이스는 **이 구간에 달린 두 개의 불리언**이다. 서류 '종류'는 기록하지
-- 않는다 — 학부모확인서냐 의사소견서냐는 학교 규정이고 해마다 바뀐다.
-- 무엇을 받기로 했는지는 memo에 적는다. memo는 자유 문장이고 앱은 해석하지 않는다.
CREATE TABLE IF NOT EXISTS absence_span
(
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    student_id   INTEGER NOT NULL REFERENCES student (id) ON DELETE CASCADE,
    date         TEXT    NOT NULL,
    reason_id    INTEGER REFERENCES attendance_reason (id), -- NULL = 미정
    type_id      INTEGER REFERENCES attendance_type (id),   -- NULL = 미정
    start_slot   TEXT,                                      -- NULL = 열린 시작
    end_slot     TEXT,                                      -- NULL = 열린 끝
    tag_id       INTEGER REFERENCES span_tag (id),          -- NULL = 태그 없음
    memo         TEXT    NOT NULL DEFAULT '',
    -- 증빙 서류
    doc_done     INTEGER NOT NULL DEFAULT 0 CHECK (doc_done IN (0, 1)),
    doc_done_on  TEXT,
    -- 마감일은 만들 때 계산해 **박아 둔다.** 설정을 바꿔도 과거 기록의 마감이
    -- 소급 변경되지 않아야 하고, 교사가 개별로 고칠 수 있어야 하기 때문이다.
    doc_due      TEXT,
    -- 나이스 등재
    neis_done    INTEGER NOT NULL DEFAULT 0 CHECK (neis_done IN (0, 1)),
    neis_done_on TEXT,
    group_id     TEXT,                                      -- 여러 날 일괄 입력 묶음
    created_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS ix_span_date ON absence_span (date);
CREATE INDEX IF NOT EXISTS ix_span_student ON absence_span (student_id, date);
CREATE INDEX IF NOT EXISTS ix_span_group ON absence_span (group_id);
CREATE INDEX IF NOT EXISTS ix_span_tag ON absence_span (tag_id);

-- 아직 채워지지 않은 기록을 찾는 질의가 자주 나온다.
CREATE INDEX IF NOT EXISTS ix_span_incomplete
    ON absence_span (date)
    WHERE reason_id IS NULL OR type_id IS NULL;

-- 서류 미제출자 · NEIS 미등재 화면이 매번 부르는 질의다.
CREATE INDEX IF NOT EXISTS ix_span_doc ON absence_span (doc_done, doc_due);
CREATE INDEX IF NOT EXISTS ix_span_neis ON absence_span (neis_done, date);

-- ─── 한도 규정 ─────────────────────────────────────────────────
-- 체험학습 연 20일, 생리통 조퇴 월 1회처럼 **세어야 하는 규정**이다.
--
-- **이 표는 입력을 막지 않는다.** absence_span에 CHECK도 트리거도 걸지 않는다 —
-- 한도를 넘었다는 이유로 교사의 입력을 거부하면, 실제로 넘긴 날 기록을 못 남긴다.
-- 프로그램은 판정하지 않는다. 통계 화면을 열 때 Rust가 세어서 알려줄 뿐이다.
-- 아래 CHECK는 규정 행 자체가 말이 되는지만 본다(기간 단위 오타 방지).
CREATE TABLE IF NOT EXISTS quota_rule
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    school_id  INTEGER NOT NULL REFERENCES school (id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    -- 무엇을 세는가. 태그가 기본이고, 두 축은 **선택적 추가 조건**이다.
    -- 셋 다 NULL이면 그 학교의 모든 구간을 센다.
    tag_id     INTEGER REFERENCES span_tag (id),
    reason_id  INTEGER REFERENCES attendance_reason (id),
    type_id    INTEGER REFERENCES attendance_type (id),
    -- 어느 기간에: 학년도 / 학기 / 달
    period     TEXT    NOT NULL CHECK (period IN ('year', 'semester', 'month')),
    limit_n    INTEGER NOT NULL CHECK (limit_n > 0),
    -- 무엇을 세는 단위로: 날짜(하루에 두 건이어도 1일) / 건수
    unit       TEXT    NOT NULL CHECK (unit IN ('day', 'count')),
    sort_order INTEGER NOT NULL DEFAULT 0,
    valid_from TEXT    NOT NULL,
    valid_to   TEXT
);

-- ─── 앱 설정 ───────────────────────────────────────────────────
-- 앱 전체에 걸린 값만 둔다(마지막에 연 학교·학년도·학급 같은 것).
-- **학교 단위 값은 여기 넣지 않는다.** school 행에 자리가 있다.
CREATE TABLE IF NOT EXISTS app_config
(
    config_key   TEXT PRIMARY KEY,
    config_value TEXT NOT NULL
);
