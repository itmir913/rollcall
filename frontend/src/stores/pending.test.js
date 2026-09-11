/**
 * 서류 미제출자 · NEIS 미등재 스토어.
 *
 * 이 화면의 핵심 동작은 하나다 — **체크해도 줄이 사라지지 않는다.**
 * 방금 누른 것이 눈앞에서 사라지면 잘못 눌렀는지 확인할 방법이 없다.
 * 그래서 표시를 바꾼 뒤 목록을 다시 부르지 않고 그 줄만 고친다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {usePendingStore} from './pending'
import {useAppStore} from './app'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))

function readyApp() {
    const app = useAppStore()
    app.schoolId = 1
    app.yearId = 2
    app.grade = 3
    app.classNo = 6
    app.today = '2026-09-10'
}

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
    invoke.mockResolvedValue([])
})

describe('서류 미제출자', () => {
    it('오늘 날짜를 화면이 넘긴다 — 커맨드가 시계를 읽지 않는다', async () => {
        readyApp()
        const pending = usePendingStore()
        await pending.fetchDocs()

        expect(invoke).toHaveBeenCalledWith('get_doc_pending', {
            schoolId: 1, yearId: 2, grade: 3, classNo: 6,
            year: null, month: null, includeDone: false, today: '2026-09-10',
        })
    })

    it('체크한 줄은 목록에서 사라지지 않는다', async () => {
        readyApp()
        const pending = usePendingStore()
        pending.docRows = [
            {id: 1, docDone: false, name: '김하늘'},
            {id: 2, docDone: false, name: '박서연'},
        ]

        await pending.setDoc(1, true)

        expect(pending.docRows).toHaveLength(2)
        expect(pending.docRows[0].docDone).toBe(true)
        expect(pending.docRows[0].docDoneOn).toBe('2026-09-10')
        // 목록을 다시 부르지 않는다 — 다시 부르면 그 줄이 빠진다.
        expect(invoke.mock.calls.map((c) => c[0])).not.toContain('get_doc_pending')
    })

    it('못 받은 것만 세어 준다', () => {
        const pending = usePendingStore()
        pending.docRows = [{id: 1, docDone: true}, {id: 2, docDone: false}]
        expect(pending.docLeft).toBe(1)
    })
})

describe('NEIS 미등재', () => {
    it('날짜 묶음 안의 줄도 그 자리에서 고친다', async () => {
        readyApp()
        const pending = usePendingStore()
        pending.neisDays = [{date: '2026-09-09', spans: [{id: 9, neisDone: false}]}]

        await pending.setNeis(9, true)

        expect(pending.neisDays[0].spans[0].neisDone).toBe(true)
        expect(pending.neisDays[0].spans).toHaveLength(1)
    })

    it('이 날짜 전부 등재는 그 카드만 바꾼다', async () => {
        readyApp()
        const pending = usePendingStore()
        pending.neisDays = [
            {date: '2026-09-09', spans: [{id: 1, neisDone: false}, {id: 2, neisDone: false}]},
            {date: '2026-09-08', spans: [{id: 3, neisDone: false}]},
        ]
        invoke.mockResolvedValue(2)

        await pending.markDay('2026-09-09')

        expect(pending.neisDays[0].spans.every((s) => s.neisDone)).toBe(true)
        expect(pending.neisDays[1].spans[0].neisDone).toBe(false)
    })

    /**
     * 커맨드 이름만 세면 `spanId`가 틀려도, 메모가 빠져도 통과한다. 고친 값이
     * 그대로 넘어가는지까지 본다 — 저장이 조용히 절반만 되는 것이 이 화면의 위험이다.
     */
    it('집중 등재는 한 커맨드로 한꺼번에 저장한다', async () => {
        readyApp()
        const pending = usePendingStore()
        await pending.saveFocus([
            {id: 1, reasonId: 10, typeId: 1, slots: ['2'], memo: '늦잠', tagId: null},
            {id: 2, reasonId: 11, typeId: 3, slots: [], memo: '', tagId: 7},
        ])

        expect(invoke).toHaveBeenCalledWith('save_focus_entries', {
            entries: [
                {spanId: 1, reasonId: 10, typeId: 1, slots: ['2'], memo: '늦잠', tagId: null},
                {spanId: 2, reasonId: 11, typeId: 3, slots: [], memo: '', tagId: 7},
            ],
            today: '2026-09-10',
        })
        // 저장이 끝나야 목록을 다시 모은다. 등재한 날은 미등재 목록에서 빠져야 한다.
        expect(invoke.mock.calls.map((c) => c[0])).toEqual([
            'save_focus_entries',
            'get_neis_pending',
        ])
    })

    /**
     * 한 건이라도 실패하면 Rust가 통째로 되돌린다. 그래서 화면의 목록은
     * 고치기 전 그대로가 맞다 — 절반만 반영된 목록을 그려 두면 교사가 무엇을
     * 다시 넣어야 하는지 알 수 없다.
     */
    it('집중 등재가 실패하면 목록을 건드리지 않고 다시 던진다', async () => {
        readyApp()
        const pending = usePendingStore()
        pending.neisDays = [{date: '2026-09-09', spans: [{id: 1, neisDone: false, memo: ''}]}]
        invoke.mockRejectedValue('출결 기록을 찾을 수 없습니다: 2')

        await expect(
            pending.saveFocus([{id: 1, reasonId: 10, typeId: 1, slots: ['2'], memo: '늦잠'}]),
        ).rejects.toBeTruthy()

        expect(pending.error).toContain('출결 기록을 찾을 수 없습니다')
        expect(pending.neisDays[0].spans[0]).toEqual({id: 1, neisDone: false, memo: ''})
        // 실패한 저장 뒤에 목록을 다시 부르지 않는다 — 들어간 것이 없다.
        expect(invoke.mock.calls.map((c) => c[0])).toEqual(['save_focus_entries'])
    })

    it('실패는 error에 담고 다시 던진다', async () => {
        readyApp()
        const pending = usePendingStore()
        invoke.mockRejectedValue('구간을 찾을 수 없습니다: 9')

        await expect(pending.setNeis(9, true)).rejects.toBeTruthy()
        expect(pending.error).toContain('구간을 찾을 수 없습니다')
    })
})
