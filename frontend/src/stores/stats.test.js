/**
 * 통계 스토어 — **세는 창 밖을 알리는 부분**만 본다.
 *
 * 학년도는 기준 연도일 뿐이라 2026학년도 학급에도 2027년 3월 기록을 남길 수 있다.
 * 그런데 한도는 학년도 창으로 센다(연 20일의 '연'이 학년도다). 그 차이로 빠진 건을
 * 조용히 넘기면 통계가 **"덜 썼다"고 거짓으로 알린다.**
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {useStatsStore} from './stats'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))

beforeEach(() => setActivePinia(createPinia()))

/** 규정 하나의 결과. 창과 창 밖 건수만 채운다. */
function report(outside, from = '2026-03-01', to = '2027-02-28') {
    return {ruleId: 1, rows: [], untagged: [], outside, windowFrom: from, windowTo: to}
}

describe('세는 창 밖에서 빠진 기록', () => {
    it('규정이 여럿이면 합쳐서 센다', () => {
        const stats = useStatsStore()
        stats.reports = [report(2), report(1)]
        expect(stats.outside).toBe(3)
    })

    it('빠진 것이 없으면 0이다 — 화면이 그때는 아무 말도 하지 않는다', () => {
        const stats = useStatsStore()
        stats.reports = [report(0), report(0)]
        expect(stats.outside).toBe(0)
    })

    it('어느 범위였는지 함께 말한다', () => {
        const stats = useStatsStore()
        stats.reports = [report(2)]
        expect(stats.window).toBe('2026-03-01 ~ 2027-02-28')
    })

    it('옛 결과에 창이 없어도 무너지지 않는다', () => {
        // Rust가 창을 실어 보내기 전의 모양이 남아 있어도 화면이 깨지면 안 된다.
        const stats = useStatsStore()
        stats.reports = [{ruleId: 1, rows: [], untagged: []}]
        expect(stats.outside).toBe(0)
        expect(stats.window).toBe('')
    })
})
