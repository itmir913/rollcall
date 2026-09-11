/**
 * 교과 화면을 실제로 그려 본다.
 *
 * 빌드는 템플릿을 컴파일할 뿐 그리지는 않는다. 여기서 지키는 것은 셋이다.
 *   1. 기록하는 것은 **있었는가 하나뿐**이다 — 구분 · 종류 · 기간 · 서류 · 나이스가 없다.
 *   2. 담임으로 적어 둔 기록은 **읽기 전용 참고**이고, 내가 기록한 결석과 눈에 띄게 구별된다.
 *   3. 개요에 담임의 숫자가 한 줄도 새지 않고, 고정 문구 대신 **실제로 센 값**이 온다.
 *
 * 데이터는 스토어에 직접 넣는다. invoke는 가짜다 — 커맨드 호출은 스토어 테스트가 본다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {flushPromises, mount} from '@vue/test-utils'
import {createPinia, setActivePinia} from 'pinia'
import {createRouter, createWebHashHistory} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useHomeStore} from '../stores/home'
import {useSubjectStore} from '../stores/subject'
import OverviewView from './OverviewView.vue'
import SubjectTodayView from './SubjectTodayView.vue'
import SubjectLogView from './SubjectLogView.vue'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn().mockResolvedValue([])}))

const router = createRouter({
    history: createWebHashHistory(),
    routes: [{path: '/:rest(.*)', component: {template: '<div/>'}}],
})

const SESSION = (over = {}) => ({
    id: 1, date: '2026-09-11', slot: '1', memo: '', absentCount: 0, total: 28, ...over,
})

const ROLL = (over = {}) => ({
    studentId: 11, grade: 3, classNo: 6, number: 5, name: '김하늘',
    absent: false, memo: '', homeroomNote: null, ...over,
})

function render(view) {
    return mount(view, {global: {plugins: [router]}})
}

/** 교과 강좌 하나를 담당하는 교사. 모드는 고른 학급의 역할이 결정한다. */
function subjectTeacher() {
    const app = useAppStore()
    app.booted = true
    app.yearId = 2
    app.today = '2026-09-11'
    app.years = [{id: 2, year: 2026}]
    app.schools = [{id: 1, yearId: 2, name: '한빛고등학교', maxSlot: 7, dueDays: 7}]
    app.classes = [{
        id: 20, schoolId: 1, role: 'subject', name: '인공지능기초 A반',
        grade: null, classNo: null, validTo: null,
    }]
    app.classId = 20
    return app
}

/** 차시 하나를 열어 둔 상태. 명단이 뜨는 화면은 이 상태에서만 만들어진다. */
function openSession(roll = [ROLL()], sessions = [SESSION()]) {
    const subject = useSubjectStore()
    subject.date = '2026-09-11'
    subject.from = '2026-09-11'
    subject.to = '2026-09-11'
    subject.classId = 20
    subject.sessions = sessions
    subject.sessionId = sessions[0].id
    subject.roll = roll
    return subject
}

beforeEach(() => {
    setActivePinia(createPinia())
    subjectTeacher()
})

