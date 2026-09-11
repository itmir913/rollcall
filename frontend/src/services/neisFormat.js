/**
 * 나이스 엑셀 **서식 정의**를 읽어 쓸 수 있는 모양으로 만든다.
 *
 * 서식이 코드가 아니라 **데이터**인 이유는 하나다 — 나이스가 열 이름이나 표기를 바꿨을 때
 * 앱을 다시 빌드해 배포하지 않고 `neisFormats.json` 한 장만 교체하기 위해서다.
 * 그래서 정규식도 문자열로 두고 여기서 컴파일한다. `.js`나 `.ts`에 리터럴로 박으면
 * 빌드 시점에 고정되어 그 길이 막힌다.
 *
 * **나중에 이 파일 하나만 바뀐다.** 지금은 번들에 든 것을 읽지만, 원격에서 받아 오게 되면
 * `load()`가 "받은 것 → 캐시 → 번들" 순으로 고르게 된다. 읽는 쪽(`neisFile.js`)은
 * `spec()`만 부르므로 손댈 것이 없다.
 *
 * **받은 것을 그대로 믿지 않는다.** 어딘가에서 온 JSON이 곧 정규식이 되므로,
 * 모양이 어긋나면 통째로 버리고 번들 것으로 돌아간다. 반쯤 맞는 서식으로 파일을 읽으면
 * 조용히 엉뚱한 값이 들어가는데, 그것이 이 앱에서 가장 비싼 실패다.
 */
import bundled from '../data/neisFormats.json'
import {normalizeHeader} from '../data/columnAliases'

/** 서식에 반드시 있어야 하는 열. 하나라도 없으면 두 서식 중 하나를 읽지 못한다. */
const REQUIRED_COLUMNS = ['date', 'number', 'name', 'code', 'slots', 'detail']

/** 정규식 하나를 컴파일한다. 못 만들면 null — 부른 쪽이 서식 전체를 버린다. */
function compile(pattern) {
    if (typeof pattern !== 'string' || !pattern) return null
    try {
        return new RegExp(pattern)
    } catch {
        return null
    }
}

function isLabelList(value) {
    return Array.isArray(value) && value.length > 0 && value.every((v) => typeof v === 'string' && v)
}

/**
 * 서식 정의를 검사하고 쓸 수 있는 모양으로 바꾼다. 어긋나면 `null`.
 *
 * 돌려주는 것은 **컴파일된 것**이다. 파일 한 줄마다 정규식을 다시 만들면 서른 줄짜리
 * 명렬표에서도 수백 번 만들게 된다.
 */
export function compileSpec(raw) {
    if (!raw || typeof raw !== 'object') return null

    const columns = raw.columns
    if (!columns || typeof columns !== 'object') return null
    if (!REQUIRED_COLUMNS.every((col) => isLabelList(columns[col]))) return null

    const slot = raw.slot
    if (!slot || typeof slot !== 'object') return null
    if (!isLabelList(slot.homeroomLabels) || !isLabelList(slot.closingLabels)) return null
    const period = compile(slot.periodPattern)
    if (!period) return null
    const minPeriod = Number.isInteger(slot.minPeriod) ? slot.minPeriod : 1

    const caption = raw.caption
    if (!caption || !isLabelList(caption.markers)) return null
    const classPattern = compile(caption.classPattern)
    if (!classPattern) return null

    const separator = compile(raw.slotList?.separatorPattern)
    if (!separator) return null

    const datePatterns = (raw.date?.patterns ?? []).map(compile)
    if (!datePatterns.length || datePatterns.some((re) => re === null)) return null

    // 열 별칭은 한 번만 정규화해 둔다. 머리글마다 다시 정규화하면 같은 일을 수백 번 한다.
    const columnIndex = new Map()
    for (const [col, aliases] of Object.entries(columns)) {
        if (!isLabelList(aliases)) return null
        for (const alias of aliases) {
            const key = normalizeHeader(alias)
            // 같은 별칭이 두 열에 있으면 앞의 것을 쓴다. 서식이 잘못 적힌 경우다.
            if (key && !columnIndex.has(key)) columnIndex.set(key, col)
        }
    }

    return {
        version: typeof raw.version === 'string' ? raw.version : '(모름)',
        columnIndex,
        homeroomLabels: slot.homeroomLabels.map(normalizeHeader),
        closingLabels: slot.closingLabels.map(normalizeHeader),
        periodPattern: period,
        minPeriod,
        captionMarkers: caption.markers,
        classPattern,
        separatorPattern: separator,
        datePatterns,
    }
}

