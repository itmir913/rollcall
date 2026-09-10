/**
 * 나이스 파일 대조.
 *
 * 파일 서식은 아직 분석하지 않았지만, **대조 규칙은 서식과 무관하다.**
 * 그래서 여기부터 확정하고 테스트를 붙인다.
 *
 * 대조의 핵심은 하나다 — 하루에 두 구간인 경우가 있으므로 **여럿을 여럿과** 비교한다.
 */
import {describe, expect, it} from 'vitest'
import {compareRecords, looksLikeXlsx, mapColumns, sameRecord, sortPairs, toIso} from './neisFile'

const mine = (over = {}) => ({
    number: 5, name: '김하늘', date: '2026-09-10',
    reasonLabel: '질병', typeLabel: '결석', spanText: '하루 종일', ...over,
})
const theirs = (over = {}) => ({
    number: 5, name: '김하늘', date: '2026-09-10',
    reasonLabel: '질병', typeLabel: '결석', detail: null, ...over,
})

describe('compareRecords', () => {
    it('같은 것은 일치로 센다', () => {
        const out = compareRecords([mine()], [theirs()])
        expect(out.same).toHaveLength(1)
        expect(out.diff).toHaveLength(0)
    })

    it('같은 날 같은 학생인데 내용이 다르면 다름이다', () => {
        const out = compareRecords([mine()], [theirs({reasonLabel: '미인정'})])
        expect(out.diff).toHaveLength(1)
        expect(out.diff[0].mine.reasonLabel).toBe('질병')
        expect(out.diff[0].theirs.reasonLabel).toBe('미인정')
    })

    it('앱에만 있으면 나이스 쪽이 비어 있다 — 칸을 없애지 않는다', () => {
        const out = compareRecords([mine()], [])
        expect(out.onlyApp).toHaveLength(1)
        expect(out.onlyApp[0].theirs).toBeNull()
    })

    it('나이스에만 있으면 앱 쪽이 비어 있다', () => {
        const out = compareRecords([], [theirs()])
        expect(out.onlyNeis).toHaveLength(1)
        expect(out.onlyNeis[0].mine).toBeNull()
    })

    it('하루 2구간에서 하나만 어긋나면 나머지는 일치로 남는다', () => {
        const out = compareRecords(
            [mine({typeLabel: '지각'}), mine({typeLabel: '조퇴'})],
            [theirs({typeLabel: '지각'}), theirs({typeLabel: '결과'})],
        )
        expect(out.same).toHaveLength(1)
        expect(out.diff).toHaveLength(1)
        expect(out.onlyApp).toHaveLength(0)
    })

    it('다른 날짜는 서로 짝이 되지 않는다', () => {
        const out = compareRecords([mine()], [theirs({date: '2026-09-11'})])
        expect(out.diff).toHaveLength(0)
        expect(out.onlyApp).toHaveLength(1)
        expect(out.onlyNeis).toHaveLength(1)
    })

    it('표기의 띄어쓰기 차이는 흡수한다 — 학교마다 다르게 적는다', () => {
        expect(sameRecord({reasonLabel: '질병', typeLabel: '조퇴'},
            {reasonLabel: '질병', typeLabel: '조 퇴'})).toBe(true)
    })
})

describe('sortPairs', () => {
    const pairs = [
        {mine: mine({number: 12, date: '2026-09-01'}), theirs: null},
        {mine: null, theirs: theirs({number: 3, date: '2026-09-20'})},
    ]

    it('번호순은 번호가 먼저다', () => {
        expect(sortPairs(pairs, 'number').map((p) => (p.mine ?? p.theirs).number)).toEqual([3, 12])
    })

    it('날짜순은 날짜가 먼저다', () => {
        expect(sortPairs(pairs, 'date').map((p) => (p.mine ?? p.theirs).date))
            .toEqual(['2026-09-01', '2026-09-20'])
    })
})

describe('파일 읽기 도구', () => {
    it('이름만 xlsx인 파일을 zip 서명으로 거른다', () => {
        expect(looksLikeXlsx(new Uint8Array([0x50, 0x4b, 0x03, 0x04, 0x00]))).toBe(true)
        expect(looksLikeXlsx(new Uint8Array([0x3c, 0x3f, 0x78, 0x6d]))).toBe(false)
    })

    it('열은 이름으로 찾는다. 위치로 찾지 않는다', () => {
        const columns = mapColumns(['학년', '반', '번호', '성명', '일자', '출결상황', '사유'])
        expect(columns.number).toBe(2)
        expect(columns.name).toBe(3)
        expect(columns.date).toBe(4)
    })

    it('학교마다 다른 표기를 별칭으로 흡수한다', () => {
        expect(mapColumns(['출석번호', '학생명', '결석일']).number).toBe(0)
        expect(mapColumns(['출석번호', '학생명', '결석일']).date).toBe(2)
    })

    it('날짜 표기가 제각각이어도 ISO로 모은다', () => {
        expect(toIso('2026.09.10.')).toBe('2026-09-10')
        expect(toIso('2026-9-3')).toBe('2026-09-03')
        expect(toIso('20260910')).toBe('2026-09-10')
        expect(toIso(new Date(2026, 8, 10))).toBe('2026-09-10')
    })

    it('읽을 수 없는 날짜는 조용히 넘기지 않고 null로 알린다', () => {
        expect(toIso('지난주')).toBeNull()
        expect(toIso('')).toBeNull()
    })
})
