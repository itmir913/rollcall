/**
 * 기한까지 남은 날을 사람이 읽는 말로.
 *
 * 숫자만 적으면 교사가 매번 오늘 날짜에서 빼야 한다. "3일 지남"과 "내일"은
 * 같은 정보를 다르게 말하는 것이 아니라, **행동이 달라지는 경계**를 말해 준다.
 *
 * `daysOverdue`는 Rust가 센다(양수면 지났고, 음수면 남았다). 이미 받은 서류는
 * 기한을 세지 않으므로 null로 온다.
 */
export function overdueLabel(span) {
    if (span.docDone) return '제출'
    const days = span.daysOverdue
    if (days === null || days === undefined) return '—'
    if (days > 0) return `${days}일 지남`
    if (days === 0) return '오늘까지'
    if (days === -1) return '내일'
    return `${-days}일 남음`
}

/** 그 줄의 색조. 기한이 지났으면 붉게, 내일이면 주황, 아니면 조용히. */
export function overdueTone(span) {
    if (span.docDone) return 'is-ok'
    const days = span.daysOverdue
    if (days === null || days === undefined) return 'is-calm'
    if (days > 0) return 'is-bad'
    if (days >= -1) return 'is-warn'
    return 'is-calm'
}
