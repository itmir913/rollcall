/**
 * 교시 규칙 테스트.
 *
 * 이 규칙이 Rust의 slots.rs와 갈라지면 조용히 엉뚱한 기간이 저장된다.
 * 그래서 양쪽에 같은 경계를 둔다 — 여기를 고칠 때는 slots.rs도 함께 본다.
 */
import {describe, expect, it} from 'vitest'
import {
    CLOSING,
    HOMEROOM,
    UNKNOWN,
    allowedSlots,
    groupRuns,
    isMulti,
    keepUsable,
    slotLabel,
    slotList,
    slotOrder,
} from './slots'
import {axisPhrase, spanPhrase, stampPhrase} from './phrase'

describe('slotList', () => {
    it('조회와 종례가 하루의 양 끝이다', () => {
        expect(slotList(7)).toEqual([HOMEROOM, '1', '2', '3', '4', '5', '6', '7', CLOSING])
    })

    it('최대 교시는 학교 설정이라 값에 따라 늘고 준다', () => {
        expect(slotList(3)).toEqual([HOMEROOM, '1', '2', '3', CLOSING])
        expect(slotList(9)).toHaveLength(11)
    })
})

describe('allowedSlots', () => {
    it('결석은 고를 것이 없다 — 하루 종일이기 때문이다', () => {
        expect(allowedSlots('none', 7)).toEqual([])
    })

    it('지각에는 조회가 없다. 조회에 이미 왔으면 지각이 아니다', () => {
        const allowed = allowedSlots('end', 7)
        expect(allowed).not.toContain(HOMEROOM)
        expect(allowed).toContain(CLOSING)
        expect(allowed).toContain(UNKNOWN)
    })

    it('조퇴에는 종례가 없다. 종례까지 있었으면 조퇴가 아니다', () => {
        const allowed = allowedSlots('start', 7)
        expect(allowed).toContain(HOMEROOM)
        expect(allowed).not.toContain(CLOSING)
    })

    it('결과는 교시만 고른다 — 조회·종례는 교과 시간이 아니고 모를 수도 없다', () => {
        expect(allowedSlots('multi', 7)).toEqual(['1', '2', '3', '4', '5', '6', '7'])
    })

    it('구분이 미정이면 전부 열려 있다', () => {
        expect(allowedSlots(null, 7)).toHaveLength(10)
    })
})

describe('groupRuns', () => {
    it('이어진 교시는 한 구간이다', () => {
        expect(groupRuns(['1', '2', '3'])).toEqual([[1, 3]])
    })

    it('떨어진 교시는 각각 구간이다 — 이어지지 않은 것을 묶으면 사이가 조용히 포함된다', () => {
        expect(groupRuns(['1', '3', '5'])).toEqual([[1, 1], [3, 3], [5, 5]])
    })

    it('순서가 뒤섞이거나 중복돼도 정리한다', () => {
        expect(groupRuns(['3', '1', '2', '2'])).toEqual([[1, 3]])
    })

    it('교시가 아닌 토큰은 세지 않는다', () => {
        expect(groupRuns([HOMEROOM, UNKNOWN, CLOSING])).toEqual([])
    })
})

describe('keepUsable', () => {
    it('구분을 바꿔 못 쓰는 기간이 되면 남기지 않는다', () => {
        expect(keepUsable([CLOSING], 'start', 7)).toEqual([])
    })

    it('하나만 고르는 구분에서는 첫 값만 남는다', () => {
        expect(keepUsable(['1', '2'], 'end', 7)).toEqual(['1'])
    })

    it('결과는 여러 개를 그대로 남긴다', () => {
        expect(keepUsable(['1', '2'], 'multi', 7)).toEqual(['1', '2'])
    })
})

describe('slotOrder · slotLabel', () => {
    it('최대 교시 밖은 순서가 없다', () => {
        expect(slotOrder('8', 7)).toBe(-1)
        expect(slotOrder('7', 7)).toBe(7)
    })

    it('조회는 처음, 종례는 끝이다', () => {
        expect(slotOrder(HOMEROOM, 7)).toBe(0)
        expect(slotOrder(CLOSING, 7)).toBe(8)
    })

    it('교시에만 "교시"를 붙인다', () => {
        expect(slotLabel('5')).toBe('5교시')
        expect(slotLabel(HOMEROOM)).toBe(HOMEROOM)
        expect(slotLabel(UNKNOWN)).toBe(UNKNOWN)
    })

    it('여러 교시를 고르는 것은 결과뿐이다', () => {
        expect(isMulti('multi')).toBe(true)
        expect(isMulti('end')).toBe(false)
    })
})

describe('spanPhrase', () => {
    it('결석은 하루 종일이다', () => {
        expect(spanPhrase({slotPrompt: 'none', slots: []})).toBe('하루 종일')
    })

    it('고른 기간이 없으면 기간 미정이다 — 정상 상태이지 오류가 아니다', () => {
        expect(spanPhrase({slotPrompt: 'end', slots: []})).toBe('기간 미정')
    })

    it('지각은 조회부터, 조퇴는 종례까지다', () => {
        expect(spanPhrase({slotPrompt: 'end', slots: ['2']})).toBe('조회부터 2교시까지')
        expect(spanPhrase({slotPrompt: 'start', slots: ['5']})).toBe('5교시부터 종례까지')
    })

    it('언제 왔는지 모르면 열린 채로 말한다', () => {
        expect(spanPhrase({slotPrompt: 'end', slots: [UNKNOWN]})).toBe('조회부터 ?까지')
    })

    it('결과는 이어진 것끼리 묶어 말한다', () => {
        expect(spanPhrase({slotPrompt: 'multi', slots: ['1', '2', '3']})).toBe('1~3교시')
        expect(spanPhrase({slotPrompt: 'multi', slots: ['1', '3', '5']})).toBe('1교시 · 3교시 · 5교시')
    })
})

describe('axisPhrase · stampPhrase', () => {
    it('비어 있는 축은 미정으로 읽는다', () => {
        expect(axisPhrase(null, null)).toBe('미정 미정')
        expect(axisPhrase('질병', null)).toBe('질병 미정')
    })

    it('축 카드 아래 한 줄을 만든다', () => {
        expect(
            stampPhrase({reasonLabel: '질병', typeLabel: '지각', slotPrompt: 'end', slots: ['2']}),
        ).toBe('질병 지각 · 조회부터 2교시까지')
    })
})
