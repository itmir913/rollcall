-- ================================================================
-- 출결관리 스키마 v1
--
-- 설계 원칙(CLAUDE.md 참고)
--   · 슬롯을 펼치지 않는다 — 열린 구간은 NULL('*')이고 학사일정/시간표 테이블이 없다.
--   · 코드는 데이터다 — 구분·종류·체크 항목은 행이지 enum이 아니다.
--   · 구간(absence_span)과 사유(daily_reason)는 별도 테이블이다.
--   · 수정은 마감 후 추가다 — valid_to / active로 마감하고 새 행을 넣는다.
--   · **미완성 기록이 정상 상태다** — 구분과 종류는 각각 NULL일 수 있다.
-- ================================================================

-- ─── 학년도 ────────────────────────────────────────────────────
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
CREATE TABLE IF NOT EXISTS student
(
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    year_id       INTEGER NOT NULL REFERENCES academic_year (id) ON DELETE CASCADE,
    grade         INTEGER NOT NULL CHECK (grade >= 1),
    class_no      INTEGER NOT NULL CHECK (class_no >= 1),
    number        INTEGER NOT NULL CHECK (number >= 1),
    name          TEXT    NOT NULL CHECK (name <> ''),
    enrolled_from TEXT    NOT NULL,
    enrolled_to   TEXT
);

-- 재학생만 번호가 유일하다. 전출 후 번호 재사용을 허용하기 위한 부분 인덱스다.
CREATE UNIQUE INDEX IF NOT EXISTS ux_student_active
    ON student (year_id, grade, class_no, number)
    WHERE enrolled_to IS NULL;

CREATE INDEX IF NOT EXISTS ix_student_class ON student (year_id, grade, class_no, number);

-- ─── 연락처 ────────────────────────────────────────────────────
-- 학생 본인·어머니·아버지·조부모… 몇 개든 붙는다. 학생 테이블의 컬럼 하나로는
-- 담을 수 없고, 가정마다 구성이 달라 라벨을 미리 정해둘 수도 없다.
CREATE TABLE IF NOT EXISTS contact
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    student_id INTEGER NOT NULL REFERENCES student (id) ON DELETE CASCADE,
    label      TEXT    NOT NULL CHECK (label <> ''), -- '학생' '어머니' '아버지' … 자유 입력
    value      TEXT    NOT NULL CHECK (value <> ''), -- 전화번호
    note       TEXT,
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
--   end   = 끝만 (지각)           · both  = 양쪽 (결과)
-- 종류가 데이터인 이상 이 속성도 데이터여야 한다. 코드에서 한국어 라벨을
-- 문자열로 비교하면 사용자가 종류를 추가하는 순간 틀린다.
CREATE TABLE IF NOT EXISTS attendance_type
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    label       TEXT    NOT NULL,
    slot_prompt TEXT    NOT NULL CHECK (slot_prompt IN ('none', 'start', 'end', 'both')),
    shortcut    TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    valid_from  TEXT    NOT NULL,
    valid_to    TEXT
);

-- ─── 구분 × 종류 = 코드 ────────────────────────────────────────
-- 두 축이 모두 정해졌을 때만 존재한다. 문구 패턴이 여기 붙는다.
-- 고치지 않는다 — valid_to로 마감하고 새 행을 넣는다. UPDATE하면 이 코드를
-- 참조해 문구를 그리는 과거 기록의 의미가 소급 변경된다.
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

-- 같은 쌍이 동시에 두 개 유효하면 문구를 어느 쪽으로 그릴지 알 수 없다.
CREATE UNIQUE INDEX IF NOT EXISTS ux_code_pair_active
    ON attendance_code (reason_id, type_id)
    WHERE valid_to IS NULL;

-- 나이스 표기 → 코드 매핑. 대조 기능과 학교별 표기 차이를 흡수한다.
CREATE TABLE IF NOT EXISTS code_alias
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    code_id INTEGER NOT NULL REFERENCES attendance_code (id) ON DELETE CASCADE,
    raw     TEXT    NOT NULL UNIQUE
);

