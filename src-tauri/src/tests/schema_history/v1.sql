-- ================================================================
-- 출결관리 스키마 v1
--
-- 설계 원칙(CLAUDE.md 참고)
--   · 슬롯을 펼치지 않는다 — 열린 구간은 NULL('*')이고 학사일정/시간표 테이블이 없다.
--   · 코드는 데이터다 — 출결 구분과 체크 항목은 행이지 enum이 아니다.
--   · 구간(absence_span)과 사유(daily_reason)는 별도 테이블이다.
--   · 수정은 마감 후 추가다 — valid_to / active로 마감하고 새 행을 넣는다.
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
CREATE TABLE IF NOT EXISTS student
(
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    year_id        INTEGER NOT NULL REFERENCES academic_year (id) ON DELETE CASCADE,
    grade          INTEGER NOT NULL CHECK (grade >= 1),
    class_no       INTEGER NOT NULL CHECK (class_no >= 1),
    number         INTEGER NOT NULL CHECK (number >= 1),
    name           TEXT    NOT NULL CHECK (name <> ''),
    guardian_phone TEXT,
    enrolled_from  TEXT    NOT NULL,
    enrolled_to    TEXT
);

-- 재학생만 번호가 유일하다. 전출 후 번호 재사용을 허용하기 위한 부분 인덱스다.
CREATE UNIQUE INDEX IF NOT EXISTS ux_student_active
    ON student (year_id, grade, class_no, number)
    WHERE enrolled_to IS NULL;

CREATE INDEX IF NOT EXISTS ix_student_class ON student (year_id, grade, class_no, number);

-- ─── 출결 코드 ─────────────────────────────────────────────────
-- 고치지 않는다. valid_to로 마감하고 새 행을 넣는다.
-- UPDATE하면 그 코드를 참조하는 과거 absence_span의 의미가 소급 변경된다.
CREATE TABLE IF NOT EXISTS attendance_code
(
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    reason         TEXT    NOT NULL, -- 출석인정 | 미인정 | 질병 | 기타
    type           TEXT    NOT NULL CHECK (type IN ('결석', '지각', '조퇴', '결과')),
    label          TEXT    NOT NULL,
    phrase_pattern TEXT,
    default_start  TEXT,             -- NULL = '*'
    default_end    TEXT,             -- NULL = '*'
    shortcut       TEXT,
    sort_order     INTEGER NOT NULL DEFAULT 0,
    valid_from     TEXT    NOT NULL,
    valid_to       TEXT              -- NULL = 현재 유효
);

CREATE INDEX IF NOT EXISTS ix_code_valid ON attendance_code (valid_to, sort_order);

-- 나이스 표기 → 코드 매핑. 대조 기능과 학교별 표기 차이를 흡수한다.
CREATE TABLE IF NOT EXISTS code_alias
(
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    code_id INTEGER NOT NULL REFERENCES attendance_code (id) ON DELETE CASCADE,
    raw     TEXT    NOT NULL UNIQUE
);

-- ─── 부재 구간 (기록) ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS absence_span
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    student_id INTEGER NOT NULL REFERENCES student (id) ON DELETE CASCADE,
    date       TEXT    NOT NULL,
    code_id    INTEGER NOT NULL REFERENCES attendance_code (id),
    start_slot TEXT,                 -- NULL = '*' 처음부터
    end_slot   TEXT,                 -- NULL = '*' 끝까지
    symptom    TEXT,
    group_id   TEXT,                 -- 기간 일괄 입력 묶음 (UUID)
    created_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS ix_span_date ON absence_span (date);
CREATE INDEX IF NOT EXISTS ix_span_student ON absence_span (student_id, date);
CREATE INDEX IF NOT EXISTS ix_span_group ON absence_span (group_id);

-- ─── 날짜별 사유 (나이스 제출용 결론) ──────────────────────────
-- 구간이 여럿이어도 나이스에 넣을 한 줄은 하루에 하나다. 교사가 판단한다.
CREATE TABLE IF NOT EXISTS daily_reason
(
    student_id INTEGER NOT NULL REFERENCES student (id) ON DELETE CASCADE,
    date       TEXT    NOT NULL,
    code_id    INTEGER REFERENCES attendance_code (id),
    reason     TEXT    NOT NULL,
    PRIMARY KEY (student_id, date)
);

-- ─── 체크 항목 (설정) ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS check_item
(
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    due_days        INTEGER,          -- NULL = 마감 없음
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
