/**
 * 한도 표와 마감 표기.
 *
 * 세는 것은 Rust가 하고, 여기서는 **이미 센 결과를 칸에 놓는 규칙**만 본다.
 * 칸이 중요한 이유는 손으로 적던 표를 그대로 옮긴 것이기 때문이다 —
 * 숫자로 `15/20`만 적으면 남은 수를 매번 빼야 한다.
 */
import {describe, expect, it} from 'vitest'
import {countOf, monthClass, quotaCells, remainLabel} from './quota'
import {overdueLabel, overdueTone} from './overdue'

const row = (over = {}) => ({
    studentId: 1, number: 5, name: '김하늘',
    used: 3, limitN: 20, dates: ['2026-03-02', '2026-04-01', '2026-05-06'],
    buckets: [], state: 'ok', ...over,
})

describe('한도 칸', () => {
    it('칸은 한도만큼 그리고 쓴 날만 채운다', () => {
        const cells = quotaCells(row())
        expect(cells).toHaveLength(20)
        expect(cells.filter(Boolean)).toHaveLength(3)
    })

    it('한도를 넘겨도 쓴 날을 버리지 않는다 — 넘긴 날도 기록이다', () => {
        const cells = quotaCells(row({limitN: 2, used: 3}))
        expect(cells).toHaveLength(3)
    })

    it('남은 수를 사람이 읽는 말로 적는다', () => {
        expect(remainLabel(row({used: 18}))).toBe('2 남음')
        expect(remainLabel(row({state: 'over', used: 20}))).toBe('소진')
    })
})

describe('달 격자', () => {
    const monthly = row({
        limitN: 1,
        buckets: [
            {key: '2026-03', label: '3월', count: 1},
            {key: '2026-06', label: '6월', count: 2},
        ],
    })

    it('그 달에 쓴 수를 버킷에서 읽는다', () => {
        expect(countOf(monthly, 3)).toBe(1)
        expect(countOf(monthly, 6)).toBe(2)
        expect(countOf(monthly, 4)).toBe(0)
    })

    it('한도를 넘긴 달만 붉다', () => {
        expect(monthClass(monthly, 3, 1)).toBe('is-used')
        expect(monthClass(monthly, 6, 1)).toBe('is-over')
        expect(monthClass(monthly, 4, 1)).toBe('')
    })
})

describe('마감 표기', () => {
    it('행동이 달라지는 경계를 말로 적는다', () => {
        expect(overdueLabel({daysOverdue: 3})).toBe('3일 지남')
        expect(overdueLabel({daysOverdue: 0})).toBe('오늘까지')
        expect(overdueLabel({daysOverdue: -1})).toBe('내일')
        expect(overdueLabel({daysOverdue: -5})).toBe('5일 남음')
    })

    it('이미 받은 서류는 마감을 세지 않는다', () => {
        expect(overdueLabel({docDone: true, daysOverdue: 9})).toBe('제출')
        expect(overdueTone({docDone: true})).toBe('is-ok')
    })

    it('마감이 없는 건은 재촉할 근거가 없다', () => {
        expect(overdueLabel({daysOverdue: null})).toBe('—')
        expect(overdueTone({daysOverdue: null})).toBe('is-calm')
    })

    it('지난 것은 붉고, 내일까지는 주황이다', () => {
        expect(overdueTone({daysOverdue: 1})).toBe('is-bad')
        expect(overdueTone({daysOverdue: -1})).toBe('is-warn')
        expect(overdueTone({daysOverdue: -7})).toBe('is-calm')
    })
})