describe('오늘 수업', () => {
    it('교시를 버튼으로 표시한다 — 최대 교시까지이고 조회 · 종례는 없다', () => {
        const wrapper = render(SubjectTodayView)
        const labels = wrapper.findAll('.pickline .pick').map((b) => b.text())

        // 최대 교시는 학교가 들고 있는 값이라 앱 상수로 박아 두지 않는다.
        expect(labels).toEqual(['1', '2', '3', '4', '5', '6', '7'])
        // 조회 · 종례는 담임이 하루의 양 끝에서 보는 시간이지 누가 가르치는 시간이 아니다.
        expect(labels).not.toContain('조회')
        expect(labels).not.toContain('종례')
    })

    it('이미 만든 교시는 눌러 둔다 — 같은 칸을 또 고르지 않는다', () => {
        openSession([ROLL()], [SESSION({slot: '3'})])
        const wrapper = render(SubjectTodayView)
        const picks = wrapper.findAll('.pickline .pick')

        expect(picks[2].attributes('disabled')).toBeDefined()
        expect(picks[0].attributes('disabled')).toBeUndefined()
    })

    it('고른 교시를 한꺼번에 더한다 — 연강도 두 칸이다', async () => {
        const subject = useSubjectStore()
        subject.date = '2026-09-11'
        const add = vi.spyOn(subject, 'addSessions').mockResolvedValue([1, 2])

        const wrapper = render(SubjectTodayView)
        const picks = wrapper.findAll('.pickline .pick')
        await picks[2].trigger('click')
        await picks[3].trigger('click')
        await wrapper.find('.ledger__head button').trigger('click')

        expect(add).toHaveBeenCalledWith([3, 4])
    })

    it('번호를 누르면 그 자리에서 기록한다 — 확인을 묻지 않는다', async () => {
        const subject = openSession([ROLL(), ROLL({studentId: 12, number: 6, name: '박서연'})])
        const toggle = vi.spyOn(subject, 'toggle').mockResolvedValue(true)

        const wrapper = render(SubjectTodayView)
        const seats = wrapper.findAll('.seat')
        expect(seats).toHaveLength(2)

        await seats[1].trigger('click')
        expect(toggle).toHaveBeenCalledWith(12)
        expect(document.querySelector('.modal')).toBeNull()
    })

    it('기록한 학생만 칸에 색이 들고 빠진 목록에 오른다', () => {
        openSession([
            ROLL({absent: true, memo: '병원'}),
            ROLL({studentId: 12, number: 6, name: '박서연'}),
        ])
        const wrapper = render(SubjectTodayView)

        const seats = wrapper.findAll('.seat')
        expect(seats[0].classes()).toContain('is-marked')
        expect(seats[1].classes()).not.toContain('is-marked')

        const absent = wrapper.findAll('.list--roll .row')
        expect(absent).toHaveLength(1)
        expect(absent[0].text()).toContain('김하늘')
        expect(absent[0].text()).toContain('병원')
    })

    /**
     * 화면에 쓰는 말을 고정한다. `찍다`는 실무 표현이라고 여겨 두었던 말인데 현직
     * 교사가 어색하다고 확인했다 — 교과 화면이 남기는 것은 `기록`이다. 범례는 내
     * 기록과 담임 참고를 구별하는 문장이라 여기서 갈라지면 그 구별부터 흐려진다.
     */
    it('출결을 찍는다고 말하지 않는다 — 내가 기록한 결석이라고 적는다', () => {
        openSession([ROLL({absent: true}), ROLL({studentId: 12, number: 6, name: '박서연'})])
        const text = render(SubjectTodayView).text()

        expect(text).toContain('내가 기록한 결석')
        expect(text).toContain('번호를 누르면 결석이 기록됩니다')
        expect(text).not.toContain('찍')
    })

    it('기록하는 것은 있었는가 하나뿐이다 — 담임의 말이 오지 않는다', () => {
        openSession([ROLL({absent: true})])
        const text = render(SubjectTodayView).text()

        for (const homeroomOnly of ['서류', 'NEIS', '구분', '종류', '체험학습']) {
            expect(text).not.toContain(homeroomOnly)
        }
    })

    it('담임 기록은 읽기 전용 참고로만 보인다 — 내가 기록한 결석과 구별된다', () => {
        openSession([
            ROLL({homeroomNote: '질병결석 · 하루 종일'}),
            ROLL({studentId: 12, number: 6, name: '박서연', absent: true}),
        ])
        const wrapper = render(SubjectTodayView)

        // 참고는 별도 장부에 있고, 그 줄에 읽기 전용이라고 적혀 있다.
        const note = wrapper.findAll('.list--note .row')
        expect(note).toHaveLength(1)
        expect(note[0].text()).toContain('질병결석 · 하루 종일')
        expect(note[0].text()).toContain('읽기 전용')
        expect(note[0].findAll('button')).toHaveLength(0)

        // 내가 기록한 결석 목록에는 담임의 문장이 섞이지 않는다.
        const mine = wrapper.findAll('.list--roll .row')
        expect(mine).toHaveLength(1)
        expect(mine[0].text()).toContain('박서연')
        expect(mine[0].text()).not.toContain('질병결석')

        // 격자에서는 아래 띠로만 말한다. 담임 기록이 있다고 결석이 기록되지는 않는다.
        const seats = wrapper.findAll('.seat')
        expect(seats[0].find('.seat__note').classes()).toContain('is-on')
        expect(seats[0].classes()).not.toContain('is-marked')
        expect(seats[1].find('.seat__note').classes()).not.toContain('is-on')
    })

    it('반이 섞이는 명단이라 학년 · 반을 함께 표시한다 — 번호만으로는 두 학생이 같은 4다', () => {
        openSession([
            ROLL({studentId: 11, grade: 3, classNo: 1, number: 4, name: '김하늘'}),
            ROLL({studentId: 12, grade: 3, classNo: 6, number: 4, name: '박서연', absent: true}),
        ])
        const wrapper = render(SubjectTodayView)

        const seats = wrapper.findAll('.seat')
        expect(seats[0].text()).toContain('3-1')
        expect(seats[1].text()).toContain('3-6')
        expect(seats[0].attributes('title')).toContain('3학년 1반 4번 김하늘')

        // 빠진 목록에도 학적이 온다. 그 줄만 보고 누구인지 알 수 있어야 한다.
        const absent = wrapper.findAll('.list--roll .row')
        expect(absent).toHaveLength(1)
        expect(absent[0].text()).toContain('3학년 6반')
    })

    it('담임 참고 줄에도 학년 · 반이 온다', () => {
        openSession([ROLL({grade: 3, classNo: 1, homeroomNote: '질병결석 · 조회부터 종례까지'})])
        const note = render(SubjectTodayView).findAll('.list--note .row')

        expect(note).toHaveLength(1)
        expect(note[0].text()).toContain('3학년 1반')
    })

    it('차시를 고르기 전에는 없다고 단언하지 않는다 — 읽은 적 없는 날이다', () => {
        const subject = useSubjectStore()
        subject.date = '2026-09-11'
        subject.classId = 20
        subject.sessions = [SESSION()]   // 칸은 있는데 아직 고르지 않았다

        const wrapper = render(SubjectTodayView)

        expect(wrapper.find('.roll .muted').text()).toContain('아직 교시를 고르지 않았습니다')
        expect(wrapper.text()).not.toContain('그날 담임으로 적어 둔 기록이 없습니다')
        expect(wrapper.text()).not.toContain('이 차시에 빠진 학생이 없습니다')
        const empties = wrapper.findAll('.ledger__empty').map((p) => p.text())
        expect(empties.filter((t) => t.includes('아직 교시를 고르지 않았습니다'))).toHaveLength(2)
    })

    it('차시를 고른 뒤 기록이 없으면 그때 없다고 적는다', () => {
        openSession([ROLL()])
        const text = render(SubjectTodayView).text()

        expect(text).toContain('그날 담임으로 적어 둔 기록이 없습니다')
        expect(text).toContain('이 차시에 빠진 학생이 없습니다')
    })

    it('날짜를 옮기면 고른 교시와 열어 둔 차시가 함께 비워진다', async () => {
        const subject = openSession([ROLL()], [SESSION({slot: '3'})])
        vi.spyOn(subject, 'fetchDay').mockResolvedValue([])

        const wrapper = render(SubjectTodayView)
        await wrapper.findAll('.pickline .pick')[0].trigger('click')
        expect(wrapper.findAll('.pickline .pick')[0].classes()).toContain('is-on')

        const picker = wrapper.find('input[type="date"]')
        picker.element.value = '2026-09-12'
        await picker.trigger('change')
        await flushPromises()

        expect(subject.date).toBe('2026-09-12')
        expect(subject.sessionId).toBeNull()
        // 고른 교시는 그 날짜의 것이다. 남으면 교사가 고른 적 없는 칸이 12일에 생긴다.
        expect(wrapper.findAll('.pickline .pick')[0].classes()).not.toContain('is-on')
        expect(wrapper.find('.roll .muted').text()).toContain('아직 교시를 고르지 않았습니다')
    })

    it('차시 지우기는 확인을 거친다 — 그 칸의 결석까지 사라진다', async () => {
        const subject = openSession()
        const remove = vi.spyOn(subject, 'removeSession').mockResolvedValue()

        const wrapper = render(SubjectTodayView)
        const trash = wrapper.find('.list--sess .row__acts button')
        expect(trash.classes()).toContain('btn--danger')
        expect(trash.find('svg.icon').exists()).toBe(true)
        await trash.trigger('click')

        expect(remove).not.toHaveBeenCalled()
        const modal = document.querySelector('.modal')
        expect(modal).not.toBeNull()
        expect(modal.textContent).toContain('되돌릴 수 없습니다')

        const confirm = [...document.querySelectorAll('.modal__foot button')]
            .find((button) => button.textContent.trim() === '지우기')
        confirm.click()
        await flushPromises()

        expect(remove).toHaveBeenCalledWith(1)
        wrapper.unmount()
    })

    it('실패를 화면에 남긴다 — 빈 명단으로 두지 않는다', async () => {
        const subject = useSubjectStore()
        subject.date = '2026-09-11'
        vi.spyOn(subject, 'fetchDay').mockImplementation(async () => {
            subject.error = '차시를 읽지 못했습니다.'
            throw new Error('읽기 실패')
        })

        const wrapper = render(SubjectTodayView)
        await flushPromises()

        expect(wrapper.text()).toContain('차시를 읽지 못했습니다.')
    })
})

