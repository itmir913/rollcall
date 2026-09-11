/**
 * 나이스 서식 정의를 데이터로 두는 이유가 이 파일에 적혀 있다.
 *
 * 여기서 고정하는 것은 둘이다.
 *   1. **교체하면 바로 적용된다.** 나이스가 열 이름을 바꿔도 JSON 한 장이면 된다.
 *   2. **어긋난 서식은 교체하지 않는다.** 반쯤 맞는 서식으로 파일을 읽으면 조용히 엉뚱한
 *      값이 들어가는데, 그것이 이 앱에서 가장 비싼 실패다. 통째로 버리고 번들로 돌아간다.
 */
import {afterEach, describe, expect, it} from 'vitest'
import bundled from '../data/neisFormats.json'
import {
    compileSpec,
    dateInText,
    matchNeisColumn,
    resetSpec,
    specSource,
    useSpec,
} from './neisFormat'
import {slotTokenOf} from './neisFile'

/** 서식을 교체하는 시험이 있으므로 매번 되돌린다. */
afterEach(resetSpec)

/** 번들 서식을 복사해 한 군데만 망가뜨린다. */
function broken(mutate) {
    const copy = JSON.parse(JSON.stringify(bundled))
    mutate(copy)
    return copy
}

describe('번들에 든 서식', () => {
    it('쓸 수 있는 모양이고 어디서 왔는지 말한다', () => {
        const {source, version} = specSource()
        expect(source).toBe('bundled')
        expect(version).toBe(bundled.version)
        // 실파일에서 확인한 열들이 그대로 잡힌다.
        expect(matchNeisColumn('출결구분')).toBe('code')
        expect(matchNeisColumn('결시교시')).toBe('slots')
        // `마감`이 코드 열인 것은 오타가 아니다 — 일일출석부가 그렇게 내보낸다.
        expect(matchNeisColumn('마감')).toBe('code')
    })
})

describe('서식을 교체한다', () => {
    it('나이스가 열 이름을 바꿔도 JSON 한 장이면 된다', () => {
        expect(matchNeisColumn('출결사항')).toBeNull()

        const next = JSON.parse(JSON.stringify(bundled))
        next.version = '2027-03-01'
        next.columns.code.push('출결사항')
        expect(useSpec(next)).toBe(true)

        expect(matchNeisColumn('출결사항')).toBe('code')
        expect(specSource()).toEqual({source: 'remote', version: '2027-03-01'})
    })

    it('교시 표기가 바뀌어도 파일 읽는 쪽은 그대로다', () => {
        expect(slotTokenOf('3차시')).toBeNull()

        const next = JSON.parse(JSON.stringify(bundled))
        next.slot.periodPattern = '^(\\d{1,2})차시$'
        next.slot.homeroomLabels = ['아침조회']
        expect(useSpec(next)).toBe(true)

        expect(slotTokenOf('3차시')).toBe('3')
        expect(slotTokenOf('아침조회')).toBe('조회')
        // 옛 표기는 더는 읽지 않는다. 서식이 곧 규칙이다.
        expect(slotTokenOf('3교시')).toBeNull()
    })

    it('날짜 표기도 서식이 정한다', () => {
        expect(dateInText('2026년 9월 10일')).toBeNull()

        const next = JSON.parse(JSON.stringify(bundled))
        next.date.patterns.push('(\\d{4})년\\s*(\\d{1,2})월\\s*(\\d{1,2})일')
        expect(useSpec(next)).toBe(true)

        expect(dateInText('2026년 9월 10일')).toBe('2026-09-10')
    })

    it('되돌리면 번들 것으로 돌아간다', () => {
        const next = JSON.parse(JSON.stringify(bundled))
        next.columns.code.push('출결사항')
        useSpec(next)
        expect(matchNeisColumn('출결사항')).toBe('code')

        resetSpec()
        expect(matchNeisColumn('출결사항')).toBeNull()
        expect(specSource().source).toBe('bundled')
    })
})

describe('어긋난 서식은 교체하지 않는다', () => {
    /**
     * **반쯤 받아들이지 않는다.** 열 하나가 빠진 서식을 그대로 쓰면 그 열이 없는
     * 파일로 읽혀 값이 통째로 비는데, 교사는 파일이 잘못된 줄 안다.
     */
    const 망가진_것 = {
        '필수 열이 빠짐': broken((c) => delete c.columns.slots),
        '열 별칭이 빈 배열': broken((c) => {
            c.columns.code = []
        }),
        '교시 정규식이 깨짐': broken((c) => {
            c.slot.periodPattern = '^(\\d{1,2}교시$'
        }),
        '학급 정규식이 깨짐': broken((c) => {
            c.caption.classPattern = '([0-9]+학년'
        }),
        '캡션 기호가 없음': broken((c) => {
            c.caption.markers = []
        }),
        '교시 목록 규칙이 없음': broken((c) => delete c.slotList),
        '날짜 표기가 하나도 없음': broken((c) => {
            c.date.patterns = []
        }),
        '통째로 빈 것': {},
    }

    for (const [이름, 서식] of Object.entries(망가진_것)) {
        it(`${이름} — 교체하지 않고 번들을 지킨다`, () => {
            expect(compileSpec(서식)).toBeNull()
            expect(useSpec(서식)).toBe(false)
            // 앞서 쓰던 서식이 그대로 살아 있다.
            expect(specSource().source).toBe('bundled')
            expect(matchNeisColumn('출결구분')).toBe('code')
            expect(slotTokenOf('3교시')).toBe('3')
        })
    }

    it('객체가 아닌 것도 조용히 지나가지 않는다', () => {
        for (const 아닌것 of [null, undefined, '서식', 42, []]) {
            expect(useSpec(아닌것)).toBe(false)
        }
        expect(specSource().source).toBe('bundled')
    })
})