-- ─── 부재 구간 (기록) ──────────────────────────────────────────
-- reason_id와 type_id는 **각각 NULL일 수 있다.** NULL은 "아직 안 정했다"는 뜻이다.
--
-- 학생이 안 왔는데 연락이 닿지 않으면 그날은 날짜와 학생만 남긴다. 웹앱이 결석
-- 번호만 비트마스크로 넘겨주는 경로도 마찬가지다 — 두 축이 다 비어 들어온 뒤
-- 데스크톱에서 채워진다. 미완성 기록은 예외가 아니라 정상 상태다.
CREATE TABLE IF NOT EXISTS absence_span
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    student_id INTEGER NOT NULL REFERENCES student (id) ON DELETE CASCADE,
    date       TEXT    NOT NULL,
    reason_id  INTEGER REFERENCES attendance_reason (id), -- NULL = 미정
    type_id    INTEGER REFERENCES attendance_type (id),   -- NULL = 미정
    start_slot TEXT,                                      -- NULL = '*' 처음부터
    end_slot   TEXT,                                      -- NULL = '*' 끝까지
    symptom    TEXT,
    group_id   TEXT,                                      -- 기간 일괄 입력 묶음 (UUID)
    created_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS ix_span_date ON absence_span (date);
CREATE INDEX IF NOT EXISTS ix_span_student ON absence_span (student_id, date);
CREATE INDEX IF NOT EXISTS ix_span_group ON absence_span (group_id);

-- 아직 채워지지 않은 기록을 찾는 질의가 자주 나온다.
CREATE INDEX IF NOT EXISTS ix_span_incomplete
    ON absence_span (date)
    WHERE reason_id IS NULL OR type_id IS NULL;

-- ─── 날짜별 사유 (나이스 제출용 결론) ──────────────────────────
-- 구간이 여럿이어도 나이스에 넣을 한 줄은 하루에 하나다. 교사가 판단한다.
-- 대표 축도 미정일 수 있다 — 사유 문구만 먼저 적어두는 경우가 있다.
CREATE TABLE IF NOT EXISTS daily_reason
(
    student_id INTEGER NOT NULL REFERENCES student (id) ON DELETE CASCADE,
    date       TEXT    NOT NULL,
    reason_id  INTEGER REFERENCES attendance_reason (id),
    type_id    INTEGER REFERENCES attendance_type (id),
    reason     TEXT    NOT NULL,
    PRIMARY KEY (student_id, date)
);

-- ─── 체크 항목 (설정) ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS check_item
(
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    due_days        INTEGER, -- NULL = 마감 없음
    include_weekend INTEGER NOT NULL DEFAULT 0 CHECK (include_weekend IN (0, 1)),
    default_done    INTEGER NOT NULL DEFAULT 0 CHECK (default_done IN (0, 1)),
    sort_order      INTEGER NOT NULL DEFAULT 0,
    active          INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1))
);

-- ─── 하루 단위 체크 ────────────────────────────────────────────
-- 행 구조다. 항목마다 컬럼을 두면 추가할 때마다 마이그레이션이 필요하다.
CREATE TABLE IF NOT EXISTS daily_check
(
    student_id INTEGER NOT NULL REFERENCES student (id) ON DELETE CASCADE,
    date       TEXT    NOT NULL,
    item_id    INTEGER NOT NULL REFERENCES check_item (id) ON DELETE CASCADE,
    done       INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
    due_date   TEXT,
    done_at    TEXT,
    PRIMARY KEY (student_id, date, item_id)
);

CREATE INDEX IF NOT EXISTS ix_check_due ON daily_check (done, due_date);

-- ─── 앱 설정 ───────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS app_config
(
    config_key   TEXT PRIMARY KEY,
    config_value TEXT NOT NULL
);
