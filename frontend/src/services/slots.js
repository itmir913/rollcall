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
 * 그 종류에서 고를 수 있는 기간.
 *
 * 결석은 하루 종일이라 고를 것이 없고, 지각은 온 때 하나, 조퇴는 나간 때 하나,
 * 결과는 빠진 교시 여러 개(조회·종례·?는 아니다). 종류가 미정이면 교시를 전부 열어 둔다.
 *
 * **지각은 조회를 고를 수 있다.** 지각은 `조회부터 N교시까지`이고, 조회만 놓친 날은
 * 그 N이 조회다. 나이스 실파일에도 `질병지각 · 결시교시 조회,`가 있었다.
 * 조퇴에서 종례를 닫아 두는 것은 그대로다 — 그쪽은 실무에서 쓰는 것을 아직 보지 못했다.
 */
export function allowedSlots(slotPrompt, maxSlot) {
    const periods = slotList(maxSlot).slice(1, -1)
    switch (slotPrompt) {
        case 'none':
            return []
        case 'end':
            return [HOMEROOM, ...periods, UNKNOWN, CLOSING]
        case 'start':
            return [HOMEROOM, ...periods, UNKNOWN]
        case 'multi':
            return periods
        default:
            // 종류가 미정이면 `?`를 열지 않는다. `?`는 **묻고 있는 쪽이 열려 있다**는
            // 뜻인데, 종류가 없으면 어느 쪽을 묻는지 정해지지 않아 저장할 수 없다.
            return [HOMEROOM, ...periods, CLOSING]
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

/** 고른 기간이 그 종류에서 여전히 쓸 수 있는가. 못 쓰면 화면이 미정으로 되돌린다. */
export function keepUsable(slots, slotPrompt, maxSlot) {
    const allowed = allowedSlots(slotPrompt, maxSlot)
    const kept = slots.filter((v) => allowed.includes(v))
    return isMulti(slotPrompt) ? kept : kept.slice(0, 1)
}

/**
 * 저장된 구간을 버튼 선택으로 되돌린다. Rust `ranges_for`의 역이다.
 *
 * **아무것도 건드리지 않고 저장해도 기간이 바뀌지 않아야 한다.** 수정 모달은 이 목록을
 * 그대로 다시 보내므로, 되돌리지 못한 값을 빈 목록으로 내놓으면 모달을 열었다 저장만
 * 해도 기간이 미정으로 날아간다. 그래서 결과(multi)는 표현할 수 없는 끝(조회 · 종례)을
 * 버리지 않고 **고를 수 있는 교시 범위로 좁힌다** — 경계 하나가 옮겨지는 것이
 * 기간 전체가 사라지는 것보다 낫다.
 *
 * 열린 쪽(NULL)은 `?`로 되돌린다. 그러지 않으면 `조회부터 ?까지`로 저장된 지각이
 * 모달에서 기간 미정으로 보인다.
 */
export function picksOf(span, maxSlot) {
    const prompt = span?.slotPrompt ?? null
    const start = span?.startSlot ?? null
    const end = span?.endSlot ?? null

    if (prompt === 'none') return []
    if (!start && !end) return []
    if (prompt === 'end') return end ? [end] : [UNKNOWN]
    if (prompt === 'start') return start ? [start] : [UNKNOWN]
    if (prompt === 'multi') {
        if (!start || !end) return []
        const a = slotOrder(start, maxSlot)
        const b = slotOrder(end, maxSlot)
        if (a < 0 || b < 0) return []
        const from = Math.max(1, a)
        const to = Math.min(maxSlot, b)
        if (to < from) return []
        const out = []
        for (let n = from; n <= to; n += 1) out.push(String(n))
        return out
    }
    return start ? [start] : []
}
