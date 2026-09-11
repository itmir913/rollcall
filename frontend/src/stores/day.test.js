/**
 * 오늘의 출결 스토어.
 *
 * 여기서 지키는 것은 **화면이 부르는 커맨드와 인자**다. 이름이나 인자가 어긋나면
 * 앱은 조용히 빈 화면을 보여준다 — 그래서 호출 자체를 테스트한다.
 *
 * 그리고 스토어의 액션은 에러를 `error`에 담고 **다시 던진다.** 삼키면 읽기 실패가
 * "데이터 없음"과 구분되지 않는다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useDayStore} from './day'
import {useAppStore} from './app'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))

function readyApp() {
    const app = useAppStore()
    app.schoolId = 1
    app.yearId = 2
    app.grade = 3
    app.classNo = 6
    app.today = '2026-09-10'
    return app
}

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
    invoke.mockResolvedValue({date: '2026-09-10', rows: [], spans: []})
})

describe('오늘의 출결', () => {
    it('학급과 날짜를 함께 넘겨 격자를 받는다', async () => {
        readyApp()
        const day = useDayStore()
        day.setDate('2026-09-10')
        await day.fetchGrid()

        expect(invoke).toHaveBeenCalledWith('get_day_grid', {
            schoolId: 1, yearId: 2, grade: 3, classNo: 6, date: '2026-09-10',
        })
    })

    it('학급이 정해지기 전에는 부르지 않는다 — 빈 인자로 물어봐야 답이 없다', async () => {
        const day = useDayStore()
        await day.fetchGrid()
        expect(invoke).not.toHaveBeenCalled()
    })

    it('찍을 때 지금 고른 조합을 그대로 넘긴다', async () => {
        readyApp()
        const day = useDayStore()
        day.setDate('2026-09-10')
        day.draft = {reasonId: 10, typeId: 1, slots: ['2']}
        invoke.mockResolvedValue({action: 'added', spanIds: [1]})

        await day.stamp(77)

        expect(invoke).toHaveBeenCalledWith('stamp_span', {
            input: {studentId: 77, date: '2026-09-10', reasonId: 10, typeId: 1, slots: ['2']},
        })
    })

    it('찍은 뒤 격자를 다시 읽는다 — 취소인지 추가인지는 Rust가 판단한다', async () => {
        readyApp()
        const day = useDayStore()
        day.setDate('2026-09-10')
        await day.stamp(77)
        expect(invoke.mock.calls.map((c) => c[0])).toContain('get_day_grid')
    })

    it('고치기는 구간 하나만 바꾼다', async () => {
        readyApp()
        const day = useDayStore()
        day.setDate('2026-09-10')
        await day.editSpan(5, {reasonId: 11, typeId: 2, slots: ['4']})

        expect(invoke).toHaveBeenCalledWith('edit_span', {
            edit: {spanId: 5, reasonId: 11, typeId: 2, slots: ['4']},
        })
    })

    it('태그를 떼는 것도 값이다 — null을 그대로 넘긴다', async () => {
        readyApp()
        const day = useDayStore()
        await day.setTag(5, null)
        expect(invoke).toHaveBeenCalledWith('set_span_tag', {spanId: 5, tagId: null})
    })

    it('실패는 error에 담고 다시 던진다', async () => {
        readyApp()
        const day = useDayStore()
        day.setDate('2026-09-10')
        invoke.mockRejectedValue('학생을 찾을 수 없습니다: 99')

        await expect(day.stamp(99)).rejects.toBeTruthy()
        expect(day.error).toContain('학생을 찾을 수 없습니다')
    })

    it('하루씩 옮긴다', async () => {
        readyApp()
        const day = useDayStore()
        day.setDate('2026-09-01')
        await day.move(-1)
        expect(day.date).toBe('2026-08-31')
    })

    it('기록된 학생 수는 구간 수가 아니다', async () => {
        readyApp()
        const day = useDayStore()
        day.setDate('2026-09-10')
        invoke.mockResolvedValue({
            date: '2026-09-10',
            rows: [],
            spans: [{studentId: 1}, {studentId: 1}, {studentId: 2}],
        })
        await day.fetchGrid()
        expect(day.recorded).toBe(2)
    })
})
