/**
 * 화면을 실제로 그려 본다.
 *
 * 빌드는 템플릿을 **컴파일**할 뿐 그리지는 않는다. 존재하지 않는 값을 참조하거나
 * 스토어 계약이 어긋난 곳은 그려 봐야 드러난다 — 그것도 교사가 아니라 여기서 나야 한다.
 *
 * 데이터는 스토어에 직접 넣는다. invoke는 가짜다 — 이 테스트가 보는 것은
 * "받은 것을 제대로 그리는가"이지 커맨드 호출이 아니다(그것은 스토어 테스트가 본다).
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {flushPromises, mount} from '@vue/test-utils'
import {nextTick} from 'vue'
import {createPinia, setActivePinia} from 'pinia'
import {createRouter, createWebHashHistory} from 'vue-router'
import App from '../App.vue'
import {modeRedirect} from '../router'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {useDayStore} from '../stores/day'
import {useHomeStore} from '../stores/home'
import {useLogStore} from '../stores/log'
import {usePendingStore} from '../stores/pending'
import {useSchoolStore} from '../stores/school'
import {useStatsStore} from '../stores/stats'
import OverviewView from './OverviewView.vue'
import TodayView from './TodayView.vue'
import LogView from './LogView.vue'
import DocsView from './DocsView.vue'
import NeisView from './NeisView.vue'
import StatsView from './StatsView.vue'
import MoveView from './MoveView.vue'
import SettingsView from './SettingsView.vue'
import VerifyView from './VerifyView.vue'
import UpdateView from './UpdateView.vue'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn().mockResolvedValue([])}))
vi.mock('@tauri-apps/plugin-dialog', () => ({save: vi.fn(), open: vi.fn()}))
vi.stubGlobal('__APP_VERSION__', '0.0.0')
// jsdom에는 matchMedia가 없다. 테마 composable이 "시스템 따름"을 확인하려고 부른다.
vi.stubGlobal('matchMedia', () => ({matches: false, addEventListener: () => {}}))

const router = createRouter({
    history: createWebHashHistory(),
    routes: [{path: '/:rest(.*)', component: {template: '<div/>'}}],
})

const span = (over = {}) => ({
    id: 1,
    studentId: 11,
    number: 5,
    name: '김하늘',
    date: '2026-09-10',
    dateLabel: '2026.09.10.(목)',
    reasonId: 1,
    typeId: 2,
    reasonLabel: '질병',
    typeLabel: '결석',
    codeLabel: '질병결석',
    startSlot: '조회',
    endSlot: '종례',
    slotPrompt: 'none',
    spanText: '하루 종일',
    tagId: null,
    tagName: null,
    memo: '감기',
    docDone: false,
    docDue: '2026-09-17',
    docDoneOn: null,
    daysOverdue: -7,
    neisDone: false,
    neisDoneOn: null,
    groupId: null,
    complete: true,
    overlapping: false,
    ...over,
})

function render(view) {
    return mount(view, {global: {plugins: [router]}})
}

/**
 * 앱 껍데기를 그린다. 부팅은 막는다 — App.vue가 onMounted에서 부르는 `boot()`는
 * 가짜 invoke의 빈 응답으로 아래에서 넣어 둔 학급을 통째로 덮는다.
 */
function renderApp() {
    vi.spyOn(useAppStore(), 'boot').mockResolvedValue()
    return mount(App, {global: {plugins: [router]}})
}

/** 사이드바에 실제로 보이는 항목. 모드가 구분되는 것이 여기서 드러난다. */
function railLabels(wrapper) {
    return wrapper.findAll('.rail__link').map((link) => link.text())
}

/** 담임과 교과를 함께 맡은 교사. 스위치와 두 목록이 이 상태에서만 만들어진다. */
function bothRoles() {
    const app = useAppStore()
    app.classes = [
        {
            id: 10, schoolId: 1, yearId: 2, role: 'homeroom', name: '3학년 6반',
            grade: 3, classNo: 6, validTo: null,
        },
        {
            id: 20, schoolId: 1, yearId: 2, role: 'subject', name: '화학Ⅰ 3반',
            grade: null, classNo: null, validTo: null,
        },
    ]
    app.lastClassId = {homeroom: 10, subject: 20}
    app.classId = 10
    return app
}

