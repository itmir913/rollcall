/**
 * 문구.
 *
 * `spanPhrase`(고른 교시 → 문장)와 `spanTextOf`(저장된 두 끝 → 문장)는 입력이 달라
 * 함수가 둘이지만 **같은 말을 써야 한다.** 그리고 그 말은 Rust `attendance.rs`의
 * `span_text`와도 같아야 한다 — 같은 기간이 화면마다 다르게 적히면 교사는 그것을
 * 다름으로 읽는다. 아래 경계값은 `attendance_tests.rs`의 것과 짝이다.
 */
import {describe, expect, it} from 'vitest'
import {axisPhrase, spanPhrase, spanTextOf, stampPhrase} from './phrase'

describe('spanTextOf — 저장된 구간', () => {
    it('결석은 기간을 묻지 않는다', () => {
        expect(spanTextOf('none', '조회', '종례')).toBe('하루 종일')
    })

    it('양쪽이 비면 기간 미정이다', () => {
        expect(spanTextOf('start', null, null)).toBe('기간 미정')
    })

    it('지각은 조회부터, 조퇴는 종례까지다', () => {
        expect(spanTextOf('end', '조회', '2')).toBe('조회부터 2교시까지')
        expect(spanTextOf('start', '5', '종례')).toBe('5교시부터 종례까지')
    })

    it('두 끝이 같으면 슬롯 하나로 적는다', () => {
        expect(spanTextOf('multi', '3', '3')).toBe('3교시')
        // 나이스 실파일에 결시교시가 `조회,` 하나뿐인 지각이 있었다.
        expect(spanTextOf('end', '조회', '조회')).toBe('조회')
        expect(spanTextOf('start', '종례', '종례')).toBe('종례')
    })

    it('열린 쪽은 물음표로 적는다', () => {
        expect(spanTextOf('multi', '3', null)).toBe('3교시부터 ?까지')
        expect(spanTextOf('multi', null, '3')).toBe('?부터 3교시까지')
    })
})

describe('spanPhrase — 아직 저장하지 않은 조합', () => {
    it('같은 기간을 spanTextOf와 같은 말로 적는다', () => {
        expect(spanPhrase({slotPrompt: 'none', slots: []})).toBe(spanTextOf('none', '조회', '종례'))
        expect(spanPhrase({slotPrompt: 'end', slots: ['2']})).toBe(spanTextOf('end', '조회', '2'))
        expect(spanPhrase({slotPrompt: 'start', slots: ['5']})).toBe(spanTextOf('start', '5', '종례'))
        expect(spanPhrase({slotPrompt: null, slots: []})).toBe(spanTextOf(null, null, null))
    })

    it('결과는 이어진 것끼리 묶어 적는다', () => {
        expect(spanPhrase({slotPrompt: 'multi', slots: ['1', '2', '3']})).toBe('1~3교시')
        expect(spanPhrase({slotPrompt: 'multi', slots: ['1', '3', '5']})).toBe('1교시 · 3교시 · 5교시')
    })
})

describe('축 문구', () => {
    it('비어 있으면 미정이다 — 미완성 기록은 정상 상태다', () => {
        expect(axisPhrase(null, null)).toBe('미정 미정')
        expect(axisPhrase('질병', null)).toBe('질병 미정')
    })

    it('축 카드 아래 한 줄은 두 축과 기간을 잇는다', () => {
        expect(stampPhrase({reasonLabel: '질병', typeLabel: '조퇴', slotPrompt: 'start', slots: ['5']}))
            .toBe('질병 조퇴 · 5교시부터 종례까지')
    })
})