describe('수업 기록', () => {
    it('하루가 카드 하나다', () => {
        const subject = useSubjectStore()
        subject.classId = 20
        subject.year = 2026
        subject.month = 9
        subject.sessions = [
            SESSION({id: 1, date: '2026-09-10', slot: '1', absentCount: 2}),
            SESSION({id: 2, date: '2026-09-10', slot: '3'}),
            SESSION({id: 3, date: '2026-09-11', slot: '2', absentCount: 1}),
        ]

        const wrapper = render(SubjectLogView)
        const cards = wrapper.findAll('.ledger')
        expect(cards).toHaveLength(2)
        expect(cards[0].text()).toContain('2026.09.11.(금)')
        expect(cards[1].text()).toContain('2026.09.10.(목)')
        expect(cards.map((card) => card.findAll('.row').length)).toEqual([1, 2])
        expect(cards[1].text()).toContain('빠진 사람 2 / 28명')
    })

    it('월을 누르면 그 달을 읽는다', async () => {
        const subject = useSubjectStore()
        const fetchMonth = vi.spyOn(subject, 'fetchMonth').mockResolvedValue([])

        const wrapper = render(SubjectLogView)
        await flushPromises()
        fetchMonth.mockClear()

        await wrapper.findAll('.filters .pick')[0].trigger('click')
        expect(subject.month).toBe(3)
        expect(fetchMonth).toHaveBeenCalled()
    })

    it('담임의 말이 오지 않는다', () => {
        const subject = useSubjectStore()
        subject.year = 2026
        subject.month = 9
        subject.sessions = [SESSION()]

        const text = render(SubjectLogView).text()
        for (const homeroomOnly of ['서류', 'NEIS', '구분 · 종류', '체험학습']) {
            expect(text).not.toContain(homeroomOnly)
        }
    })

    it('[오늘 수업 열기]가 오늘을 연다 — 지난 차시를 본 뒤라도', async () => {
        const subject = useSubjectStore()
        subject.classId = 20
        subject.year = 2026
        subject.month = 9
        subject.sessions = [SESSION({id: 5, date: '2026-09-03', slot: '2'})]
        // 지난 차시를 한 번 열어 본 상태. 날짜가 그 날에 머물러 있다.
        subject.date = '2026-09-03'
        subject.sessionId = 5
        subject.roll = [ROLL()]
        vi.spyOn(subject, 'fetchMonth').mockResolvedValue([])

        const wrapper = render(SubjectLogView)
        await flushPromises()
        const open = wrapper.findAll('button').find((b) => b.text() === '오늘 수업 열기')
        await open.trigger('click')
        await flushPromises()

        expect(subject.date).toBe('2026-09-11')
        expect(subject.sessionId).toBeNull()
        expect(router.currentRoute.value.path).toBe('/subject/today')
    })

    it('지난 차시를 열면 날짜부터 옮기고 그 칸을 고른다', async () => {
        const subject = useSubjectStore()
        subject.classId = 20
        subject.date = '2026-09-11'
        subject.year = 2026
        subject.month = 9
        subject.sessions = [SESSION({id: 5, date: '2026-09-03', slot: '2'})]
        vi.spyOn(subject, 'fetchMonth').mockResolvedValue([])
        const select = vi.spyOn(subject, 'select').mockResolvedValue()

        const wrapper = render(SubjectLogView)
        await flushPromises()
        await wrapper.find('.list--sesslog .row__acts button').trigger('click')
        await flushPromises()

        // 날짜가 먼저다. 뒤바뀌면 방금 고른 칸을 setDate가 지운다.
        expect(subject.date).toBe('2026-09-03')
        expect(select).toHaveBeenCalledWith(5)
    })
})

