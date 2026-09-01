import {describe, expect, it} from 'vitest'
import {formatSpan, normalizeSlot, parseCommand, slotCount} from './commandParser.js'

const CODES = [
    {id: 1, label: '질병결석', shortcut: 'Q', slotPrompt: 'none', defaultStart: null, defaultEnd: null},
    {id: 2, label: '질병조퇴', shortcut: 'W', slotPrompt: 'start', defaultStart: null, defaultEnd: null},
    {id: 3, label: '질병지각', shortcut: 'A', slotPrompt: 'end', defaultStart: null, defaultEnd: null},
    {id: 4, label: '질병결과', shortcut: 'S', slotPrompt: 'both', defaultStart: null, defaultEnd: null},
]

describe('parseCommand', () => {
    it('결석은 번호 + 단축키 + 증상이면 끝난다', () => {
        const r = parseCommand('2 q 몸살', CODES)
        expect(r.ok).toBe(true)
        expect(r.number).toBe(2)
        expect(r.code.label).toBe('질병결석')
        expect(r.startSlot).toBe(null)
        expect(r.endSlot).toBe(null)
        expect(r.symptom).toBe('몸살')
    })

    it('조퇴는 다음 토큰을 시작 교시로 먹는다', () => {
        const r = parseCommand('5 w 6 복통', CODES)
        expect(r.ok).toBe(true)
        expect(r.number).toBe(5)
        expect(r.startSlot).toBe('6')
        expect(r.endSlot).toBe(null)
        expect(r.symptom).toBe('복통')
    })

    it('지각은 끝 교시를 먹는다', () => {
        const r = parseCommand('3 a 4 감기', CODES)
        expect(r.startSlot).toBe(null)
        expect(r.endSlot).toBe('4')
    })

    it('결과는 양쪽 교시를 먹는다', () => {
        const r = parseCommand('7 s 3 3', CODES)
        expect(r.ok).toBe(true)
        expect(r.startSlot).toBe('3')
        expect(r.endSlot).toBe('3')
        expect(r.symptom).toBe(null)
    })

    it('번호와 교시가 둘 다 숫자여도 자리로 구분된다', () => {
        const r = parseCommand('6 w 6', CODES)
        expect(r.number).toBe(6)
        expect(r.startSlot).toBe('6')
    })

    it('단축키는 대소문자를 가리지 않는다', () => {
        expect(parseCommand('2 Q 몸살', CODES).ok).toBe(true)
        expect(parseCommand('2 q 몸살', CODES).ok).toBe(true)
    })

    it('증상은 공백을 포함할 수 있다', () => {
        expect(parseCommand('2 q 심한 몸살', CODES).symptom).toBe('심한 몸살')
    })

    it('증상은 생략할 수 있다', () => {
        const r = parseCommand('2 q', CODES)
        expect(r.ok).toBe(true)
        expect(r.symptom).toBe(null)
    })

    it('조회·종례도 교시로 받는다', () => {
        expect(parseCommand('2 w 조회', CODES).startSlot).toBe('조회')
        expect(parseCommand('2 w 종', CODES).startSlot).toBe('종례')
    })

    it('입력 도중에는 무엇이 더 필요한지 알려준다', () => {
        expect(parseCommand('', CODES).needs).toBe('number')
        expect(parseCommand('2', CODES).needs).toBe('code')
        expect(parseCommand('2 w', CODES).needs).toBe('slot')
        expect(parseCommand('2 s 3', CODES).needs).toBe('slot')
    })

    it('없는 단축키는 오류다', () => {
        expect(parseCommand('2 z 몸살', CODES).error).toContain('단축키')
    })

    it('번호가 아닌 첫 토큰은 오류다', () => {
        expect(parseCommand('이영희 q', CODES).error).toContain('번호')
    })

    it('교시가 아닌 토큰은 오류다', () => {
        expect(parseCommand('2 w 열두', CODES).error).toContain('교시')
        expect(parseCommand('2 w 11', CODES).error).toContain('교시')
    })

    it('역순 구간은 저장 전에 걸린다', () => {
        expect(parseCommand('2 s 6 3', CODES).error).toContain('시작 교시')
    })

    it('코드의 기본 구간을 출발점으로 쓴다', () => {
        const codes = [{id: 9, label: '조퇴', shortcut: 'D', slotPrompt: 'start', defaultStart: null, defaultEnd: '종례'}]
        const r = parseCommand('2 d 5', codes)
        expect(r.startSlot).toBe('5')
        expect(r.endSlot).toBe('종례')
    })
})

describe('보조 함수', () => {
    it('slotCount는 유형이 요구하는 교시 수다', () => {
        expect(slotCount('none')).toBe(0)
        expect(slotCount('start')).toBe(1)
        expect(slotCount('end')).toBe(1)
        expect(slotCount('both')).toBe(2)
    })

    it('normalizeSlot은 약어를 편다', () => {
        expect(normalizeSlot('조')).toBe('조회')
        expect(normalizeSlot('5')).toBe('5')
        expect(normalizeSlot('점심')).toBe(null)
    })

    it('formatSpan은 열린 쪽을 *로 쓴다', () => {
        expect(formatSpan(null, null)).toBe('* ~ *')
        expect(formatSpan('5', null)).toBe('5 ~ *')
    })
})
