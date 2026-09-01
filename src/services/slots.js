/**
 * 슬롯(교시) 상수와 표기. Rust의 `slots.rs`와 같은 순서·같은 규칙이다.
 *
 * 저장과 검증은 Rust가 한다. 여기 있는 것은 화면이 버튼을 그리고 미리보기를
 * 만들기 위한 것뿐이다.
 */

export const SLOTS = ['조회', '1', '2', '3', '4', '5', '6', '7', '8', '9', '종례']

/** `"5"` → `"5교시"`, `"조회"` → `"조회"` */
export function slotLabel(slot) {
    if (!slot) return '*'
    return slot === '조회' || slot === '종례' ? slot : `${slot}교시`
}

/** 구간 요약 표기. 열린 쪽은 `*`. */
export function formatSpan(startSlot, endSlot) {
    return `${startSlot ?? '*'} ~ ${endSlot ?? '*'}`
}

/**
 * 코드가 교사에게 물어야 하는 교시의 개수.
 *
 * `slotPrompt`는 Rust가 유형에서 파생해 내려준다(결석 none, 조퇴 start,
 * 지각 end, 결과 both). 화면이 유형을 보고 다시 계산하지 않는다.
 */
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

/** 시작 교시를 물어야 하는가 */
export function needsStart(slotPrompt) {
    return slotPrompt === 'start' || slotPrompt === 'both'
}

/** 끝 교시를 물어야 하는가 */
export function needsEnd(slotPrompt) {
    return slotPrompt === 'end' || slotPrompt === 'both'
}

/** 시작이 끝보다 뒤인가. 저장 전에 화면에서 먼저 알려주기 위한 것이다. */
export function isReversed(startSlot, endSlot) {
    if (!startSlot || !endSlot) return false
    return SLOTS.indexOf(startSlot) > SLOTS.indexOf(endSlot)
}
