/**
 * 명렬표 열 자동 인식에 쓰는 헤더 별칭표.
 *
 * **열은 이름으로 찾는다. 위치로 찾지 않는다.** 학교마다 열 순서가 다르고,
 * 쓰지 않는 열이 중간에 끼어 있는 경우도 흔하다. 인덱스로 읽으면 그런 파일에서
 * 조용히 엉뚱한 값이 들어간다 — 번호 자리에 학년이 들어가도 숫자라서 통과한다.
 *
 * 기본 양식은 `학년 · 반 · 번호 · 이름` 네 열이다.
 *
 * 교과 강좌 명렬표는 여러 반이 섞여 있어 반 열의 이름이 `원반` · `소속반`인 경우가 있다.
 * 담임 학급을 뜻하는 같은 값이므로 `classNo`로 받는다.
 */
export const ROSTER_COL_ALIASES = {
    grade: ['학년', 'grade'],
    classNo: ['반', '학급', '반번호', '원반', '소속반', 'class', 'classno', 'class_no', 'classnum'],
    number: ['번호', '번', '출석번호', 'number', 'no', 'num'],
    name: ['이름', '성명', '학생명', '학생이름', 'name'],
}

/** 없으면 명렬표로 쓸 수 없는 열. */
export const REQUIRED_COLS = ['number', 'name']

export const COL_LABELS = {
    grade: '학년',
    classNo: '반',
    number: '번호',
    name: '이름',
}

/**
 * 헤더 칸을 비교용으로 다듬는다.
 * 공백·마침표·괄호는 버리고 소문자로 맞춘다. `학 년`, `학년(Grade)`, `GRADE`가
 * 모두 같은 것으로 취급되어야 한다.
 */
export function normalizeHeader(text) {
    return String(text ?? '')
        .replace(/[\s.·・_()[\]{}]/g, '')
        .toLowerCase()
}

/** 다듬은 헤더 → 우리가 아는 열 이름. 모르는 열이면 null. */
export function matchColumn(header) {
    const key = normalizeHeader(header)
    if (!key) return null
    for (const [col, aliases] of Object.entries(ROSTER_COL_ALIASES)) {
        if (aliases.some((a) => normalizeHeader(a) === key)) return col
    }
    return null
}

/**
 * 나이스 출결 파일의 열 별칭. 명렬표와 같은 이유로 **이름으로 찾는다.**
 *
 * 2026-09-11에 실제로 내려받은 두 파일에서 확인한 머리글이 기준이다.
 *   · 일일출석부   `번호 · 성명 · 마감 · 조회 · 1교시 … · 종례 · 비고`
 *   · 월별 출결 현황 `일자 · 번호 · 성명 · 출결구분 · 결시교시 · 사유`
 *
 * `마감`이 출결 코드 열인 것은 오타가 아니다 — 일일출석부는 **마감된 출결**을
 * 그 열에 적는다. 뜻을 짐작해 고치면 실제 파일이 읽히지 않는다.
 *
 * 교시 열(`조회` · `3교시` · `종례`)은 여기 두지 않는다. 학교 설정에 따라 개수가
 * 달라지므로 별칭표가 아니라 `slotTokenOf`가 모양으로 알아본다.
 */
export const NEIS_COL_ALIASES = {
    date: ['일자', '날짜', '출결일자', '결석일'],
    number: ['번호', '학번', '출석번호'],
    name: ['성명', '이름', '학생명'],
    code: ['출결구분', '출결내용', '출결상황', '출결', '마감'],
    slots: ['결시교시', '결과교시', '해당교시'],
    detail: ['사유', '비고', '내용', '특기사항'],
}

/** 다듬은 헤더 → 나이스 열 이름. 모르는 열이면 null. */
export function matchNeisColumn(header) {
    const key = normalizeHeader(header)
    if (!key) return null
    for (const [col, aliases] of Object.entries(NEIS_COL_ALIASES)) {
        if (aliases.some((a) => normalizeHeader(a) === key)) return col
    }
    return null
}