/**
 * **`등록하지 않음`과 `고르지 않음`은 다른 상태다.** 강좌를 이미 만든 교사에게
 * "아직 수업을 등록하지 않았습니다"라고 적으면 이미 한 일을 다시 시키는 것이고,
 * 그 화면에서 나갈 길도 없다. 개요가 두 상태를 구분하는 방식을 그대로 쓴다.
 */
describe('교과 화면의 빈 상태', () => {
    it('강좌가 하나도 없으면 등록하러 가는 길을 준다', async () => {
        const app = useAppStore()
        app.classes = []
        app.classId = null
        app.lastMode = 'subject'

        for (const view of [SubjectTodayView, SubjectLogView]) {
            const wrapper = render(view)
            expect(wrapper.text()).toContain('아직 수업을 등록하지 않았습니다')

            const lead = wrapper.find('.lead button')
            expect(lead.text()).toBe('수업 등록하기')
            await lead.trigger('click')
            await flushPromises()
            expect(router.currentRoute.value.path).toBe('/settings')
        }
    })

    it('담당 강좌는 있는데 고르지 않았으면 고르러 가는 길을 준다', async () => {
        const app = useAppStore()
        app.classId = null      // 강좌 목록은 그대로 있다
        app.lastMode = 'subject'

        for (const view of [SubjectTodayView, SubjectLogView]) {
            const wrapper = render(view)
            expect(wrapper.text()).toContain('보고 있는 수업이 없습니다')
            expect(wrapper.text()).not.toContain('아직 수업을 등록하지 않았습니다')

            const lead = wrapper.find('.lead button')
            expect(lead.text()).toBe('수업 고르기')
            await lead.trigger('click')
            await flushPromises()
            expect(router.currentRoute.value.path).toBe('/move')
        }
    })
})

