/**
 * 기간을 사람이 읽는 문장으로. Rust `attendance.rs`의 `span_text`와 **같은 문구**여야 한다.
 *
 * 저장된 구간의 문구는 Rust가 만들어 `spanText`로 내려준다. 여기 있는 것은
 * **아직 저장하지 않은 조합**(축 카드의 "현재 선택된 출결")을 위한 것이다.
 * 두 문구가 다르면 교사는 입력 전과 후에 다른 말을 보게 된다.
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
        // 저장되면 묶음 하나가 구간 하나가 되고, Rust가 그것을 `1교시부터 3교시까지`로
        // 적는다. 여기서 `1~3교시`라고 하면 입력 전과 후에 다른 말을 보게 된다.
        return runs
            .map(([a, b]) => (a === b ? `${a}교시` : `${a}교시부터 ${b}교시까지`))
            .join(' · ')
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

/**
 * **저장된** 구간의 문구. Rust `span_text`와 같은 말을 쓴다.
 *
 * `spanPhrase`는 아직 저장하지 않은 조합(고른 교시 목록)을 받고, 이것은 이미 저장된
 * 두 끝(start · end)을 받는다. 입력이 달라 함수가 둘이지만 **문구는 하나여야 한다** —
 * 나이스 파일에서 읽은 구간을 내 기록 옆에 나란히 놓는 화면에서, 같은 기간이
 * 다른 말로 적히면 교사는 그것을 다름으로 읽는다.
 */
export function spanTextOf(slotPrompt, start, end) {
    if (slotPrompt === 'none') return '하루 종일'
    if (!start && !end) return '기간 미정'
    // 두 끝이 같으면 슬롯 하나다. 나이스 실파일에 결시교시가 `조회,` 하나뿐인 지각이 있었다.
    if (start && start === end) return slotLabel(start)
    if (slotPrompt === 'end') {
        return `${slotLabel(start ?? HOMEROOM)}부터 ${slotLabel(end ?? UNKNOWN)}까지`
    }
    if (slotPrompt === 'start') {
        return `${slotLabel(start ?? UNKNOWN)}부터 ${slotLabel(end ?? CLOSING)}까지`
    }
    return `${slotLabel(start ?? UNKNOWN)}부터 ${slotLabel(end ?? UNKNOWN)}까지`
}
