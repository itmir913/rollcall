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

/** 담임 학급 하나를 고른 상태. **범위는 그 학급 하나다.** */
function readyApp() {
    const app = useAppStore()
    app.schools = [{id: 1, name: '한빛고등학교', maxSlot: 7}]
    app.yearId = 2
    app.classes = [{
        id: 9, schoolId: 1, yearId: 2, role: 'homeroom', name: '3학년 6반',
        grade: 3, classNo: 6, validTo: null,
    }]
    app.classId = 9
    app.today = '2026-09-10'
    return app
}

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
    invoke.mockResolvedValue({date: '2026-09-10', rows: [], spans: []})
})

describe('오늘의 출결', () => {
    it('학급과 날짜를 함께 넘겨 격자를 받는다 — 학급은 번호 하나다', async () => {
        readyApp()
        const day = useDayStore()
        day.setDate('2026-09-10')
        await day.fetchGrid()

        expect(invoke).toHaveBeenCalledWith('get_day_grid', {classId: 9, date: '2026-09-10'})
    })

    /**
     * 같은 학생이 내 담임 반에도 내 교과 강좌에도 있을 수 있다. 학생으로 거르면
     * 교과 화면에 담임 출결이 유출되므로, 범위는 언제나 학급이다.
     */
    it('여러 날 미리보기도 학급으로 묻는다 — 휴업일은 학급이 아는 학교에 있다', async () => {
        readyApp()
        const day = useDayStore()
        invoke.mockResolvedValue({dates: [], skipped: []})

        await day.previewBulk('2026-09-10', '2026-09-12')

        expect(invoke).toHaveBeenCalledWith('preview_bulk', {
            classId: 9, from: '2026-09-10', to: '2026-09-12',
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

        // **classId가 빠지면 Rust가 통째로 거절한다**(`StampInput.class_id`는 필수다).
        // 이 단언이 전에는 classId 없는 모양을 고정하고 있어, 찍기가 죽은 채로
        // 테스트가 초록이었다. 인자를 통째로 비교해 다시 그렇게 되지 않게 한다.
        expect(invoke).toHaveBeenCalledWith('stamp_span', {
            input: {
                classId: 9, studentId: 77, date: '2026-09-10',
                reasonId: 10, typeId: 1, slots: ['2'],
            },
        })
    })

    it('무르기가 거부되면 이유를 담는다 — 화면은 아무 일도 없는 것처럼 보인다', async () => {
        // 태그 · 사유 · 서류 · 나이스 표시가 붙은 구간은 다시 눌러도 지워지지 않는다.
        // 그때 격자는 그대로이므로, 이유를 알리지 않으면 교사는 단추가 고장 난 줄 안다.
        readyApp()
        const day = useDayStore()
        day.setDate('2026-09-10')
        invoke.mockResolvedValue({
            action: 'kept',
            spanIds: [5],
            message: '태그가 남아 있어 무르지 않았습니다. 지우려면 그 줄의 휴지통 버튼을 쓰세요.',
        })

        await day.stamp(77)

        expect(day.notice).toContain('무르지 않았습니다')
        // 오류가 아니다. 교사가 잘못한 것이 없다.
        expect(day.error).toBe('')
    })

    it('다음에 제대로 찍히면 그 이유를 지운다', async () => {
        readyApp()
        const day = useDayStore()
        day.setDate('2026-09-10')
        invoke.mockResolvedValue({action: 'kept', spanIds: [5], message: '남아 있습니다.'})
        await day.stamp(77)
        expect(day.notice).not.toBe('')

        invoke.mockResolvedValue({action: 'added', spanIds: [6]})
        await day.stamp(78)
        expect(day.notice).toBe('')
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