describe('개요 — 교과', () => {
    /** 담임 요약이 남아 있어도 교과 화면에는 오지 않는다는 것을 보려고 고정한다. */
    function staleHomeroomSummary() {
        const home = useHomeStore()
        vi.spyOn(home, 'fetchSummary').mockResolvedValue()
        home.summary = {
            dateLabel: '2026.09.11.(금)', enrolled: 28, recorded: 4, incomplete: 1,
            docPending: 11, docOverdue: 2, neisPending: 6, docRows: [], neisRows: [],
        }
        return home
    }

    it('강좌가 없으면 담임 어휘 대신 수업을 등록하러 가는 길을 준다', () => {
        const app = useAppStore()
        app.classes = []
        app.classId = null
        app.lastMode = 'subject'

        const wrapper = render(OverviewView)
        expect(wrapper.text()).toContain('아직 수업을 등록하지 않았습니다')
        expect(wrapper.text()).not.toContain('명렬표')
        expect(wrapper.find('.lead button').text()).toBe('수업 등록하기')
    })

    it('수업 기록 자리에 실제로 센 값이 온다 — 고정 문구를 두지 않는다', async () => {
        staleHomeroomSummary()
        const subject = useSubjectStore()
        vi.spyOn(subject, 'fetchMonth').mockImplementation(async () => {
            subject.sessions = [
                SESSION({id: 1, date: '2026-09-10', slot: '1', absentCount: 2}),
                SESSION({id: 2, date: '2026-09-11', slot: '2', absentCount: 1}),
                SESSION({id: 3, date: '2026-09-11', slot: '4', absentCount: 0, memo: '수행평가'}),
            ]
            return subject.sessions
        })

        const wrapper = render(OverviewView)
        await flushPromises()

        expect(wrapper.text()).not.toContain('아직 만들지 않았습니다')
        const cells = wrapper.findAll('.strip__cell')
        expect(cells.map((cell) => [cell.find('b').text(), cell.find('span').text()])).toEqual([
            ['2', '오늘 차시'],
            ['1', '오늘 빠진 사람'],
            ['3', '9월 차시'],
            ['2', '9월 수업한 날'],
        ])
        // 오늘의 차시만 줄로 온다. 어제 것은 수업 기록에서 본다.
        const rows = wrapper.findAll('.list--todaysess .row')
        expect(rows).toHaveLength(2)
        expect(rows[1].text()).toContain('수행평가')
    })

    it('담임의 숫자가 한 줄도 새지 않는다', async () => {
        staleHomeroomSummary()
        const subject = useSubjectStore()
        vi.spyOn(subject, 'fetchMonth').mockResolvedValue([])

        const wrapper = render(OverviewView)
        await flushPromises()

        for (const homeroomOnly of ['서류 미제출', 'NEIS 미등재', '재학', '구분 · 종류 미정']) {
            expect(wrapper.text()).not.toContain(homeroomOnly)
        }
        expect(wrapper.text()).toContain('인공지능기초 A반')
    })
})