beforeEach(() => {
    setActivePinia(createPinia())

    const app = useAppStore()
    app.booted = true
    app.yearId = 2
    app.today = '2026-09-10'
    app.years = [{id: 2, year: 2026}]
    app.schools = [{id: 1, name: '한빛고등학교', maxSlot: 7, dueDays: 7, dueSkipOffdays: true}]
    // 범위는 classId 하나다. 학년 · 반은 그 학급이 들고 있는 값이라
    // 화면이 네 값을 조합해 "우리 반"을 다시 만들지 않는다.
    app.classes = [{
        id: 10, schoolId: 1, yearId: 2, role: 'homeroom', name: '3학년 6반',
        grade: 3, classNo: 6, validTo: null,
    }]
    app.classId = 10

    const axis = useAxisStore()
    axis.reasons = [{id: 1, label: '질병'}, {id: 4, label: '출석인정'}]
    axis.types = [
        {id: 2, label: '결석', slotPrompt: 'none'},
        {id: 3, label: '지각', slotPrompt: 'end'},
    ]
    axis.memos = ['감기', '장염']

    const school = useSchoolStore()
    school.school = app.school
    school.tags = [{id: 7, name: '체험학습'}]
    school.rules = [{
        id: 1, name: '체험학습 연 20일', tagId: 7, tagName: '체험학습',
        period: 'year', limitN: 20, unit: 'day',
    }]
    school.offDays = [{id: 1, date: '2026-09-15', label: '개교기념일'}]
})

describe('개요', () => {
    it('숫자와 두 목록을 그린다', () => {
        useHomeStore().summary = {
            date: '2026-09-10', dateLabel: '2026.09.10.(목)',
            enrolled: 28, recorded: 4, incomplete: 1,
            docPending: 11, docOverdue: 2, neisPending: 6,
            docRows: [span()], neisRows: [span({id: 2, docDue: null})],
        }
        const wrapper = render(OverviewView)
        expect(wrapper.text()).toContain('개요')
        expect(wrapper.text()).toContain('서류 미제출')
        expect(wrapper.text()).toContain('NEIS 미등재')
        // 칸 수만 세면 미제출(11)과 그중 마감 지남(2)을 바꿔 연결해도 그대로 지나간다.
        // 둘은 뜻이 다른 수라 어느 숫자가 어느 이름 아래 오는지까지 본다.
        const cells = wrapper.findAll('.strip__cell')
        expect(cells).toHaveLength(5)
        expect(cells.map((cell) => [cell.find('b').text(), cell.find('span').text()])).toEqual([
            ['28', '재학'],
            ['4', '오늘 기록'],
            ['1', '구분 · 종류 미정'],
            ['11', '미제출'],
            ['2', '그중 마감 지남'],
        ])
    })

    it('나이스 목록에도 마감 칸이 있다 — 칸을 없애면 두 목록의 눈높이가 어긋난다', () => {
        useHomeStore().summary = {
            enrolled: 1, recorded: 1, incomplete: 0,
            docPending: 1, docOverdue: 0, neisPending: 1,
            docRows: [span()], neisRows: [span({id: 2})], dateLabel: '2026.09.10.(목)',
        }
        const wrapper = render(OverviewView)
        const rows = wrapper.findAll('.row')
        expect(rows).toHaveLength(2)
        // 칸 수만 같으면 순서가 뒤바뀌어도 통과한다. 어느 칸이 몇 번째인지까지 비교한다.
        const columns = (row) => row.findAll('span').map((cell) => cell.attributes('class'))
        expect(columns(rows[1])).toEqual(columns(rows[0]))
        // 서류 쪽 마감은 데이터에서 온다. 나이스 쪽은 마감이 없어 그 자리에 고정 문구가 온다.
        expect(rows[0].find('.row__due').text()).toBe('마감 2026-09-17')
        expect(rows[1].find('.row__due').text()).toBe('마감 없음')
    })

    // **없는 것과 고르지 않은 것은 다음 걸음이 다르다.** 앞은 설정에서 만들어야 하고
    // 뒤는 이동에서 고르기만 하면 된다. 한 문장으로 합치면 이미 학급을 만든 교사를
    // 설정으로 보내 놓고 거기서 또 무엇을 해야 하는지 알려주지 않게 된다.
    // 교과 쪽은 이미 두 갈래로 구분하고 있었다 — 담임도 짝을 맞춘다.
    it('담임 학급이 하나도 없으면 등록하라고 알린다 — 빈 화면으로 두지 않는다', () => {
        const app = useAppStore()
        app.classes = []
        app.classId = null

        const text = render(OverviewView).text()
        expect(text).toContain('담임 학급을 등록하지 않았습니다')
        expect(text).toContain('담임 학급 등록하기')
    })

    /**
     * 화면에 쓰는 말을 고정한다. `출결을 찍다`는 실무 표현이라고 여겨 예외로 두었던
     * 말인데 현직 교사가 어색하다고 확인했다 — `입력하다` · `기록하다`를 쓴다.
     * 빈 상태 문구는 첫 실행 직후에 교사가 가장 먼저 읽는 문장이라 여기서 고정한다.
     */
    it('빈 상태에서 출결을 찍는다고 말하지 않는다', () => {
        const app = useAppStore()
        app.classes = []
        app.classId = null

        const text = render(OverviewView).text()
        expect(text).toContain('출결을 입력할 수 있습니다')
        expect(text).not.toContain('찍')
    })

    it('학급은 있는데 고르지 않았으면 고르라고 알린다 — 설정으로 보내지 않는다', () => {
        useAppStore().classId = null

        const text = render(OverviewView).text()
        expect(text).toContain('보고 있는 학급이 없습니다')
        expect(text).toContain('학급 고르기')
        // 만들라는 말은 하지 않는다. 이미 만들어 둔 교사에게는 틀린 안내다.
        expect(text).not.toContain('담임 학급 등록하기')
    })
})

