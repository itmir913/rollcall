-- ================================================================
-- 최초 생성 시 1회만 실행되는 시드 데이터.
--
-- 여기 있는 값은 전부 사용자가 설정 화면에서 바꿀 수 있는 "데이터"다.
-- 학교마다 다른 항목은 기본값만 넣고 규정으로 굳히지 않는다.
--
-- valid_from은 '1900-01-01'이다. 최초 집합에는 시작일이 없다 — 설치일을 넣으면
-- 설치 전 날짜의 출결을 입력할 때 목록이 통째로 비어 버린다.
-- ================================================================

-- ─── 학교 ──────────────────────────────────────────────────────
-- 이름은 설정에서 바꾼다. 최대 교시 7과 제출 기한 7일은 흔한 값일 뿐 규정이 아니다.
INSERT INTO school (name, max_slot, due_days, due_skip_offdays, sort_order, active)
VALUES ('우리 학교', 7, 7, 1, 10, 1);

-- ─── 축 1: 구분 ────────────────────────────────────────────────
INSERT INTO attendance_reason (label, shortcut, sort_order, valid_from)
VALUES ('질병', 'Q', 10, '1900-01-01'),
       ('미인정', 'W', 20, '1900-01-01'),
       ('기타', 'E', 30, '1900-01-01'),
       ('출석인정', 'R', 40, '1900-01-01');

-- ─── 축 2: 종류 ────────────────────────────────────────────────
-- slot_prompt는 그 종류가 물어야 하는 기간이 어느 쪽인지다.
--   결석 = 하루 종일(묻지 않는다) · 지각 = 온 때 하나 · 조퇴 = 나간 때 하나
--   결과 = 빠진 교시 여러 개
INSERT INTO attendance_type (label, slot_prompt, shortcut, sort_order, valid_from)
VALUES ('지각', 'end', 'A', 10, '1900-01-01'),
       ('조퇴', 'start', 'S', 20, '1900-01-01'),
       ('결석', 'none', 'D', 30, '1900-01-01'),
       ('결과', 'multi', 'F', 40, '1900-01-01');

-- ─── 구분 × 종류 = 코드 ────────────────────────────────────────
-- 라벨 규칙(구분+종류)과 문구 패턴으로 만든다.
INSERT INTO attendance_code (reason_id, type_id, label, phrase_pattern, sort_order, valid_from)
SELECT r.id,
       t.id,
       r.label || t.label,
       CASE t.slot_prompt
           WHEN 'none' THEN '{메모}(으)로 ' || r.label || t.label
           WHEN 'start' THEN '{메모}(으)로 {시작교시}부터 ' || r.label || t.label
           WHEN 'end' THEN '{메모}(으)로 {끝교시}까지 ' || r.label || t.label
           ELSE '{메모}(으)로 {시작교시} ' || r.label || t.label
           END,
       r.sort_order + t.sort_order / 10,
       '1900-01-01'
FROM attendance_reason r
         CROSS JOIN attendance_type t;

-- 나이스 표기 → 코드. 라벨과 같은 표기를 기본으로 넣어 둔다.
-- 학교별 표기 차이("질병 조퇴"처럼 띄어쓴 것)는 검증 기능에서 추가한다.
INSERT INTO code_alias (code_id, raw)
SELECT id, label
FROM attendance_code;

-- ─── 태그 ──────────────────────────────────────────────────────
-- 세는 대상의 기본값. 학교마다 부르는 이름이 다르므로 설정에서 고친다.
INSERT INTO span_tag (school_id, name, sort_order, valid_from)
SELECT id, '체험학습', 10, '1900-01-01'
FROM school
UNION ALL
SELECT id, '생리통', 20, '1900-01-01'
FROM school;

-- ─── 한도 규정 ─────────────────────────────────────────────────
-- 흔한 두 가지를 미리 넣는다. 숫자는 학교마다 다르므로 설정에서 고친다.
-- 이 규정은 **입력을 막지 않는다.** 통계 화면에서 세어 알려줄 뿐이다.
INSERT INTO quota_rule (school_id, name, tag_id, period, limit_n, unit, sort_order, valid_from)
SELECT t.school_id, '체험학습 연 20일', t.id, 'year', 20, 'day', 10, '1900-01-01'
FROM span_tag t
WHERE t.name = '체험학습'
UNION ALL
SELECT t.school_id, '생리통 월 1회', t.id, 'month', 1, 'count', 20, '1900-01-01'
FROM span_tag t
WHERE t.name = '생리통';
