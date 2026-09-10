/**
 * 교시(슬롯) 규칙. Rust의 `slots.rs`와 **같은 순서, 같은 규칙**이다.
 *
 * 두 구현이 갈라지면 조용히 엉뚱한 기간이 저장된다. 그래서 이 파일과 slots.rs는
 * 같은 표를 보고 만들었고, 양쪽에 같은 경계 테스트를 둔다.
 *
 * 하루는 `조회 · 1교시 … N교시 · 종례`로 이어진다. **N은 학교 설정값이다** —
 * 앱 상수가 아니라 학교가 들고 있는 값이라 함수마다 인자로 받는다.
 */

export const HOMEROOM = '조회'
export const CLOSING = '종례'

/** 열린 쪽을 화면에 적을 때 쓰는 기호. 저장은 NULL이다. */
export const UNKNOWN = '?'

/** 기간을 아직 정하지 않은 상태. 저장은 양쪽 NULL이다. */
export const UNDECIDED = '미정'

/** 조회 < 1교시 < … < N교시 < 종례 */
export function slotList(maxSlot) {
    const out = [HOMEROOM]
    for (let n = 1; n <= maxSlot; n += 1) out.push(String(n))
    out.push(CLOSING)
    return out
}

/** 화면 표기. `"5"` → `"5교시"`, `"조회"` → `"조회"` */
export function slotLabel(slot) {
    if (slot === HOMEROOM || slot === CLOSING || slot === UNKNOWN) return slot
    return `${slot}교시`
}

/**
 * 그 구분에서 고를 수 있는 기간.
 *
 * 결석은 하루 종일이라 고를 것이 없고, 지각은 온 때 하나(조회일 수 없다),
 * 조퇴는 나간 때 하나(종례일 수 없다), 결과는 빠진 교시 여러 개(조회·종례·?는 아니다).
 * 구분이 미정이면 전부 열어 둔다.
 */
export function allowedSlots(slotPrompt, maxSlot) {
    const periods = slotList(maxSlot).slice(1, -1)
    switch (slotPrompt) {
        case 'none':
            return []
        case 'end':
            return [...periods, UNKNOWN, CLOSING]
        case 'start':
            return [HOMEROOM, ...periods, UNKNOWN]
        case 'multi':
            return periods
        default:
            return [HOMEROOM, ...periods, UNKNOWN, CLOSING]
    }
}

/** 결과만 여러 교시를 고른다. */
export function isMulti(slotPrompt) {
    return slotPrompt === 'multi'
}

/** 슬롯 토큰의 순서값. 모르는 토큰이면 -1. */
export function slotOrder(slot, maxSlot) {
    if (slot === HOMEROOM) return 0
    if (slot === CLOSING) return maxSlot + 1
    const n = Number(slot)
    return Number.isInteger(n) && n >= 1 && n <= maxSlot ? n : -1
}

/**
 * 이어진 교시끼리 묶는다. `[1,2,3]` → `[[1,3]]`, `[1,3,5]` → 세 구간.
 *
 * 이어지지 않은 것을 한 구간으로 저장하면 2교시가 조용히 포함된다.
 */
export function groupRuns(slots) {
    // 중복을 먼저 지운다. 같은 교시가 두 번 들어오면 다음 칸과 이어붙지 못해
    // 한 구간이 둘로 갈라진다(1,2,2,3 → 1~2 와 2~3).
    const nums = [...new Set(slots.map(Number).filter((n) => Number.isInteger(n)))].sort(
        (a, b) => a - b,
    )
    const out = []
    for (const n of nums) {
        const last = out[out.length - 1]
        if (last && n === last[1] + 1) last[1] = n
        else out.push([n, n])
    }
    return out
}

/** 고른 기간이 그 구분에서 여전히 쓸 수 있는가. 못 쓰면 화면이 미정으로 되돌린다. */
export function keepUsable(slots, slotPrompt, maxSlot) {
    const allowed = allowedSlots(slotPrompt, maxSlot)
    const kept = slots.filter((v) => allowed.includes(v))
    return isMulti(slotPrompt) ? kept : kept.slice(0, 1)
}