describe('오늘의 출결', () => {
    it('축 카드와 격자와 목록을 함께 그린다', () => {
        const day = useDayStore()
        day.date = '2026-09-10'
        day.grid = {
            date: '2026-09-10', dateLabel: '2026.09.10.(목)', maxSlot: 7,
            rows: [{studentId: 11, number: 5, name: '김하늘', spans: [span()]}],
            spans: [span()],
        }
        const wrapper = render(TodayView)
        expect(wrapper.find('.axis').exists()).toBe(true)
        expect(wrapper.findAll('.seat')).toHaveLength(1)
        expect(wrapper.text()).toContain('오늘의 출결')
    })

    it('번호를 누르면 그 자리에서 입력한다', async () => {
        const day = useDayStore()
        day.date = '2026-09-10'
        day.grid = {
            dateLabel: '', maxSlot: 7,
            rows: [{studentId: 11, number: 5, name: '김하늘', spans: []}],
            spans: [],
        }
        const stamp = vi.spyOn(day, 'stamp').mockResolvedValue({action: 'added'})

        await render(TodayView).find('.seat').trigger('click')
        expect(stamp).toHaveBeenCalledWith(11)
    })

    it('지우기는 확인을 거친다 — 되돌릴 수 없는 것은 이것뿐이다', async () => {
        const day = useDayStore()
        day.date = '2026-09-10'
        day.grid = {dateLabel: '', maxSlot: 7, rows: [], spans: [span()]}
        const remove = vi.spyOn(day, 'deleteSpan').mockResolvedValue()

        const wrapper = render(TodayView)
        await wrapper.find('.row__acts button').trigger('click')

        expect(remove).not.toHaveBeenCalled()
        expect(document.querySelector('.modal')).not.toBeNull()
        expect(document.querySelector('.modal').textContent).toContain('되돌릴 수 없습니다')
        wrapper.unmount()
    })

    it('가져오기가 실패하면 그 사실을 화면에 남긴다 — 빈 태그 목록으로 두지 않는다', async () => {
        const day = useDayStore()
        day.date = '2026-09-10'
        day.grid = {dateLabel: '', maxSlot: 7, rows: [], spans: []}
        vi.spyOn(day, 'fetchGrid').mockResolvedValue()

        const axis = useAxisStore()
        vi.spyOn(axis, 'fetchMemos').mockImplementation(async () => {
            axis.error = '사유 후보를 읽지 못했습니다.'
            throw new Error('읽기 실패')
        })
        const school = useSchoolStore()
        vi.spyOn(school, 'fetchAll').mockImplementation(async () => {
            school.error = '태그를 읽지 못했습니다.'
            throw new Error('읽기 실패')
        })

        const wrapper = render(TodayView)
        await flushPromises()

        expect(wrapper.text()).toContain('사유 후보를 읽지 못했습니다.')
        expect(wrapper.text()).toContain('태그를 읽지 못했습니다.')
    })
})

