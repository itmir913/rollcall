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
 * 헤더 칸을 비교용으로 정규화한다.
 * 공백·마침표·괄호는 버리고 소문자로 맞춘다. `학 년`, `학년(Grade)`, `GRADE`가
 * 모두 같은 것으로 취급되어야 한다.
 */
export function normalizeHeader(text) {
    return String(text ?? '')
        .replace(/[\s.·・_()[\]{}]/g, '')
        .toLowerCase()
}

/** 정규화한 헤더 → 우리가 아는 열 이름. 모르는 열이면 null. */
export function matchColumn(header) {
    const key = normalizeHeader(header)
    if (!key) return null
    for (const [col, aliases] of Object.entries(ROSTER_COL_ALIASES)) {
        if (aliases.some((a) => normalizeHeader(a) === key)) return col
    }
    return null
}

// 나이스 출결 파일의 열 별칭은 **여기 없다.** `data/neisFormats.json`에 있고
// `services/neisFormat.js`가 읽는다. 명렬표 별칭과 나눠 둔 이유는 갱신 주기가 다르기
// 때문이다 — 명렬표는 교사가 직접 만드는 파일이라 학교마다 다르고 좀처럼 바뀌지 않지만,
// 나이스 서식은 나이스가 바꾼다. 그쪽만 데이터로 빼 두면 앱을 다시 배포하지 않고
// JSON 한 장으로 새 서식에 맞출 수 있다.
