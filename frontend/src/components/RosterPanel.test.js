/**
 * 명렬표 가져오기 — **명단이 붙는 곳은 지금 고른 학급 하나다.**
 *
 * 예전에는 파일이 말하는 학년 · 반이 그 자리를 정했다. 학년 · 반 · 번호는 그 학생의
 * 학적이지 소속이 아니므로 지금은 `classId`가 정하고, 파일이 가리키는 반은
 * **알리기만 한다** — 다른 반 학생이 우리 반 명단에 실리는 일이 실제로 있다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {flushPromises, mount} from '@vue/test-utils'
import {createPinia, setActivePinia} from 'pinia'
import {useAppStore} from '../stores/app'
import {useRosterStore} from '../stores/roster'
import RosterPanel from './RosterPanel.vue'
import RosterImport from './RosterImport.vue'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn().mockResolvedValue([])}))
vi.mock('@tauri-apps/plugin-dialog', () => ({save: vi.fn(), open: vi.fn()}))

const ENTRIES = [{grade: 3, classNo: 6, number: 1, name: '김하늘'}]
const DIFF = [{number: 1, incomingName: '김하늘', currentName: null, studentId: null, action: 'added'}]

/** 파일을 읽은 것처럼 만든다. 파일 형식은 RosterImport가 이미 따로 검사한다. */
async function load(wrapper) {
    wrapper.findComponent(RosterImport).vm.$emit('loaded', {
        entries: ENTRIES, parser: 'exceljs', skipped: [],
    })
    await flushPromises()
}

function build(detected = {grade: 3, classNo: 6, mixed: false}) {
    const roster = useRosterStore()
    vi.spyOn(roster, 'detectClass').mockResolvedValue(detected)
    vi.spyOn(roster, 'fetchStudents').mockResolvedValue()
    vi.spyOn(roster, 'preview').mockResolvedValue(DIFF)
    vi.spyOn(roster, 'apply').mockResolvedValue({added: 1, renamed: 0, withdrawn: 0})
    return {wrapper: mount(RosterPanel), roster}
}

beforeEach(() => {
    setActivePinia(createPinia())

    const app = useAppStore()
    app.booted = true
    app.today = '2026-09-10'
    app.yearId = 2
    app.years = [{id: 2, year: 2026}]
    app.schools = [{id: 1, name: '한빛고등학교', maxSlot: 7, dueDays: 7, dueSkipOffdays: true}]
    app.classes = [{
        id: 10, schoolId: 1, yearId: 2, role: 'homeroom', name: '3학년 6반',
        grade: 3, classNo: 6, validTo: null,
    }]
    app.classId = 10
})

describe('명렬표 가져오기', () => {
    it('미리보기도 저장도 classId 하나로 묻는다', async () => {
        const {wrapper, roster} = build()
        await load(wrapper)

        expect(roster.preview).toHaveBeenCalledWith(10, ENTRIES)

        const save = wrapper.findAll('button').find((b) => b.text() === '저장')
        await save.trigger('click')
        await flushPromises()

        expect(roster.apply).toHaveBeenCalledWith(10, '2026-09-10', DIFF)
    })

    it('파일이 다른 반을 가리켜도 막지 않고 알린다', async () => {
        const {wrapper, roster} = build({grade: 2, classNo: 1, mixed: false})
        await load(wrapper)

        // 막지 않는다. 미리보기는 그대로 지금 학급으로 간다.
        expect(roster.preview).toHaveBeenCalledWith(10, ENTRIES)
        expect(wrapper.text()).toContain('2학년 1반')
        expect(wrapper.text()).toContain('3학년 6반')
    })

    it('같은 반이면 알리지 않는다 — 늘 붙어 있는 경고는 읽지 않게 된다', async () => {
        const {wrapper} = build()
        await load(wrapper)

        expect(wrapper.text()).not.toContain('가리킵니다')
    })
})
