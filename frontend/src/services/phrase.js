/**
 * 기간을 사람이 읽는 문장으로. Rust `attendance.rs`의 `span_text`와 **같은 문구**여야 한다.
 *
 * 저장된 구간의 문구는 Rust가 만들어 `spanText`로 내려준다. 여기 있는 것은
 * **아직 저장하지 않은 조합**(축 카드의 "현재 선택된 출결")을 위한 것이다.
 * 두 문구가 다르면 교사는 찍기 전과 후에 다른 말을 보게 된다.
 */
import {CLOSING, HOMEROOM, UNKNOWN, groupRuns, slotLabel} from './slots'

/**
 * @param {{slotPrompt: string|null, slots: string[]}} draft
 */
export function spanPhrase({slotPrompt, slots}) {
    if (slotPrompt === 'none') return '하루 종일'
    if (!slots || slots.length === 0) return '기간 미정'

    if (slotPrompt === 'multi') {
        const runs = groupRuns(slots)
        if (runs.length === 0) return '기간 미정'
        return runs.map(([a, b]) => (a === b ? `${a}교시` : `${a}~${b}교시`)).join(' · ')
    }

    const first = slots[0]
    if (slotPrompt === 'end') return `${HOMEROOM}부터 ${slotLabel(first)}까지`
    if (slotPrompt === 'start') return `${slotLabel(first)}부터 ${CLOSING}까지`
    return slotLabel(first)
}

/** 두 축을 사람이 읽는 말로. 비어 있으면 "미정"이다 — 미완성 기록은 정상 상태다. */
export function axisPhrase(reasonLabel, typeLabel) {
    return `${reasonLabel || '미정'} ${typeLabel || '미정'}`
}

/** 축 카드 아래에 적히는 한 줄. */
export function stampPhrase({reasonLabel, typeLabel, slotPrompt, slots}) {
    return `${axisPhrase(reasonLabel, typeLabel)} · ${spanPhrase({slotPrompt, slots})}`
}

/** 열린 쪽 기호를 그대로 노출한다. 화면에서 `*`가 아니라 `?`를 쓴다. */
export {UNKNOWN}