describe('출결 기록 · 서류 · NEIS', () => {
    it('출결 기록은 하루가 카드 하나다', () => {
        const log = useLogStore()
        log.year = 2026
        log.month = 9
        log.days = [
            {date: '2026-09-10', dateLabel: '2026.09.10.(목)', enrolled: 28, spans: [span()]},
            {date: '2026-09-09', dateLabel: '2026.09.09.(수)', enrolled: 28, spans: [span({id: 2})]},
        ]
        const wrapper = render(LogView)
        // 하루가 카드 하나다. 두 날을 한 카드에 몰면 카드가 하나로 줄고, 날짜 머리글이
        // 목록 중간에 섞여 어디까지가 그날인지 흐려진다.
        const cards = wrapper.findAll('.ledger')
        expect(cards).toHaveLength(2)
        expect(cards[0].text()).toContain('2026.09.10.(목)')
        expect(cards[1].text()).toContain('2026.09.09.(수)')
        expect(cards.map((card) => card.findAll('.row').length)).toEqual([1, 1])
    })

    it('머리말은 학급 이름 그대로다 — 화면이 학년 · 반을 다시 조합하지 않는다', () => {
        useLogStore().days = []
        expect(render(LogView).text()).toContain('3학년 6반')
        expect(render(StatsView).text()).toContain('3학년 6반')
    })

    it('서류 화면은 모은 명단을 그대로 보여준다 — 받은 줄도 자리에 남는다', () => {
        const pending = usePendingStore()
        pending.docRows = [span(), span({id: 2, docDone: true, number: 12, name: '박서연'})]

        const wrapper = render(DocsView)
        expect(wrapper.findAll('.row')).toHaveLength(2)
        expect(wrapper.text()).toContain('못 받은 것 1건')
        expect(wrapper.text()).toContain('모은 명단 2건')
    })

    it('NEIS 화면은 날짜별 카드에 한 명씩 등재 버튼을 둔다', () => {
        const pending = usePendingStore()
        pending.neisDays = [{
            date: '2026-09-09', dateLabel: '2026.09.09.(수)', enrolled: 28,
            spans: [span({id: 3})],
        }]
        const wrapper = render(NeisView)
        expect(wrapper.text()).toContain('한 명씩 등재')
        expect(wrapper.text()).toContain('이 날짜 전부 등재')
        expect(wrapper.find('.copy').exists()).toBe(true)
    })
})

describe('설정', () => {
    /** 읽기를 고정한다. 화면을 그린 직후 스토어가 빈 응답으로 덮이면 볼 것이 없다. */
    function freezeFetches() {
        vi.spyOn(useSchoolStore(), 'fetchAll').mockResolvedValue()
        vi.spyOn(useAxisStore(), 'fetchAll').mockResolvedValue()
    }

    it('휴업일 지우기는 확인을 거친다 — 행이 사라지는 DELETE다', async () => {
        freezeFetches()
        const remove = vi.spyOn(useSchoolStore(), 'removeOffDay').mockResolvedValue()

        const wrapper = render(SettingsView)
        await wrapper.find('[title="휴업일 지우기"]').trigger('click')

        expect(remove).not.toHaveBeenCalled()
        expect(document.querySelector('.modal').textContent).toContain('되돌릴 수 없습니다')

        const confirm = [...document.querySelectorAll('.modal__foot button')]
            .find((button) => button.textContent.trim() === '지우기')
        confirm.click()
        await flushPromises()

        expect(remove).toHaveBeenCalledWith(1)
        wrapper.unmount()
    })

    it('지우기는 글자가 아니라 휴지통 모양 + 경고색이다', () => {
        freezeFetches()
        const wrapper = render(SettingsView)
        const trash = wrapper.find('[title="휴업일 지우기"]')

        expect(trash.classes()).toContain('btn--danger')
        expect(trash.find('svg.icon').exists()).toBe(true)
        expect(trash.text()).toBe('')
    })

    it('설정을 못 읽으면 그 사실을 화면에 남긴다', async () => {
        const school = useSchoolStore()
        vi.spyOn(school, 'fetchAll').mockImplementation(async () => {
            school.error = '학교 설정을 읽지 못했습니다.'
            throw new Error('읽기 실패')
        })
        const axis = useAxisStore()
        vi.spyOn(axis, 'fetchAll').mockImplementation(async () => {
            axis.error = '출결 구분을 읽지 못했습니다.'
            throw new Error('읽기 실패')
        })

        const wrapper = render(SettingsView)
        await flushPromises()

        expect(wrapper.text()).toContain('학교 설정을 읽지 못했습니다.')
        expect(wrapper.text()).toContain('출결 구분을 읽지 못했습니다.')
    })
})

