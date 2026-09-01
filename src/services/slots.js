/**
 * 슬롯(교시) 상수와 표기. Rust의 `slots.rs`와 같은 순서·같은 규칙이다.
 *
 * 저장과 검증은 Rust가 한다. 여기 있는 것은 화면이 버튼을 그리기 위한 것뿐이다.
 * `slotPrompt`는 `attendance_type` 행이 들고 있는 값이라 화면이 종류 이름을
 * 문자열로 비교하지 않는다.
 */

export const SLOTS = ['조회', '1', '2', '3', '4', '5', '6', '7', '8', '9', '종례']

/** `"5"` → `"5교시"`, `"조회"` → `"조회"`, 비었으면 `"*"` */
export function slotLabel(slot) {
    if (!slot) return '*'
    return slot === '조회' || slot === '종례' ? slot : `${slot}교시`
}

/** 구간 요약 표기. 열린 쪽은 `*`. */
export function formatSpan(startSlot, endSlot) {
    return `${startSlot ?? '*'} ~ ${endSlot ?? '*'}`
}

export function needsStart(slotPrompt) {
    return slotPrompt === 'start' || slotPrompt === 'both'
}

export function needsEnd(slotPrompt) {
    return slotPrompt === 'end' || slotPrompt === 'both'
}

/** 교사에게 물어야 하는 교시가 하나라도 있는가 */
export function needsAnySlot(slotPrompt) {
    return needsStart(slotPrompt) || needsEnd(slotPrompt)
}

/** 시작이 끝보다 뒤인가. 저장 전에 화면에서 먼저 알려주기 위한 것이다. */
export function isReversed(startSlot, endSlot) {
    if (!startSlot || !endSlot) return false
    return SLOTS.indexOf(startSlot) > SLOTS.indexOf(endSlot)
}

/**
 * 구간이 저장 가능한 상태인가. 두 축이 비어 있는 것은 **문제가 아니다** —
 * 그것이 "아직 안 정했다"는 정상 상태이기 때문이다. 여기서 보는 것은 교시뿐이다.
 */
export function slotProblem(slotPrompt, startSlot, endSlot) {
    if (needsStart(slotPrompt) && !startSlot) return '시작 교시를 골라주세요.'
    if (needsEnd(slotPrompt) && !endSlot) return '끝 교시를 골라주세요.'
    if (isReversed(startSlot, endSlot)) return '시작 교시가 끝 교시보다 뒤입니다.'
    return ''
}
