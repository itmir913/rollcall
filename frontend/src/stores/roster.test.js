/**
 * 명렬표 스토어.
 *
 * 여기서 잡는 것은 둘이다.
 *   · **명단은 학급으로 묻는다.** 학년 · 반 · 번호는 학적이지 소속이 아니라,
 *     반으로 걸러내면 교과 강좌(여러 반에서 모인다)와 전학생을 담지 못한다.
 *   · **학적을 건드리는 일은 학교를 함께 넘긴다.** 순회 교사가 학교를 둘 이상
 *     등록하면 같은 학년 · 반이 겹치는데, 학교를 빼고 물으면 다른 학교의 3학년
 *     6반이 섞여 들어온다. 그때는 이미 늦다.
 *
 * 학생 이름은 전부 가짜다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useRosterStore} from './roster'
import {useAppStore} from './app'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
    invoke.mockResolvedValue([])
    const app = useAppStore()
    app.schools = [{id: 1, name: '한빛고등학교', maxSlot: 7}]
    app.classes = [{
        id: 9, schoolId: 1, yearId: 2, role: 'homeroom', name: '3학년 6반',
        grade: 3, classNo: 6, validTo: null,
    }]
    app.classId = 9
})

describe('명렬표', () => {
    it('학생 목록은 학급 하나로 묻는다 — 명단은 class_member가 안다', async () => {
        await useRosterStore().fetchStudents(9)
        expect(invoke).toHaveBeenCalledWith('get_students', {classId: 9})
    })

    it('미리보기와 저장 모두 학급을 넘긴다 — 재가져오기는 차분이다', async () => {
        const roster = useRosterStore()
        await roster.preview(9, [{number: 1, name: '학생1'}])
        expect(invoke).toHaveBeenCalledWith('preview_roster', {
            classId: 9, entries: [{number: 1, name: '학생1'}],
        })

        invoke.mockResolvedValue({added: 1, renamed: 0, withdrawn: 0})
        await roster.apply(9, '2026-03-02', [])
        const applyCall = invoke.mock.calls.find((c) => c[0] === 'apply_roster')
        expect(applyCall[1]).toEqual({classId: 9, effectiveDate: '2026-03-02', rows: []})
    })

    it('저장한 뒤 같은 학급의 명단을 다시 읽는다', async () => {
        const roster = useRosterStore()
        invoke.mockResolvedValue({added: 1, renamed: 0, withdrawn: 0})

        await roster.apply(9, '2026-03-02', [])

        expect(invoke).toHaveBeenLastCalledWith('get_students', {classId: 9})
    })

    it('학생 수정과 전출도 지금 학급 안에서 한다', async () => {
        // 두 커맨드도 범위가 classId 하나다. 전에는 schoolId를 보내고 있었고
        // 그 모양을 이 테스트가 고정하고 있어, 화면을 붙이는 순간 깨질 상태였다.
        const roster = useRosterStore()
        await roster.updateStudent(9, 5, '학생1')
        await roster.withdraw(9, '2026-09-10')

        expect(invoke).toHaveBeenCalledWith('update_student', {
            classId: 9, id: 9, number: 5, name: '학생1',
        })
        expect(invoke).toHaveBeenCalledWith('withdraw_student', {
            classId: 9, id: 9, date: '2026-09-10',
        })
    })

    it('학급 판단은 파일이 말해 준다 — 넣을 명단은 교사가 고른 학급이다', async () => {
        await useRosterStore().detectClass([{number: 1, name: '학생1'}])
        expect(invoke).toHaveBeenCalledWith('detect_roster_class', {
            entries: [{number: 1, name: '학생1'}],
        })
    })

    it('실패는 error에 담고 다시 던진다', async () => {
        invoke.mockRejectedValue('학생을 찾을 수 없습니다: 9')
        const roster = useRosterStore()

        await expect(roster.updateStudent(9, 5, '학생1')).rejects.toBeTruthy()
        expect(roster.error).toContain('학생을 찾을 수 없습니다')
    })
})
