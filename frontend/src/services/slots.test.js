/**
 * 교시 규칙 테스트.
 *
 * 이 규칙이 Rust의 slots.rs와 갈라지면 조용히 엉뚱한 기간이 저장된다.
 * 순서 · 표기 · 묶기는 두 구현이 같은 규칙을 따로 만든 자리라, 기대값을 양쪽에
 * 베껴 두는 대신 **고정 벡터 파일 하나**(`src-tauri/src/tests/slot_vectors.json`)를
 * 양쪽 테스트가 읽는다. 베껴 두면 구현이 갈라질 때 테스트도 함께 갈라진다.
 *
 * 나머지(고를 수 있는 기간 · 되돌리기)는 프런트에만 있는 규칙이라 여기서만 확인한다.
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
/** Rust 테스트가 읽는 것과 같은 파일이다. 복사본을 두면 복사본부터 갈라진다. */
import vectors from '../../../src-tauri/src/tests/slot_vectors.json'

/** 벡터의 null은 순서값이 없다는 뜻이다. Rust는 None, 이쪽은 -1로 돌려준다. */
const noOrder = (order) => order ?? -1

/** 실패했을 때 그 줄이 왜 있는지 함께 보여준다. */
const why = (note) => (note ? ` — ${note}` : '')

describe('고정 벡터 — slots.rs와 같은 파일을 읽는다', () => {
    it('묶음이 하나도 비어 있지 않다', () => {
        // 항목 이름이 바뀌면 아래 반복문이 조용히 한 번도 돌지 않고 통과한다.
        const groups = ['slotList', 'ordinal', 'display', 'groupRuns', 'jsOnlyGroupRuns', 'divergences']
        for (const key of groups) {
            expect(vectors[key]?.length, `${key} 묶음이 비어 있다`).toBeGreaterThan(0)
        }
    })

    it('하루의 차례가 벡터와 같다', () => {
        for (const c of vectors.slotList) {
            expect(slotList(c.maxSlot), `maxSlot=${c.maxSlot}${why(c.note)}`).toEqual(c.slots)
        }
    })

    it('슬롯의 순서값이 벡터와 같다', () => {
        for (const c of vectors.ordinal) {
            expect(
                slotOrder(c.slot, c.maxSlot),
                `maxSlot=${c.maxSlot}에서 ${JSON.stringify(c.slot)}${why(c.note)}`,
            ).toBe(noOrder(c.order))
        }
    })

    it('표기가 벡터와 같다', () => {
        for (const c of vectors.display) {
            expect(slotLabel(c.slot), `${JSON.stringify(c.slot)}${why(c.note)}`).toBe(c.label)
        }
    })

    it('이어진 교시를 묶은 결과가 벡터와 같다', () => {
        // Rust의 group_runs는 이미 숫자인 순서값을 받는다. 이쪽은 화면 버튼이 넘기는
        // 문자열 토큰을 받으므로 같은 벡터를 문자열로 바꾸어 넣는다.
        for (const c of vectors.groupRuns) {
            const slots = c.periods.map(String)
            expect(groupRuns(slots), `${JSON.stringify(slots)}${why(c.note)}`).toEqual(c.runs)
        }
    })

    it('교시가 아닌 토큰은 세지 않는다 — 이쪽에만 있는 규칙이다', () => {
        // Rust group_runs에는 이미 걸러진 순서값만 도달하므로 같은 값을 요구하면
        // 거짓 대응이 된다. 그래서 벡터 파일에도 이 묶음만 따로 두었다.
        for (const c of vectors.jsOnlyGroupRuns) {
            expect(groupRuns(c.slots), `${JSON.stringify(c.slots)}${why(c.note)}`).toEqual(c.runs)
        }
    })

    it('갈라진 자리는 기록된 값 그대로다', () => {
        // 두 구현이 다르게 답하는 자리를 벡터 파일에 적어 두었다. 이 테스트는 그 차이를
        // 옳다고 인정하는 것이 아니라, 어느 한쪽이 말없이 또 움직이는 것을 막는다.
        for (const c of vectors.divergences) {
            expect(c.fn).toBe('ordinal')
            expect(
                slotOrder(c.slot, c.maxSlot),
                `${JSON.stringify(c.slot)}${why(c.note)}`,
            ).toBe(noOrder(c.js))
        }
    })
})

describe('allowedSlots', () => {
    it('결석은 고를 것이 없다 — 하루 종일이기 때문이다', () => {
        expect(allowedSlots('none', 7)).toEqual([])
    })

    it('지각은 조회도 고른다 — 조회만 놓친 날이 있다', () => {
        // 지각은 `조회부터 N교시까지`이고, 조회만 놓쳤으면 그 N이 조회다.
        // 나이스 실파일에도 `질병지각 · 결시교시 조회,`가 있었다.
        const allowed = allowedSlots('end', 7)
        expect(allowed).toContain(HOMEROOM)
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

describe('isMulti', () => {
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
            span({slotPrompt: 'end', startSlot: HOMEROOM, endSlot: HOMEROOM}),
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

    it('조회까지인 지각도 되돌려 다시 저장할 수 있다', () => {
        // 나이스 실파일의 `질병지각 · 결시교시 조회,`. 종류를 바꿨다 되돌려도
        // 살아남아야 한다 — 살아남지 못하면 가져온 기록이 수정 모달에서 지워진다.
        const late = span({slotPrompt: 'end', startSlot: HOMEROOM, endSlot: HOMEROOM})
        expect(picksOf(late, 7)).toEqual([HOMEROOM])
        expect(keepUsable(picksOf(late, 7), 'end', 7)).toEqual([HOMEROOM])
    })
})
