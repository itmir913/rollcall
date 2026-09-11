/**
 * 나이스 파일 열기 스토어.
 *
 * 지키려는 것은 셋이다.
 *   · **일 · 월을 묻지 않는다** — 기간은 파일 안에 있으므로 넘기지 않는다.
 *   · 처음에 켜 두는 것은 **추가뿐**이다. 덮어쓰기는 교사가 스스로 선택한다.
 *   · 못 읽으면 던진다. 조용히 빈 결과를 내면 "결석 없는 달"과 구별되지 않는다.
 *
 * 학생 이름은 전부 가짜다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useNeisImportStore} from './neisImport'
import {useAppStore} from './app'
import {useAxisStore} from './axis'
import {readNeisFile} from '../services/neisFile'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))
vi.mock('../services/neisFile', async (original) => ({
    ...(await original()),
    readNeisFile: vi.fn(),
}))

const ROWS = [
    {number: 5, name: '학생5', date: '2026-09-01', reasonLabel: '질병', typeLabel: '결석'},
    {number: 6, name: '학생6', date: '2026-09-01', reasonLabel: '질병', typeLabel: '조퇴'},
]
const META = {parser: 'exceljs', from: '2026-09-01', to: '2026-09-01', skipped: [], unknownCodes: [], merged: 0}
const RESULT = {added: 1, replaced: 1, marked: 0, skipped: 0}
const PREVIEW = {
    items: [
        {key: 0, verdict: 'add', number: 5},
        {key: 1, verdict: 'differ', number: 6, spanId: 11},
    ],
    same: 0, add: 1, differ: 1, unreadable: 0, onlyMine: 0,
    from: '2026-09-01', to: '2026-09-01',
}

function ready() {
    const app = useAppStore()
    app.schools = [{id: 1, name: '한빛고등학교', maxSlot: 7}]
    app.yearId = 2
    app.classes = [{
        id: 9, schoolId: 1, yearId: 2, role: 'homeroom', name: '3학년 6반',
        grade: 3, classNo: 6, validTo: null,
    }]
    app.classId = 9
    app.today = '2026-09-11'
    const axis = useAxisStore()
    axis.reasons = [{id: 1, label: '질병'}]
    axis.types = [{id: 1, label: '결석', slotPrompt: 'none'}]
}

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
    readNeisFile.mockReset()
    readNeisFile.mockResolvedValue({rows: ROWS, meta: META})
    invoke.mockResolvedValue(PREVIEW)
})

describe('파일 읽기', () => {
    it('구분 · 종류를 파서에 넘긴다 — 후보는 DB에서 온다', async () => {
        ready()
        await useNeisImportStore().load(new Uint8Array([1]))

        expect(readNeisFile).toHaveBeenCalledWith(expect.anything(), {
            reasons: [{id: 1, label: '질병'}],
            types: [{id: 1, label: '결석', slotPrompt: 'none'}],
        })
    })

    it('일 · 월을 묻지 않는다. 기간은 파일 안에 있다', async () => {
        ready()
        await useNeisImportStore().load(new Uint8Array([1]))

        expect(invoke).toHaveBeenCalledWith('preview_neis_import', {
            classId: 9, rows: ROWS, today: '2026-09-11',
        })
    })

    it('추가만 미리 선택해 둔다. 덮어쓰기는 교사가 선택한다', async () => {
        ready()
        const store = useNeisImportStore()
        await store.load(new Uint8Array([1]))

        expect([...store.picked.add]).toEqual([0])
        expect([...store.picked.replace]).toEqual([])
    })

    it('못 읽으면 던지고 이유를 남긴다', async () => {
        ready()
        readNeisFile.mockRejectedValue(new Error('아직 분석하지 않은 서식입니다'))
        const store = useNeisImportStore()

        await expect(store.load(new Uint8Array([1]))).rejects.toThrow()
        expect(store.error).toContain('아직 분석하지 않은 서식')
        expect(store.preview).toBeNull()
    })
})

describe('선택', () => {
    it('한 번 더 누르면 뺀다', async () => {
        ready()
        const store = useNeisImportStore()
        await store.load(new Uint8Array([1]))

        store.toggle('add', 0)
        expect(store.picked.add.has(0)).toBe(false)
        store.toggle('add', 0)
        expect(store.picked.add.has(0)).toBe(true)
    })

    it('아무것도 안 선택했고 등재 표시도 껐으면 할 일이 없다', async () => {
        ready()
        const store = useNeisImportStore()
        await store.load(new Uint8Array([1]))

        store.pickAll('add', [])
        store.markNeis = false
        expect(store.hasWork).toBe(false)
    })

    it('같은 것이 있으면 등재 표시만으로도 할 일이 된다', async () => {
        ready()
        invoke.mockResolvedValue({...PREVIEW, items: [], add: 0, differ: 0, same: 3})
        const store = useNeisImportStore()
        await store.load(new Uint8Array([1]))

        expect(store.hasWork).toBe(true)
    })
})

describe('적용', () => {
    it('선택한 것과 파일의 줄을 그대로 넘긴다', async () => {
        ready()
        const store = useNeisImportStore()
        await store.load(new Uint8Array([1]))
        store.toggle('replace', 1)

        invoke.mockResolvedValue(RESULT)
        await store.apply()

        expect(invoke).toHaveBeenLastCalledWith('apply_neis_import', {
            classId: 9,
            rows: ROWS,
            choice: {add: [0], replace: [1], markNeis: true},
            today: '2026-09-11',
        })
    })

    it('실패를 삼키지 않는다', async () => {
        ready()
        const store = useNeisImportStore()
        await store.load(new Uint8Array([1]))

        invoke.mockRejectedValue('명렬표를 먼저 맞춰주세요')
        await expect(store.apply()).rejects.toBeTruthy()
        expect(store.error).toContain('명렬표')
    })
})

describe('파일이 말하는 것을 화면에 그대로 넘긴다', () => {
    it('버린 줄 · 모르는 표기 · 합쳐진 줄을 store에 담는다', async () => {
        // 이 셋이 "조용히 넘기지 않는다"의 전부다. store가 흘려보내면 창이 말할 것이 없다.
        ready()
        readNeisFile.mockResolvedValue({
            rows: ROWS,
            meta: {...META, skipped: [{line: 7, why: '번호를 읽지 못했습니다.'}],
                unknownCodes: ['공결'], merged: 1},
        })
        const store = useNeisImportStore()
        await store.load(new Uint8Array([1]))

        expect(store.meta.skipped).toEqual([{line: 7, why: '번호를 읽지 못했습니다.'}])
        expect(store.meta.unknownCodes).toEqual(['공결'])
        expect(store.meta.merged).toBe(1)
        expect(store.meta.parser).toBe('exceljs')
    })
})
