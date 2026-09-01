import {describe, expect, it} from 'vitest'
import {
    formatSpan, isReversed, needsAnySlot, needsEnd, needsStart, SLOTS, slotLabel, slotProblem,
} from './slots.js'

describe('슬롯', () => {
    it('조회가 처음이고 종례가 끝이다', () => {
        expect(SLOTS[0]).toBe('조회')
        expect(SLOTS[SLOTS.length - 1]).toBe('종례')
    })

    it('숫자 교시에만 "교시"를 붙인다', () => {
        expect(slotLabel('5')).toBe('5교시')
        expect(slotLabel('조회')).toBe('조회')
    })

    it('열린 쪽은 *로 쓴다', () => {
        expect(slotLabel(null)).toBe('*')
        expect(formatSpan(null, null)).toBe('* ~ *')
        expect(formatSpan('5', null)).toBe('5 ~ *')
    })

    it('어느 쪽을 물을지는 slotPrompt가 정한다', () => {
        expect(needsStart('start')).toBe(true)
        expect(needsEnd('start')).toBe(false)
        expect(needsEnd('end')).toBe(true)
        expect(needsStart('both')).toBe(true)
        expect(needsEnd('both')).toBe(true)
        expect(needsAnySlot('none')).toBe(false)
        expect(needsAnySlot('both')).toBe(true)
    })

    it('역순 구간을 저장 전에 알아챈다', () => {
        expect(isReversed('6', '3')).toBe(true)
        expect(isReversed('4', '4')).toBe(false)
        expect(isReversed(null, '3')).toBe(false)
    })
})

describe('slotProblem', () => {
    it('축이 비어 있는 것은 문제가 아니다', () => {
        // "아직 안 정했다"는 정상 상태다. 교시만 본다.
        expect(slotProblem('none', null, null)).toBe('')
        expect(slotProblem(undefined, null, null)).toBe('')
    })

    it('종류가 요구하는 교시가 비면 알려준다', () => {
        expect(slotProblem('start', null, null)).toContain('시작 교시')
        expect(slotProblem('end', null, null)).toContain('끝 교시')
        expect(slotProblem('both', '3', null)).toContain('끝 교시')
    })

    it('역순이면 알려준다', () => {
        expect(slotProblem('both', '6', '3')).toContain('시작 교시가 끝 교시보다 뒤')
    })

    it('다 채워지면 문제가 없다', () => {
        expect(slotProblem('start', '5', null)).toBe('')
        expect(slotProblem('both', '3', '6')).toBe('')
    })
})
