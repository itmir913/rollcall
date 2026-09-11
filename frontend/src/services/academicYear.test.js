/**
 * 학년도 ↔ 달력 연도 변환 테스트.
 *
 * 월 필터가 선택한 달을 날짜로 바꾸는 계산이다. 1·2월을 놓치면 그 두 달의 기록이
 * 한 해 전 날짜로 조회되어 화면이 통째로 비어 보인다.
 */
import {describe, expect, it} from 'vitest'
import {MONTHS, calendarYearOf} from './academicYear'

describe('calendarYearOf', () => {
    it('3~12월은 학년도와 같은 해다', () => {
        expect(calendarYearOf(2026, 3)).toBe(2026)
        expect(calendarYearOf(2026, 12)).toBe(2026)
    })

    it('1·2월은 이듬해다 — 2026학년도 1월은 2027년이다', () => {
        expect(calendarYearOf(2026, 1)).toBe(2027)
        expect(calendarYearOf(2026, 2)).toBe(2027)
    })
})

describe('MONTHS', () => {
    it('3월에 시작해 1월·2월로 끝난다', () => {
        expect(MONTHS[0]).toBe(3)
        expect(MONTHS.slice(-2)).toEqual([1, 2])
    })

    it('열두 달이 한 번씩만 들어 있다', () => {
        expect(new Set(MONTHS).size).toBe(12)
    })

    it('차례대로 이으면 연도가 한 번만 넘어간다', () => {
        // 월 필터를 왼쪽에서 오른쪽으로 누르면 날짜도 같은 방향으로 나아가야 한다.
        const dates = MONTHS.map((m) => calendarYearOf(2026, m) * 100 + m)
        expect(dates).toEqual([...dates].sort((a, b) => a - b))
    })
})
