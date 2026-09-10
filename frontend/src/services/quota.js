/**
 * 한도 표를 그리기 위한 순수 계산.
 *
 * 세는 것은 Rust가 한다. 여기 있는 것은 **이미 센 결과를 칸에 놓는 규칙**뿐이다.
 */

/** 그 달에 몇 번 썼는가. Rust가 채워 준 버킷(`2026-06`)에서 읽는다. */
export function countOf(row, month) {
    const key = String(month).padStart(2, '0')
    return row.buckets?.find((b) => b.key.endsWith(`-${key}`))?.count ?? 0
}

/** 달 칸의 색. 한도를 넘긴 달만 붉다. */
export function monthClass(row, month, limitN) {
    const count = countOf(row, month)
    if (count === 0) return ''
    return count > limitN ? 'is-over' : 'is-used'
}

/**
 * 한도 칸. 쓴 날은 날짜를 담고 남은 자리는 비운다.
 *
 * 한도보다 많이 쓴 경우에도 쓴 날을 **버리지 않는다** — 프로그램은 판정하지 않고,
 * 넘긴 날도 기록으로 남아야 한다.
 */
export function quotaCells(row) {
    const length = Math.max(row.limitN, row.dates?.length ?? 0)
    return Array.from({length}, (_, i) => row.dates?.[i] ?? null)
}

/** 남은 수를 사람이 읽는 말로. */
export function remainLabel(row) {
    if (row.state === 'over') return '소진'
    const left = row.limitN - row.used
    return `${left} 남음`
}
