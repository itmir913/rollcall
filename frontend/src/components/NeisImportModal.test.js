/**
 * 나이스 가져오기 창.
 *
 * 화면이 지켜야 하는 것은 셋이다.
 *   · **왼쪽이 내 기록, 오른쪽이 나이스.** NEIS 검증 화면과 같은 배치여야 한다.
 *   · 앱에만 있는 기록을 지운다고 말하지 않는다. 세어서 알리기만 한다.
 *   · 갈래가 비어도 자리를 지운다. 목록이 생겼다 사라지면 단추 자리가 매번 달라진다.
 *
 * 학생 이름은 전부 가짜다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {mount} from '@vue/test-utils'
import NeisImportModal from './NeisImportModal.vue'
import {useNeisImportStore} from '../stores/neisImport'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))

const ITEMS = [
    {
        key: 0, verdict: 'add', number: 5, name: '학생5',
        date: '2026-09-01', dateLabel: '2026.09.01.(화)',
        theirAxis: '질병 결석', theirSpan: '하루 종일', detail: '몸살',
        spanId: null, myAxis: null, mySpan: null, neisDone: false, why: null,
    },
    {
        key: 1, verdict: 'differ', number: 6, name: '학생6',
        date: '2026-09-01', dateLabel: '2026.09.01.(화)',
        theirAxis: '질병 조퇴', theirSpan: '1교시부터 종례까지', detail: null,
        spanId: 11, myAxis: '미인정 조퇴', mySpan: '1교시부터 종례까지',
        neisDone: false, why: null,
    },
    {
        key: 2, verdict: 'unreadable', number: 99, name: '',
        date: '2026-09-01', dateLabel: '2026-09-01',
        theirAxis: '질병결석', theirSpan: '', detail: null,
        spanId: null, myAxis: null, mySpan: null, neisDone: false,
        why: '그날 이 반에 없는 번호입니다. 명렬표를 먼저 맞춰주세요.',
    },
]

const PREVIEW = {
    items: ITEMS,
    same: 4, add: 1, differ: 1, unreadable: 1, onlyMine: 2,
    from: '2026-09-01', to: '2026-09-30',
}

function build(preview = PREVIEW, meta = {parser: 'exceljs', skipped: [], unknownCodes: [], merged: 0}) {
    const store = useNeisImportStore()
    store.preview = preview
    store.meta = meta
    store.picked = {add: new Set([0]), replace: new Set()}
    const wrapper = mount(NeisImportModal, {props: {open: true}, attachTo: document.body})
    return {wrapper, store}
}

const text = () => document.body.textContent
const rows = (tone) => [...document.querySelectorAll(`.cmp.${tone}`)]

beforeEach(() => {
    setActivePinia(createPinia())
    document.body.innerHTML = ''
})

describe('갈래', () => {
    it('세 갈래를 각각의 목록으로 보여준다', () => {
        const {wrapper} = build()
        expect(text()).toContain('앱에 없는 기록')
        expect(text()).toContain('서로 다름')
        expect(text()).toContain('읽지 못한 줄')
        wrapper.unmount()
    })

    it('갈래가 비어도 자리를 지운다', () => {
        const {wrapper} = build({...PREVIEW, items: [], add: 0, differ: 0, unreadable: 0})
        expect(text()).toContain('나이스에만 있는 기록이 없습니다.')
        expect(text()).toContain('어긋난 것이 없습니다.')
        expect(text()).toContain('전부 읽었습니다.')
        wrapper.unmount()
    })

    it('앱에만 있는 기록은 세기만 하고 지운다고 말하지 않는다', () => {
        const {wrapper} = build()
        expect(text()).toContain('앱에만')
        expect(text()).toContain('지우지 않는다')
        wrapper.unmount()
    })
})

describe('두 열', () => {
    it('앱에 없는 기록은 왼쪽이 빈 상자다 — 칸을 없애지 않는다', () => {
        const {wrapper} = build()
        const row = rows('is-only')[0]
        expect(row.querySelector('.side--empty').textContent).toContain('기록 없음')
        expect(row.textContent).toContain('질병 결석')
        wrapper.unmount()
    })

    it('다름은 양쪽을 다 보여준다', () => {
        const {wrapper} = build()
        const row = rows('is-diff')[0]
        expect(row.textContent).toContain('미인정 조퇴')
        expect(row.textContent).toContain('질병 조퇴')
        wrapper.unmount()
    })

    it('못 읽은 줄은 이유를 그대로 적는다. 지어내지 않는다', () => {
        const {wrapper} = build()
        expect(text()).toContain('명렬표를 먼저 맞춰주세요')
        wrapper.unmount()
    })
})

describe('고르기', () => {
    it('줄의 토글이 그 줄만 켠다', async () => {
        const {wrapper, store} = build()
        const toggle = rows('is-diff')[0].querySelector('.mark')
        toggle.click()
        await wrapper.vm.$nextTick()

        expect([...store.picked.replace]).toEqual([1])
        expect([...store.picked.add]).toEqual([0])
        wrapper.unmount()
    })

    it('전부 고르기가 그 갈래만 채운다', async () => {
        const {wrapper, store} = build()
        const heads = [...document.querySelectorAll('.ledger__head')]
        const diffHead = heads.find((h) => h.textContent.includes('서로 다름'))
        diffHead.querySelector('button').click()
        await wrapper.vm.$nextTick()

        expect([...store.picked.replace]).toEqual([1])
        wrapper.unmount()
    })

    it('할 일이 없으면 가져오기를 누를 수 없다', async () => {
        const {wrapper, store} = build({...PREVIEW, items: [], add: 0, differ: 0, same: 0})
        store.picked = {add: new Set(), replace: new Set()}
        store.markNeis = false
        await wrapper.vm.$nextTick()

        const apply = [...document.querySelectorAll('.modal__foot .btn')].at(-1)
        expect(apply.hasAttribute('disabled')).toBe(true)
        wrapper.unmount()
    })
})

describe('알림', () => {
    it('합쳐 내보낸 줄이 있으면 확인하라고 말한다', () => {
        const {wrapper} = build(PREVIEW,
            {parser: 'exceljs', skipped: [], unknownCodes: [], merged: 2})
        expect(text()).toContain('합쳐 내보낸 것이 2건')
        wrapper.unmount()
    })

    it('모르는 표기를 모아 알린다', () => {
        const {wrapper} = build(PREVIEW,
            {parser: 'exceljs', skipped: [], unknownCodes: ['공결'], merged: 0})
        expect(text()).toContain('모르는 출결 표기입니다 — 공결')
        wrapper.unmount()
    })

    it('버린 줄을 조용히 넘기지 않는다', () => {
        const {wrapper} = build(PREVIEW,
            {parser: 'exceljs', skipped: [{line: 7, why: ''}], unknownCodes: [], merged: 0})
        expect(text()).toContain('7번째 줄')
        wrapper.unmount()
    })
})
