/**
 * 출결 기록 스토어.
 *
 * 여기서 지키는 것은 하나다 — **NEIS 검증은 파일의 기간으로 대조한다.**
 * 화면이 마지막에 보던 달로 맞추면 6월 파일을 9월 기록과 견주고는
 * "전부 앱에 없음"이라고 말한다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useLogStore} from './log'
import {useAppStore} from './app'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))

function ready() {
    const app = useAppStore()
    app.schoolId = 1
    app.yearId = 2
    app.grade = 3
    app.classNo = 6
    app.today = '2026-09-11'
}

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
    invoke.mockResolvedValue([])
})

describe('fetchBetween', () => {
    it('달이 아니라 기간으로 묻는다', async () => {
        ready()
        await useLogStore().fetchBetween('2026-06-01', '2026-06-30')

        expect(invoke).toHaveBeenCalledWith('get_spans_between', {
            schoolId: 1, yearId: 2, grade: 3, classNo: 6,
            from: '2026-06-01', to: '2026-06-30', today: '2026-09-11',
        })
    })

    it('달을 고르지 않았어도 부를 수 있다 — 검증은 출결 기록을 거치지 않는다', async () => {
        ready()
        const log = useLogStore()
        expect(log.month).toBeNull()
        await expect(log.fetchBetween('2026-06-01', '2026-06-30')).resolves.toEqual([])
    })

    it('실패를 삼키지 않는다', async () => {
        ready()
        invoke.mockRejectedValue('끝 날짜가 시작 날짜보다 앞입니다.')
        const log = useLogStore()
        await expect(log.fetchBetween('2026-06-30', '2026-06-01')).rejects.toBeTruthy()
        expect(log.error).toContain('끝 날짜')
    })
})
