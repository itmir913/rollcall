/**
 * 명렬표 열 자동 인식에 쓰는 헤더 별칭표.
 *
 * **열은 이름으로 찾는다. 위치로 찾지 않는다.** 학교마다 열 순서가 다르고,
 * 쓰지 않는 열이 중간에 끼어 있는 경우도 흔하다. 인덱스로 읽으면 그런 파일에서
 * 조용히 엉뚱한 값이 들어간다 — 번호 자리에 학년이 들어가도 숫자라서 통과한다.
 *
 * 기본 양식은 `학년 · 반 · 번호 · 이름` 네 열이다.
 */
export const ROSTER_COL_ALIASES = {
    grade: ['학년', 'grade'],
    classNo: ['반', '학급', '반번호', 'class', 'classno', 'class_no', 'classnum'],
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
