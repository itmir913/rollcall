import {describe, expect, it} from 'vitest'
import {formatSpan, isReversed, needsEnd, needsStart, SLOTS, slotCount, slotLabel} from './slots.js'

describe('슬롯', () => {
    it('조회가 처음이고 종례가 끝이다', () => {
        expect(SLOTS[0]).toBe('조회')
        expect(SLOTS[SLOTS.length - 1]).toBe('종례')
    })

    it('숫자 교시에만 "교시"를 붙인다', () => {
        expect(slotLabel('5')).toBe('5교시')
        expect(slotLabel('조회')).toBe('조회')
        expect(slotLabel('종례')).toBe('종례')
    })

    it('열린 쪽은 *로 쓴다', () => {
        expect(slotLabel(null)).toBe('*')
        expect(formatSpan(null, null)).toBe('* ~ *')
        expect(formatSpan('5', null)).toBe('5 ~ *')
        expect(formatSpan(null, '4')).toBe('* ~ 4')
    })

    it('물어야 하는 교시 수는 slotPrompt가 정한다', () => {
        expect(slotCount('none')).toBe(0)
        expect(slotCount('start')).toBe(1)
        expect(slotCount('end')).toBe(1)
        expect(slotCount('both')).toBe(2)
    })

    it('어느 쪽을 물을지도 slotPrompt가 정한다', () => {
        expect(needsStart('start')).toBe(true)
        expect(needsEnd('start')).toBe(false)
        expect(needsStart('end')).toBe(false)
        expect(needsEnd('end')).toBe(true)
        expect(needsStart('both')).toBe(true)
        expect(needsEnd('both')).toBe(true)
        expect(needsStart('none')).toBe(false)
        expect(needsEnd('none')).toBe(false)
    })

    it('역순 구간을 저장 전에 알아챈다', () => {
        expect(isReversed('6', '3')).toBe(true)
        expect(isReversed('3', '6')).toBe(false)
        expect(isReversed('4', '4')).toBe(false)
        expect(isReversed('조회', '종례')).toBe(false)
    })

    it('열린 구간은 역순일 수 없다', () => {
        expect(isReversed(null, '3')).toBe(false)
        expect(isReversed('6', null)).toBe(false)
    })
})
