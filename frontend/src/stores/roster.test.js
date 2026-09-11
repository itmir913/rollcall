/**
 * 명렬표 스토어.
 *
 * 여기서 잡는 것은 하나다 — **모든 질의가 school_id를 함께 넘긴다.**
 * 순회 교사가 학교를 둘 이상 등록하면 같은 학년·반이 겹치는데, 학교를 빼고 물으면
 * 다른 학교의 3학년 6반이 섞여 들어온다. 그때는 이미 늦다.
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
    useAppStore().schoolId = 1
})

describe('명렬표', () => {
    it('학생 목록은 학교와 학급을 함께 묻는다', async () => {
        await useRosterStore().fetchStudents(2, 3, 6)
        expect(invoke).toHaveBeenCalledWith('get_students', {
            schoolId: 1, yearId: 2, grade: 3, classNo: 6,
        })
    })

    it('학급 목록도 학교 안에서만 찾는다', async () => {
        await useRosterStore().fetchClasses(2)
        expect(invoke).toHaveBeenCalledWith('get_classes', {schoolId: 1, yearId: 2})
    })

    it('미리보기와 저장 모두 학교를 넘긴다 — 재가져오기는 차분이다', async () => {
        const roster = useRosterStore()
        await roster.preview(2, 3, 6, [{number: 1, name: '김하늘'}])
        expect(invoke.mock.calls[0][1].schoolId).toBe(1)

        invoke.mockResolvedValue({added: 1, renamed: 0, withdrawn: 0})
        await roster.apply(2, 3, 6, '2026-03-02', [])
        const applyCall = invoke.mock.calls.find((c) => c[0] === 'apply_roster')
        expect(applyCall[1].schoolId).toBe(1)
    })

    it('학생 수정과 전출도 학교 안에서만 한다', async () => {
        const roster = useRosterStore()
        await roster.updateStudent(9, 5, '김하늘')
        await roster.withdraw(9, '2026-09-10')

        expect(invoke).toHaveBeenCalledWith('update_student', {
            schoolId: 1, id: 9, number: 5, name: '김하늘',
        })
        expect(invoke).toHaveBeenCalledWith('withdraw_student', {
            schoolId: 1, id: 9, date: '2026-09-10',
        })
    })

    it('학급 판단은 파일이 말해 준다 — 학교를 묻지 않는다', async () => {
        await useRosterStore().detectClass([{number: 1, name: '김하늘'}])
        expect(invoke).toHaveBeenCalledWith('detect_roster_class', {
            entries: [{number: 1, name: '김하늘'}],
        })
    })

    it('실패는 error에 담고 다시 던진다', async () => {
        invoke.mockRejectedValue('학생을 찾을 수 없습니다: 9')
        const roster = useRosterStore()

        await expect(roster.updateStudent(9, 5, '김하늘')).rejects.toBeTruthy()
        expect(roster.error).toContain('학생을 찾을 수 없습니다')
    })
})
