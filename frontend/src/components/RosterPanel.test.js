/**
 * 명렬표 파일 열기 — **명단이 붙는 곳은 지금 선택한 학급 하나다.**
 *
 * 예전에는 파일이 말하는 학년 · 반이 그 자리를 결정했다. 학년 · 반 · 번호는 그 학생의
 * 학적이지 소속이 아니므로 지금은 `classId`가 결정하고, 파일이 가리키는 반은
 * **알리기만 한다** — 다른 반 학생이 우리 반 명단에 실리는 일이 실제로 있다.
 *
 * **열쇠는 역할이 결정한다.** 담임은 번호 하나이고, 교과 강좌는 (학년, 반, 번호) 자리
 * 전체다 — 3학년 1반 4번과 3학년 6반 4번이 같은 강좌에 있다. 화면도 그만큼 구별된다:
 * 교과에서만 자리 칸과 반별 인원을 그리고, 학급 판단(`detectClass`)은 묻지 않는다.
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

const HOMEROOM = {
    id: 10, schoolId: 1, yearId: 2, role: 'homeroom', name: '3학년 6반',
    grade: 3, classNo: 6, validTo: null,
}
const SUBJECT = {
    id: 21, schoolId: 1, yearId: 2, role: 'subject', name: '인공지능기초A',
    grade: null, classNo: null, validTo: null,
}

const ENTRIES = [{grade: 3, classNo: 6, number: 1, name: '김하늘', line: 2}]
const DIFF = [{
    key: 0, grade: 3, classNo: 6, line: 2, number: 1,
    incomingName: '김하늘', currentName: null, studentId: null, action: 'added',
}]

/** 저장 결과. 칸을 전부 채워 둔다 — 빠뜨리면 화면에 `undefined명`이 나온다. */
const APPLIED = {
    added: 1, created: 0, renamed: 0, withdrawn: 0, blocked: 0, seatClosed: 0,
}

/** 교과 강좌 미리보기. **같은 번호가 반을 달리해 두 줄 표시된다** — 번호로는 구별하지 못한다. */
const SUBJECT_DIFF = [
    {
        key: 0, grade: 3, classNo: 1, line: 2, number: 4,
        incomingName: '김하늘', currentName: null, studentId: null, action: 'added',
    },
    {
        key: 1, grade: 3, classNo: 6, line: 3, number: 4,
        incomingName: '박서연', currentName: null, studentId: null, action: 'added',
    },
    {
        key: 2, grade: 3, classNo: 6, line: 4, number: 7,
        incomingName: '이도윤', currentName: '이도윤', studentId: 5, action: 'unchanged',
    },
]

/** 파일을 읽은 것처럼 만든다. 파일 형식은 RosterImport가 이미 따로 검사한다. */
async function load(wrapper, result = {}) {
    wrapper.findComponent(RosterImport).vm.$emit('loaded', {
        entries: ENTRIES, parser: 'exceljs', skipped: [], missing: [], inherited: 0, ...result,
    })
    await flushPromises()
}

function build({
    detected = {grade: 3, classNo: 6, mixed: false},
    diff = DIFF,
    applied = APPLIED,
    students = [],
} = {}) {
    const roster = useRosterStore()
    vi.spyOn(roster, 'detectClass').mockResolvedValue(detected)
    vi.spyOn(roster, 'fetchStudents').mockResolvedValue(students)
    vi.spyOn(roster, 'preview').mockResolvedValue(diff)
    vi.spyOn(roster, 'apply').mockResolvedValue(applied)
    return {wrapper: mount(RosterPanel), roster}
}

/** 교과 강좌를 보고 있게 만든다. 화면이 구별되는 것은 학급의 역할 하나다. */
function pickSubject() {
    const app = useAppStore()
    app.classes = [HOMEROOM, SUBJECT]
    app.classId = SUBJECT.id
}

const save = (wrapper) => wrapper.findAll('button').find((b) => b.text() === '저장')