/** 번들에 든 서식. 원격 서식이 어긋났을 때 돌아갈 자리다. */
const BUNDLED = compileSpec(bundled)

if (!BUNDLED) {
    // 번들 서식이 깨진 것은 배포 사고다. 조용히 지나가면 나이스 파일 열기가 통째로
    // 죽은 채 앱이 뜬다 — 테스트가 이것을 잡는다.
    throw new Error('번들에 든 나이스 서식 정의가 올바르지 않습니다: neisFormats.json')
}

let current = BUNDLED
let currentSource = 'bundled'

/** 지금 쓰는 서식. 파일을 읽는 쪽은 이것만 부른다. */
export function spec() {
    return current
}

/** 서식이 어디서 왔는지. 값이 이상할 때 어디를 의심할지 알려주는 단서다. */
export function specSource() {
    return {source: currentSource, version: current.version}
}

/**
 * 밖에서 받은 서식으로 교체한다. **어긋나면 교체하지 않고 false를 돌려준다.**
 *
 * 지금은 테스트만 부른다. 원격 갱신을 붙일 때 그 코드가 부를 자리다.
 */
export function useSpec(raw, source = 'remote') {
    const compiled = compileSpec(raw)
    if (!compiled) return false
    current = compiled
    currentSource = source
    return true
}

/** 번들에 든 것으로 되돌린다. */
export function resetSpec() {
    current = BUNDLED
    currentSource = 'bundled'
}

// ── 서식이 답하는 물음들 ──────────────────────────────────────
//
// 아래 함수들이 서식 정의와 파일 읽기 사이의 유일한 통로다. `neisFile.js`는
// 정규식도 별칭표도 직접 보지 않는다 — 서식이 바뀌어도 그쪽은 그대로다.

/** 정규화한 머리글 → 우리가 아는 열 이름. 모르는 열이면 null. */
export function matchNeisColumn(header) {
    const key = normalizeHeader(header)
    if (!key) return null
    return current.columnIndex.get(key) ?? null
}

/**
 * 교시 칸의 글자가 하루의 어느 자리인가. 교시 자리가 아니면 null.
 *
 * 조회 · 종례는 서식이 부르는 이름이고, 돌려주는 것은 이 앱의 토큰이다. 둘을 나눠 두는
 * 이유는 나이스가 `조회`를 `아침조회`로 바꿔도 앱의 토큰은 그대로여야 하기 때문이다.
 */
export function neisSlotToken(text, {homeroom, closing}) {
    const key = normalizeHeader(text)
    if (!key) return null
    if (current.homeroomLabels.includes(key)) return homeroom
    if (current.closingLabels.includes(key)) return closing
    const m = key.match(current.periodPattern)
    if (!m) return null
    const n = Number(m[1])
    return Number.isInteger(n) && n >= current.minPeriod ? String(n) : null
}

/** 교시 표기이긴 한데 이 앱의 하루에 자리가 없는 것(`0교시`)인가. */
export function looksLikePeriod(text) {
    const key = normalizeHeader(text)
    return !!key && current.periodPattern.test(key)
}

/** `조회,1교시,종례,` 를 칸으로 분리한다. 꼬리 쉼표는 나이스가 늘 붙인다. */
export function splitSlotText(text) {
    return String(text ?? '').split(new RegExp(current.separatorPattern, 'g'))
}

/** 캡션 줄인가 — `※`로 시작하는 칸을 찾는다. */
export function captionCellText(cell) {
    const text = String(cell ?? '').trim()
    return current.captionMarkers.some((m) => text.startsWith(m)) ? text : null
}

/** 캡션에서 학년 · 반을 읽는다. 없으면 null. */
export function classInCaption(text) {
    const m = String(text ?? '').match(current.classPattern)
    return m ? {grade: Number(m[1]), classNo: Number(m[2])} : null
}

/** 글자에서 날짜를 읽어 ISO로. 못 읽으면 null. */
export function dateInText(text) {
    const s = String(text ?? '').trim()
    for (const re of current.datePatterns) {
        const m = s.match(re)
        if (m) {
            const [, y, mo, d] = m
            return `${y}-${String(mo).padStart(2, '0')}-${String(d).padStart(2, '0')}`
        }
    }
    return null
}
