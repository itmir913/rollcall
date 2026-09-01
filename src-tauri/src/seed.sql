-- ================================================================
-- 최초 생성 시 1회만 실행되는 시드 데이터.
--
-- 여기 있는 값은 전부 사용자가 설정 화면에서 바꿀 수 있는 "데이터"다.
-- 리로스쿨·하이클래스처럼 학교마다 다른 항목은 넣지 않는다.
--
-- valid_from은 '1900-01-01'이다. 최초 집합에는 시작일이 없다 — 설치일을 넣으면
-- 설치 전 날짜의 출결을 입력할 때 목록이 통째로 비어 버린다.
-- ================================================================

-- ─── 축 1: 구분 ────────────────────────────────────────────────
INSERT INTO attendance_reason (label, shortcut, sort_order, valid_from)
VALUES ('질병', 'Q', 10, '1900-01-01'),
       ('출석인정', 'W', 20, '1900-01-01'),
       ('미인정', 'E', 30, '1900-01-01'),
       ('기타', 'R', 40, '1900-01-01');

-- ─── 축 2: 종류 ────────────────────────────────────────────────
-- slot_prompt는 3.1의 구간 표에서 그대로 온다.
--   결석 `* ~ *` · 조퇴 `N ~ *` · 지각 `* ~ N` · 결과 `N ~ M`
INSERT INTO attendance_type (label, slot_prompt, shortcut, sort_order, valid_from)
VALUES ('결석', 'none', 'A', 10, '1900-01-01'),
       ('지각', 'end', 'S', 20, '1900-01-01'),
       ('조퇴', 'start', 'D', 30, '1900-01-01'),
       ('결과', 'both', 'F', 40, '1900-01-01');

-- ─── 구분 × 종류 = 코드 ────────────────────────────────────────
-- 4 × 4 = 16행을 라벨 규칙(구분+종류)과 문구 패턴으로 만든다.
-- 문구 패턴은 종류의 slot_prompt에 맞춰 붙인다.
INSERT INTO attendance_code (reason_id, type_id, label, phrase_pattern, sort_order, valid_from)
SELECT r.id,
       t.id,
       r.label || t.label,
       CASE t.slot_prompt
           WHEN 'none' THEN '{증상}(으)로 ' || r.label || t.label
           WHEN 'start' THEN '{증상}(으)로 {시작교시}부터 ' || r.label || t.label
           WHEN 'end' THEN '{증상}(으)로 {끝교시}까지 ' || r.label || t.label
           ELSE '{증상}(으)로 {시작교시} ' || r.label || t.label
           END,
       r.sort_order + t.sort_order / 10,
       '1900-01-01'
FROM attendance_reason r
         CROSS JOIN attendance_type t;

-- 나이스 표기 → 코드. 라벨과 같은 표기를 기본으로 넣어 둔다.
-- 학교별 표기 차이("질병 조퇴" 처럼 띄어쓴 것)는 대조 기능에서 추가한다.
INSERT INTO code_alias (code_id, raw)
SELECT id, label
FROM attendance_code;

-- ─── 체크 항목 ─────────────────────────────────────────────────
-- 마감 일수는 학교마다 다르다. 5일은 흔한 값일 뿐 규정이 아니다.
INSERT INTO check_item (name, due_days, include_weekend, default_done, sort_order, active)
VALUES ('나이스 입력 완료', NULL, 0, 0, 10, 1),
       ('증빙 서류 제출 완료', 5, 0, 0, 20, 1);