beforeEach(() => {
    setActivePinia(createPinia())

    const app = useAppStore()
    app.booted = true
    app.today = '2026-09-10'
    app.yearId = 2
    app.years = [{id: 2, year: 2026}]
    app.schools = [{id: 1, name: '한빛고등학교', maxSlot: 7, dueDays: 7, dueSkipOffdays: true}]
    app.classes = [HOMEROOM]
    app.classId = HOMEROOM.id
})

describe('명렬표 파일 열기', () => {
    it('미리보기도 저장도 classId 하나로 묻는다', async () => {
        const {wrapper, roster} = build()
        await load(wrapper)

        expect(roster.preview).toHaveBeenCalledWith(10, ENTRIES)

        await save(wrapper).trigger('click')
        await flushPromises()

        expect(roster.apply).toHaveBeenCalledWith(10, '2026-09-10', DIFF)
    })

    it('파일이 다른 반을 가리켜도 막지 않고 알린다', async () => {
        const {wrapper, roster} = build({detected: {grade: 2, classNo: 1, mixed: false}})
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

    it('담임 화면에는 자리 칸을 그리지 않는다 — 같은 반이 서른 번 반복된다', async () => {
        const {wrapper} = build()
        await load(wrapper)

        expect(wrapper.findAll('.row__seat')).toHaveLength(0)
        expect(wrapper.text()).not.toContain('반별 인원')
    })
})

describe('명렬표 파일 열기 — 교과 강좌', () => {
    it('학급 판단을 묻지 않는다 — 반이 섞인 것이 정상이다', async () => {
        pickSubject()
        const {wrapper, roster} = build({diff: SUBJECT_DIFF})
        await load(wrapper)

        expect(roster.detectClass).not.toHaveBeenCalled()
        expect(roster.preview).toHaveBeenCalledWith(SUBJECT.id, ENTRIES)
        expect(wrapper.text()).not.toContain('가리킵니다')
    })

    it('번호가 겹쳐도 줄이 통째로 표시된다 — 목록의 열쇠는 번호가 아니라 key다', async () => {
        pickSubject()
        const {wrapper} = build({diff: SUBJECT_DIFF})
        await load(wrapper)

        // 4번이 두 줄이다. 번호를 키로 쓰면 한 줄이 사라지거나 서로 덮어쓴다.
        expect(wrapper.findAll('.row__seat')).toHaveLength(SUBJECT_DIFF.length)
        expect(wrapper.text()).toContain('김하늘')
        expect(wrapper.text()).toContain('박서연')
    })

    it('줄마다 다른 키를 단다 — 번호를 키로 쓰면 4번 두 줄이 같은 줄이 된다', async () => {
        pickSubject()
        const {wrapper} = build({diff: SUBJECT_DIFF})
        await load(wrapper)

        // Vue가 목록을 짝지을 때 쓰는 키. Rust가 채워 보내는 0..n이 그 자리다.
        const keys = wrapper.findAll('.row').map((row) => row.element.__vnode.key)
        expect(keys).toEqual([0, 1, 2])

        const numbers = SUBJECT_DIFF.map((row) => row.number)
        expect(new Set(numbers).size, '번호는 겹친다').toBeLessThan(numbers.length)
    })

    it('같은 번호 두 줄을 눌러도 서로 섞이지 않는다', async () => {
        pickSubject()
        const {wrapper} = build({diff: SUBJECT_DIFF.map((row) => ({...row}))})
        await load(wrapper)

        const rows = () => wrapper.findAll('.row')
        await rows()[0].find('button').trigger('click')

        expect(rows()[0].find('.row__name').text()).toBe('김하늘')
        expect(rows()[0].find('button').text()).toBe('그대로')
        expect(rows()[1].find('.row__name').text()).toBe('박서연')
        expect(rows()[1].find('button').text()).toBe('새로 들어옴')
    })

    it('자리를 보인다 — 어느 반 4번인지 보이지 않으면 눈으로 맞춰야 한다', async () => {
        pickSubject()
        const {wrapper} = build({diff: SUBJECT_DIFF})
        await load(wrapper)

        const seats = wrapper.findAll('.row__seat').map((s) => s.text())
        expect(seats).toEqual(['3학년 1반', '3학년 6반', '3학년 6반'])
    })

    it('반별 인원을 머리에 적는다 — 교과 교사는 이 숫자로 파일을 확인한다', async () => {
        pickSubject()
        const {wrapper} = build({diff: SUBJECT_DIFF})
        await load(wrapper)

        expect(wrapper.find('.roster__seats').text()).toContain('3학년 1반 1 · 6반 2 — 3명')
    })

    it('지금 명단에도 자리를 보인다 — 미리보기만 고치면 한쪽이 분리된다', async () => {
        pickSubject()
        const {wrapper} = build({
            students: [
                {id: 1, grade: 3, classNo: 1, number: 4, name: '김하늘'},
                {id: 2, grade: 3, classNo: 6, number: 4, name: '박서연'},
            ],
        })
        await flushPromises()

        expect(wrapper.findAll('.row__seat').map((s) => s.text()))
            .toEqual(['3학년 1반', '3학년 6반'])
    })

    it('학년 · 반 열이 통째로 없으면 한 문장으로 멈추고 양식을 옆에 둔다', async () => {
        // 서른 줄을 전부 같은 이유의 blocked로 멈추면 그것이 곧 늘 붙어 있는 경고가 된다.
        pickSubject()
        const {wrapper, roster} = build({diff: SUBJECT_DIFF})
        await load(wrapper, {missing: ['grade', 'classNo']})

        expect(roster.preview).not.toHaveBeenCalled()
        const stop = wrapper.find('.roster__stop')
        expect(stop.text()).toContain('학년 · 반 열이')
        expect(stop.findAll('button').some((b) => b.text() === '예시 파일로 저장')).toBe(true)
    })

    it('담임은 학년 · 반 열이 없어도 그대로 간다 — 열쇠가 번호 하나다', async () => {
        const {wrapper, roster} = build()
        await load(wrapper, {missing: ['grade', 'classNo']})

        expect(wrapper.find('.roster__stop').exists()).toBe(false)
        expect(roster.preview).toHaveBeenCalledWith(10, ENTRIES)
    })
})

describe('명렬표 파일 열기 — 자리를 결정하지 못한 줄', () => {
    /**
     * 줄을 눌러 바꾸는 시험이 있으므로 매번 새로 만든다.
     *
     * **`why`는 Rust가 실제로 내보내는 문장이어야 한다.** 지어낸 문구를 넣어 두면
     * 이 시험이 production에 없는 출력을 "검증"한다 — 이 저장소가 스토어 가짜에
     * 옛 인자 모양을 단언해 초록으로 지나간 적이 있다.
     * 줄 번호는 **문장에 들어 있지 않다.** 화면이 한 곳에서만 붙인다.
     */
    const blocked = () => [
        {
            key: 0, grade: null, classNo: null, line: 5, number: 4,
            incomingName: '김하늘', currentName: null, studentId: 7, action: 'blocked',
            why: '학년 · 반이 비어 있어 어느 반의 4번인지 구별할 수 없습니다. '
                + '파일에서 그 줄을 채운 뒤 다시 열어주세요.',
        },
        {
            key: 1, grade: null, classNo: null, line: 6, number: 5,
            incomingName: '박서연', currentName: null, studentId: null, action: 'blocked',
            why: '같은 자리(3학년 1반 5번)가 파일에 두 줄 이상 있습니다. '
                + '한 자리에 두 학생일 수 없으므로 파일을 확인하세요.',
        },
    ]

    it('이유를 그대로 적고 파일 줄 번호를 함께 보인다', async () => {
        pickSubject()
        const {wrapper} = build({diff: blocked()})
        await load(wrapper)

        const text = wrapper.text()
        expect(text).toContain('5번째 줄 — 학년 · 반이 비어 있어 어느 반의 4번인지')
        expect(text).toContain('6번째 줄 — 같은 자리(3학년 1반 5번)가 파일에')
        expect(wrapper.findAll('.row.is-warn')).toHaveLength(2)
        // 줄 번호는 한 번만 적힌다. Rust와 화면이 각각 붙이면 겹친다.
        expect(text).not.toContain('5번째 줄 — 5번째 줄')
    })

    it('교사가 읽는 문구를 한자어로 적는다 — 순수 한국어 동사는 한 번 멈추게 한다', async () => {
        pickSubject()
        const {wrapper} = build({
            diff: blocked(),
            applied: {added: 0, created: 0, renamed: 0, withdrawn: 0, blocked: 2, seatClosed: 0},
        })
        await load(wrapper)

        expect(wrapper.text()).toContain('대조되지 않아')
        expect(wrapper.text()).not.toContain('짝을 잃어')

        await save(wrapper).trigger('click')
        await flushPromises()

        expect(wrapper.text()).toContain('자리를 결정하지 못해 넘긴 줄 2개')
        expect(wrapper.text()).not.toContain('앉히')
    })

    it('제외를 자동으로 표시하지 않았다는 것을 알린다', async () => {
        // 읽지 못한 줄은 대조에 참여하지 못한다. 그 줄이 가리키던 학생이 대조되지 않아
        // 명단에서 제외되면, 교사는 "한 줄만 못 넣었구나" 하고 저장을 누른다.
        pickSubject()
        const {wrapper} = build({diff: blocked()})
        await load(wrapper)

        expect(wrapper.text()).toContain('제외는 자동으로 표시하지 않았습니다')
    })

    it('학적이 있는 줄만 [명단에만 연결]로 순환한다 — 단추는 줄마다 하나다', async () => {
        pickSubject()
        const {wrapper} = build({diff: blocked()})
        await load(wrapper)

        const buttons = wrapper.findAll('.row__acts button')
        expect(buttons).toHaveLength(2)
        expect(buttons[0].text()).toBe('넘김')
        // 학적이 없는 줄은 넘기는 것 말고 할 일이 없다.
        expect(buttons[1].attributes('disabled')).toBeDefined()

        await buttons[0].trigger('click')
        expect(wrapper.findAll('.row__acts button')[0].text()).toBe('명단에만 연결')

        await wrapper.findAll('.row__acts button')[0].trigger('click')
        expect(wrapper.findAll('.row__acts button')[0].text()).toBe('넘김')
    })
})

/**
 * **무엇으로 읽었는지 알린다.** 파서 이름을 알리는 것과 같은 이유다 — 이름이 깨져
 * 보일 때 파일을 의심할지 해독을 의심할지 알려주는 단서다. 특히 어느 인코딩으로도
 * 깨끗하게 읽히지 않은 파일은 반드시 말한다. 읽히는 만큼 읽되 숨기지 않는다.
 */
describe('명렬표 파일 열기 — 무엇으로 읽었는가', () => {
    const head = (wrapper) => wrapper.find('.ledger__head').text()
    const warnings = (wrapper) => wrapper.findAll('.notice--warn').map((n) => n.text()).join(' ')

    it('CSV를 어느 인코딩으로 읽었는지 머리글에 적는다', async () => {
        const {wrapper} = build()
        await load(wrapper, {parser: 'csv', encoding: 'euc-kr'})

        expect(head(wrapper)).toContain('csv로 읽음')
        expect(head(wrapper)).toContain('euc-kr로 해독')
    })

    it('엑셀은 인코딩을 말하지 않는다 — zip 안은 언제나 UTF-8이라 물을 것이 없다', async () => {
        const {wrapper} = build()
        await load(wrapper, {parser: 'exceljs', encoding: ''})

        expect(head(wrapper)).toContain('exceljs로 읽음')
        // 언제나 붙어 있는 문구는 읽지 않게 된다.
        expect(head(wrapper)).not.toContain('해독')
    })

    it('판별하지 못한 인코딩은 반드시 알린다 — 이름이 깨진 채 들어올 수 있다', async () => {
        const {wrapper} = build()
        await load(wrapper, {parser: 'csv', encoding: '알 수 없음'})

        expect(warnings(wrapper)).toContain('인코딩을 판별하지 못해')
        // 무엇으로 다시 저장하면 되는지까지 적는다.
        expect(warnings(wrapper)).toContain('CP949')
    })

    it('읽힌 인코딩은 경고로 올리지 않는다 — 늘 붙어 있는 경고는 읽지 않게 된다', async () => {
        const {wrapper} = build()
        await load(wrapper, {parser: 'csv', encoding: 'utf-8'})

        expect(warnings(wrapper)).not.toContain('인코딩을 판별하지 못해')
    })

    it('학급을 옮기면 앞 파일의 인코딩이 남지 않는다', async () => {
        const {wrapper} = build()
        await load(wrapper, {parser: 'csv', encoding: '알 수 없음'})
        expect(warnings(wrapper)).toContain('인코딩을 판별하지 못해')

        pickSubject()
        await flushPromises()

        expect(wrapper.text()).not.toContain('인코딩을 판별하지 못해')
    })
})

describe('명렬표 파일 열기 — 저장 결과', () => {
    it('전출이라고 말하지 않는다 — 내 명단에서 제외되는 것과 학교를 떠나는 것은 다르다', async () => {
        const {wrapper} = build({
            diff: [{
                key: 0, grade: 3, classNo: 6, line: null, number: 3,
                incomingName: null, currentName: '이도윤', studentId: 5, action: 'withdrawn',
            }],
        })
        await load(wrapper)

        expect(wrapper.text()).toContain('내 명단에서 제외')
        expect(wrapper.text()).not.toContain('전출로')
    })

    it('학적을 새로 만든 수와 넘긴 줄을 함께 말한다', async () => {
        const {wrapper} = build({
            applied: {added: 3, created: 2, renamed: 1, withdrawn: 0, blocked: 1, seatClosed: 0},
        })
        await load(wrapper)
        await save(wrapper).trigger('click')
        await flushPromises()

        const text = wrapper.text()
        expect(text).toContain('명단에 새로 3명')
        expect(text).toContain('학적을 새로 만든 것 2명')
        expect(text).toContain('자리를 결정하지 못해 넘긴 줄 1개')
        expect(text).not.toContain('undefined')
    })

    it('마감한 학적은 경고 위계로 말한다 — 되돌릴 수 없는 유일한 쓰기다', async () => {
        const {wrapper} = build({
            applied: {added: 1, created: 1, renamed: 0, withdrawn: 0, blocked: 0, seatClosed: 2},
        })
        await load(wrapper)
        await save(wrapper).trigger('click')
        await flushPromises()

        const warn = wrapper.findAll('.notice--warn').map((n) => n.text()).join(' ')
        expect(warn).toContain('학적 2건을 마감했습니다')
        expect(warn).toContain('되돌릴 수 없습니다')
    })

    it('교과에서 전원의 학적을 새로 만들었으면 파일의 반을 의심하라고 적는다', async () => {
        pickSubject()
        const {wrapper} = build({
            diff: SUBJECT_DIFF,
            applied: {added: 3, created: 3, renamed: 0, withdrawn: 0, blocked: 0, seatClosed: 0},
        })
        await load(wrapper)
        await save(wrapper).trigger('click')
        await flushPromises()

        expect(wrapper.text()).toContain('파일의 학년 · 반이 통째로 다른지')
    })

    it('담임에는 그 말을 붙이지 않는다 — 3월에는 전원이 새 학적인 것이 정상이다', async () => {
        const {wrapper} = build({
            applied: {added: 1, created: 1, renamed: 0, withdrawn: 0, blocked: 0, seatClosed: 0},
        })
        await load(wrapper)
        await save(wrapper).trigger('click')
        await flushPromises()

        expect(wrapper.text()).not.toContain('통째로 다른지')
    })
})
