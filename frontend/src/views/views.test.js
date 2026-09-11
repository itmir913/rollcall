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
import {createPinia, setActivePinia} from 'pinia'
import {createRouter, createWebHashHistory} from 'vue-router'
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

beforeEach(() => {
    setActivePinia(createPinia())

    const app = useAppStore()
    app.booted = true
    app.schoolId = 1
    app.yearId = 2
    app.grade = 3
    app.classNo = 6
    app.today = '2026-09-10'
    app.years = [{id: 2, year: 2026}]
    app.school = {id: 1, name: '한빛고등학교', maxSlot: 7, dueDays: 7, dueSkipOffdays: true}

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
        // 칸 수만 세면 미제출(11)과 그중 마감 지남(2)을 바꿔 이어도 그대로 지나간다.
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

    it('명단이 없으면 무엇을 해야 하는지 알린다 — 빈 화면으로 두지 않는다', () => {
        useAppStore().grade = null
        expect(render(OverviewView).text()).toContain('명단이 없습니다')
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

    it('번호를 누르면 그 자리에서 찍는다', async () => {
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
    /** 읽기를 멈춰 세운다. 화면을 그린 직후 스토어가 빈 응답으로 덮이면 볼 것이 없다. */
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
