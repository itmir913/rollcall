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

    it('집중 등재는 저장할 때 한꺼번에 반영한다', async () => {
        readyApp()
        const pending = usePendingStore()
        await pending.saveFocus([
            {id: 1, reasonId: 10, typeId: 1, slots: ['2'], memo: '늦잠', tagId: null},
        ])

        const called = invoke.mock.calls.map((c) => c[0])
        expect(called).toContain('edit_span')
        expect(called).toContain('set_span_memo')
        expect(called).toContain('set_span_tag')
        expect(called).toContain('set_neis_done')
    })

    it('실패는 error에 담고 다시 던진다', async () => {
        readyApp()
        const pending = usePendingStore()
        invoke.mockRejectedValue('구간을 찾을 수 없습니다: 9')

        await expect(pending.setNeis(9, true)).rejects.toBeTruthy()
        expect(pending.error).toContain('구간을 찾을 수 없습니다')
    })
})
