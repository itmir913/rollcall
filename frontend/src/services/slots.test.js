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
    picksOf,
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

    it('종류가 미정이면 교시는 전부 열리되 `?`는 열지 않는다', () => {
        // `?`는 **묻고 있는 쪽이 열려 있다**는 뜻이라, 종류가 없으면 어느 쪽을
        // 묻는지 정해지지 않아 저장할 수 없다. Rust `ranges_for`도 이때는 거절한다.
        const allowed = allowedSlots(null, 7)
        expect(allowed).toEqual(['조회', '1', '2', '3', '4', '5', '6', '7', '종례'])
        expect(allowed).not.toContain(UNKNOWN)
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
    it('종류를 바꿔 못 쓰는 기간이 되면 남기지 않는다', () => {
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
        expect(spanPhrase({slotPrompt: 'multi', slots: ['1', '2', '3']}))
            .toBe('1교시부터 3교시까지')
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

describe('picksOf — 저장된 구간을 버튼 선택으로', () => {
    const span = (over) => ({slotPrompt: null, startSlot: null, endSlot: null, ...over})

    it('결석은 고를 것이 없다', () => {
        expect(picksOf(span({slotPrompt: 'none', startSlot: HOMEROOM, endSlot: CLOSING}), 7))
            .toEqual([])
    })

    it('지각은 끝만, 조퇴는 시작만 되돌린다', () => {
        expect(picksOf(span({slotPrompt: 'end', startSlot: HOMEROOM, endSlot: '2'}), 7))
            .toEqual(['2'])
        expect(picksOf(span({slotPrompt: 'start', startSlot: '5', endSlot: CLOSING}), 7))
            .toEqual(['5'])
    })

    it('열린 쪽은 `?`로 되돌린다 — 기간 미정과 구별되어야 한다', () => {
        expect(picksOf(span({slotPrompt: 'end', startSlot: HOMEROOM, endSlot: null}), 7))
            .toEqual([UNKNOWN])
        expect(picksOf(span({slotPrompt: 'start', startSlot: null, endSlot: CLOSING}), 7))
            .toEqual([UNKNOWN])
    })

    it('양쪽이 비면 기간 미정이다', () => {
        expect(picksOf(span({slotPrompt: 'end'}), 7)).toEqual([])
    })

    it('결과의 끝이 조회·종례여도 기간을 지우지 않는다', () => {
        // 지우면 수정 모달을 열었다 저장만 해도 기간이 통째로 날아간다.
        expect(picksOf(span({slotPrompt: 'multi', startSlot: '6', endSlot: CLOSING}), 7))
            .toEqual(['6', '7'])
        expect(picksOf(span({slotPrompt: 'multi', startSlot: HOMEROOM, endSlot: '2'}), 7))
            .toEqual(['1', '2'])
    })

    it('아무것도 건드리지 않고 저장해도 기간이 그대로다', () => {
        // Rust `ranges_for`가 이 목록으로 같은 구간을 다시 만들어야 한다.
        for (const s of [
            span({slotPrompt: 'end', startSlot: HOMEROOM, endSlot: '3'}),
            span({slotPrompt: 'end', startSlot: HOMEROOM, endSlot: null}),
            span({slotPrompt: 'start', startSlot: '4', endSlot: CLOSING}),
            span({slotPrompt: 'start', startSlot: null, endSlot: CLOSING}),
            span({slotPrompt: 'multi', startSlot: '2', endSlot: '4'}),
        ]) {
            const picks = picksOf(s, 7)
            expect(keepUsable(picks, s.slotPrompt, 7)).toEqual(picks)
        }
    })

    it('조회까지인 지각은 되돌려도 고르개가 받아 주지 않는다 — 미해결', () => {
        // 나이스 실파일에 `질병지각 · 결시교시 조회,`가 있고, 가져오기는 그대로 저장한다.
        // 그런데 고르개는 지각에서 조회를 열지 않기로 되어 있어(교사가 정한 규칙),
        // 수정 모달에서 종류를 바꿨다 되돌리면 이 값이 미정으로 떨어진다.
        //
        // 열어야 하는지는 실무 판단이라 앱이 임의로 바꾸지 않는다. 그 사실을 여기
        // 적어 둔다 — 규칙이 바뀌면 이 테스트가 먼저 깨져 다시 보게 된다.
        const late = span({slotPrompt: 'end', startSlot: HOMEROOM, endSlot: HOMEROOM})
        expect(picksOf(late, 7)).toEqual([HOMEROOM])
        expect(allowedSlots('end', 7)).not.toContain(HOMEROOM)
        expect(keepUsable(picksOf(late, 7), 'end', 7)).toEqual([])
    })
})