describe('담임 · 교과 모드', () => {
    /** 교과 모드의 사이드바. 담임 항목이 한 줄도 없어야 한다. */
    const SUBJECT_RAIL = ['개요', '이동', '오늘 수업', '수업 기록', '설정', '업데이트 확인']

    const HOMEROOM_RAIL = [
        '개요', '이동',
        '오늘의 출결', '출결 기록', '서류 미제출자', 'NEIS 미등재',
        '통계', 'NEIS 검증',
        '설정', '업데이트 확인',
    ]

    it('모드를 바꾸면 사이드바가 통째로 갈린다 — 교과에 담임 항목이 하나도 없다', async () => {
        const app = bothRoles()
        const wrapper = renderApp()
        await flushPromises()

        expect(railLabels(wrapper)).toEqual(HOMEROOM_RAIL)

        // 모드는 고른 학급이 결정한다. 교과 강좌로 옮기면 사이드바가 함께 바뀐다.
        app.classId = 20
        await nextTick()

        expect(railLabels(wrapper)).toEqual(SUBJECT_RAIL)
        // 목록 비교만으로도 걸리지만, 무엇이 새면 안 되는지를 문장으로 남긴다.
        for (const homeroomOnly of ['오늘의 출결', '출결 기록', '서류 미제출자', 'NEIS 미등재', '통계', 'NEIS 검증']) {
            expect(wrapper.text()).not.toContain(homeroomOnly)
        }
    })

    it('한쪽만 맡아도 스위치를 그린다 — 숨기면 그 모드가 있다는 것조차 알 수 없다', async () => {
        // **의도 3.** 비담임 교사에게 스위치를 숨기면 담임 기능이 이 앱에 있다는 사실을
        // 화면에서 알 길이 없고, 나중에 담임을 담당해도 들어갈 입구가 없다.
        const wrapper = renderApp()
        await flushPromises()

        expect(wrapper.find('.rail__switch').exists()).toBe(true)
        expect(wrapper.findAll('.rail__mode').map((b) => b.text())).toEqual(['담임', '교과'])
    })

    it('담당 학급 · 강좌가 없는 모드는 사이드바를 비활성화하고 등록할 길을 알린다', async () => {
        // 지우지 않고 잠근다. 지우면 무엇이 있었는지 알 수 없다.
        const app = useAppStore()
        app.classes = [{
            id: 20, schoolId: 1, role: 'subject', name: '화학Ⅰ 3반',
            grade: null, classNo: null, validTo: null,
        }]
        app.lastClassId = {homeroom: null, subject: 20}
        app.classId = 20

        const wrapper = renderApp()
        await flushPromises()

        const homeroom = wrapper.findAll('.rail__mode').find((b) => b.text() === '담임')
        await homeroom.trigger('click')
        await flushPromises()

        // 담임 항목이 사라지지 않고, 눌리지도 않는다.
        expect(wrapper.text()).toContain('담임 학급')
    })

    it('둘 다 맡으면 스위치가 뜨고, 누르면 그 모드로 옮긴다', async () => {
        const app = bothRoles()
        const setMode = vi.spyOn(app, 'setMode').mockImplementation(async (mode) => {
            app.lastMode = mode
            app.classId = app.lastClassId[mode]
        })

        const wrapper = renderApp()
        await flushPromises()

        const modes = wrapper.findAll('.rail__mode')
        expect(modes.map((button) => button.text())).toEqual(['담임', '교과'])
        expect(modes[0].classes()).toContain('is-on')

        await modes[1].trigger('click')
        await flushPromises()

        expect(setMode).toHaveBeenCalledWith('subject')
        expect(railLabels(wrapper)).toEqual(SUBJECT_RAIL)
    })

    it('학급 이름 자리를 누르면 이동 화면으로 간다', async () => {
        bothRoles()
        const wrapper = renderApp()
        await flushPromises()

        const here = wrapper.find('.rail__here')
        expect(here.text()).toContain('3학년 6반')
        expect(here.attributes('href')).toContain('/move')
    })

    it('교과 모드에서 담임 화면 주소로 들어오면 개요로 되돌린다', () => {
        const app = bothRoles()
        app.classId = 20

        expect(modeRedirect(app, {meta: {mode: 'homeroom'}})).toEqual({name: 'overview'})
        // 두 모드가 함께 쓰는 화면은 그대로 통과한다.
        expect(modeRedirect(app, {meta: {}})).toBe(true)

        app.classId = 10
        expect(modeRedirect(app, {meta: {mode: 'homeroom'}})).toBe(true)
    })
})

