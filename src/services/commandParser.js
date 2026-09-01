/**
 * 빠른 입력 한 줄을 해석한다.
 *
 *   "2 q 몸살"      → 2번, 질병결석(Q), 증상 "몸살"
 *   "5 w 6 복통"    → 5번, 질병조퇴(W), 6교시부터, 증상 "복통"
 *
 * 토큰 순서는 [번호] [단축키] [교시…] [증상…]으로 고정이다. 코드가 교시를
 * 몇 개 요구하는지는 `slotPrompt`가 정하므로 파싱이 결정적이다 — 번호와 교시가
 * 둘 다 숫자여도 자리로 구분된다.
 *
 * 이 함수는 UI 입력 해석이므로 프론트엔드에 있다. 저장 규칙과 문구 생성은
 * Rust에 있고 여기서 흉내내지 않는다.
 */

const SLOT_ALIASES = { 조회: '조회', 종례: '종례', 조: '조회', 종: '종례' }

/** 앱 상수. Rust의 slots.rs와 같은 순서다. */
export const SLOTS = ['조회', '1', '2', '3', '4', '5', '6', '7', '8', '9', '종례']

export function normalizeSlot(token) {
    if (!token) return null
    const alias = SLOT_ALIASES[token]
    if (alias) return alias
    return SLOTS.includes(token) ? token : null
}

export function slotCount(slotPrompt) {
    switch (slotPrompt) {
        case 'start':
        case 'end':
            return 1
        case 'both':
            return 2
        default:
            return 0
    }
}

/**
 * @param {string} line 사용자가 친 한 줄
 * @param {Array} codes get_codes 결과
 * @returns {{ok: boolean, error?: string, number?: number, code?: object,
 *            startSlot?: string|null, endSlot?: string|null, symptom?: string|null,
 *            needs?: string}}
 */
export function parseCommand(line, codes) {
    const tokens = String(line || '').trim().split(/\s+/).filter(Boolean)
    if (tokens.length === 0) return {ok: false, needs: 'number'}

    const number = Number(tokens[0])
    if (!Number.isInteger(number) || number < 1) {
        return {ok: false, error: `번호가 아닙니다: ${tokens[0]}`}
    }
    if (tokens.length === 1) return {ok: false, number, needs: 'code'}

    const key = tokens[1].toUpperCase()
    const code = codes.find((c) => (c.shortcut || '').toUpperCase() === key)
    if (!code) return {ok: false, number, error: `단축키가 없습니다: ${tokens[1]}`}

    const need = slotCount(code.slotPrompt)
    const slotTokens = tokens.slice(2, 2 + need)
    if (slotTokens.length < need) {
        return {ok: false, number, code, needs: 'slot'}
    }

    const slots = slotTokens.map(normalizeSlot)
    if (slots.some((s) => s === null)) {
        return {ok: false, number, code, error: `교시가 아닙니다: ${slotTokens.join(' ')}`}
    }

    let startSlot = code.defaultStart ?? null
    let endSlot = code.defaultEnd ?? null
    if (code.slotPrompt === 'start') startSlot = slots[0]
    else if (code.slotPrompt === 'end') endSlot = slots[0]
    else if (code.slotPrompt === 'both') {
        startSlot = slots[0]
        endSlot = slots[1]
    }

    if (startSlot && endSlot && SLOTS.indexOf(startSlot) > SLOTS.indexOf(endSlot)) {
        return {ok: false, number, code, error: '시작 교시가 끝 교시보다 뒤입니다.'}
    }

    const symptom = tokens.slice(2 + need).join(' ') || null

    return {ok: true, number, code, startSlot, endSlot, symptom}
}

/** 구간 요약 표기. Rust의 format_span과 같은 규칙이다. */
export function formatSpan(startSlot, endSlot) {
    return `${startSlot ?? '*'} ~ ${endSlot ?? '*'}`
}