describe('이동', () => {
    it('현재 모드의 것만 나열한다 — 모드를 넘나드는 길은 스위치뿐이다', async () => {
        const app = bothRoles()
        const wrapper = render(MoveView)

        const names = () => wrapper.findAll('.move__name').map((cell) => cell.text())
        expect(names()).toEqual(['3학년 6반'])

        app.classId = 20
        await nextTick()
        expect(names()).toEqual(['화학Ⅰ 3반'])
    })

    it('지금 있는 곳을 표시하고, 다른 줄을 누르면 옮긴다', async () => {
        const app = bothRoles()
        app.classes.push({
            id: 11, schoolId: 1, yearId: 2, role: 'homeroom', name: '3학년 7반',
            grade: 3, classNo: 7, validTo: null,
        })
        const select = vi.spyOn(app, 'selectClass').mockResolvedValue()

        const wrapper = render(MoveView)
        const rows = wrapper.findAll('.move')
        expect(rows[0].classes()).toContain('is-on')
        expect(rows[0].find('.move__here').text()).toBe('지금 보는 중')
        // 빈 줄에도 칸은 남는다. 접히면 줄마다 열 폭이 달라져 목록이 흔들린다.
        expect(rows[1].find('.move__here').exists()).toBe(true)
        expect(rows[1].find('.move__here').text()).toBe('')

        await rows[1].trigger('click')
        expect(select).toHaveBeenCalledWith(11)

        // 이미 보고 있는 줄은 누를 것이 없다.
        await rows[0].trigger('click')
        expect(select).toHaveBeenCalledTimes(1)
    })

    it('학교 이름표는 학교가 둘 이상일 때만 붙는다', async () => {
        const app = bothRoles()
        expect(render(MoveView).find('.move__school').exists()).toBe(false)

        app.schools = [...app.schools, {id: 2, name: '푸른중학교', maxSlot: 6}]
        await nextTick()
        expect(render(MoveView).find('.move__school').text()).toBe('한빛고등학교')
    })
})

describe('통계 · 검증 · 업데이트', () => {
    it('한도 칸을 한도 수만큼 그리고 쓴 날만 채운다', () => {
        useStatsStore().reports = [{
            rule: {id: 1, name: '체험학습 연 20일', tagName: '체험학습', period: 'year', limitN: 20, unit: 'day'},
            rows: [{
                studentId: 11, number: 5, name: '김하늘', used: 2, limitN: 20,
                dates: ['2026-03-02', '2026-04-01'], buckets: [], state: 'ok',
            }],
            usedTotal: 2, nearCount: 0, overCount: 0, untagged: [span({id: 9})],
        }]
        const wrapper = render(StatsView)
        expect(wrapper.findAll('.dot')).toHaveLength(20)
        expect(wrapper.findAll('.dot.is-used')).toHaveLength(2)
        expect(wrapper.text()).toContain('태그가 없는 출결')
    })

    it('검증은 파일을 읽기 전에 마법사를 보여준다', () => {
        const wrapper = render(VerifyView)
        expect(wrapper.find('.drop').exists()).toBe(true)
        expect(wrapper.findAll('.step')).toHaveLength(3)
    })

    it('업데이트 화면은 확인한 적 없는 것을 확인했다고 말하지 않는다', () => {
        const text = render(UpdateView).text()
        expect(text).toContain('연결되지 않았습니다')
        expect(text).not.toContain('최신')
    })
})
