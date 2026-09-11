/**
 * 첫 실행 화면 — 다섯 단계.
 *
 * 여기서 고정하는 것은 다섯이다.
 *   1. 단계를 눌러 앞뒤로 오간다. 되돌아갈 길이 없으면 잘못 적은 교사가 앱을 껐다 켠다.
 *   2. 담당 학급 · 강좌를 하나도 추가하지 않으면 완료로 갈 수 없다. **단추는 잠글 뿐 숨기지 않는다.**
 *   3. 교과 강좌에는 학년 · 반 칸이 없다. 선택과목은 반이 섞여 가리킬 반이 없다.
 *   4. 담임과 교과가 **같은 명렬표 화면**을 쓴다. 다만 교과는 그 강좌를 선택하지 않는다 —
 *      선택하면 모드가 저장되어 다음 실행이 교과 모드로 열린다.
 *   5. 완료 단계의 요약은 **실제로 등록한 것**을 말한다. 고정 문구를 그리면 아무도 모른다.
 *   6. 학년도 삭제는 **학교를 내리는 것과 다른 말을 쓴다.** 학교는 행이 남지만 학년도는
 *      기록까지 지운다. 무엇이 함께 사라지는지 수로 먼저 보여주고 한 번 묻는다.
 *
 * 데이터는 스토어에 직접 넣는다. invoke는 가짜다 — 이 화면이 커맨드를 제대로 부르는지는
 * 스토어 테스트가 본다(`stores/app.test.js`).
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {flushPromises, mount} from '@vue/test-utils'
import {createPinia, setActivePinia} from 'pinia'
import {createRouter, createWebHashHistory} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useRosterStore} from '../stores/roster'
import {useSchoolStore} from '../stores/school'
import WelcomeView from './WelcomeView.vue'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn().mockResolvedValue([])}))
vi.mock('@tauri-apps/plugin-dialog', () => ({save: vi.fn(), open: vi.fn()}))

const router = createRouter({
    history: createWebHashHistory(),
    routes: [{path: '/:rest(.*)', component: {template: '<div/>'}}],
})

const HOMEROOM = {
    id: 10, schoolId: 1, yearId: 2, role: 'homeroom', name: '3학년 6반',
    grade: 3, classNo: 6, validTo: null,
}
const SUBJECT = {
    id: 21, schoolId: 1, yearId: 2, role: 'subject', name: '인공지능기초A',
    grade: null, classNo: null, validTo: null,
}

/**
 * 학년도 한 줄. **뒤의 다섯 수는 "지우면 무엇이 함께 사라지는가"다** — 목록(`get_years`)에
 * 실려 오므로 확인 대화상자가 묻기 전에 그대로 표시한다. 다섯을 모두 다른 값으로 둔다.
 * 같은 값이면 화면이 엉뚱한 수를 그려도 시험이 통과한다.
 */
const YEAR_2026 = {
    id: 2, year: 2026, startsOn: '2026-03-01', endsOn: '2027-02-28',
    schoolCount: 1, classCount: 2, studentCount: 31, spanCount: 12, sessionCount: 40,
}

/** 만들기만 하고 아무것도 넣지 않은 학년도. 지워도 안전하다는 것을 말해야 하는 쪽이다. */
const YEAR_2027 = {
    id: 3, year: 2027, startsOn: '2027-03-01', endsOn: '2028-02-29',
    schoolCount: 0, classCount: 0, studentCount: 0, spanCount: 0, sessionCount: 0,
}

/** 학급마다 인원을 다르게 둔다. 요약이 실제로 센 값인지 보려면 둘이 달라야 한다. */
const STUDENTS = {
    10: [{id: 1, number: 1, name: '김하늘'}, {id: 2, number: 2, name: '박서연'}],
    21: [{id: 3, number: 4, name: '이도윤'}],
}

/** 학급 목록을 설정해 두고 화면을 그린다. 읽기는 전부 가짜로 대체한다. */
async function render(classes = []) {
    const app = useAppStore()
    app.classes = classes

    const roster = useRosterStore()
    vi.spyOn(roster, 'fetchStudents').mockImplementation(async (classId) => {
        roster.students = STUDENTS[classId] ?? []
    })
    vi.spyOn(useSchoolStore(), 'fetchAll').mockResolvedValue()

    const wrapper = mount(WelcomeView, {global: {plugins: [router]}})
    await flushPromises()
    return wrapper
}

/** 단계 표시의 단추들. 차례가 곧 단계 번호다. */
const steps = (wrapper) => wrapper.findAll('.steps .step')

/** 아래 고정된 [뒤로] · [다음]. 단계가 바뀌어도 자리가 같다. */
const acts = (wrapper) => wrapper.findAll('.wiz__acts button')

/** 그 단계로 간다. 교사가 하는 것과 같은 방법 — 단계 표시를 누른다. */
async function goStep(wrapper, n) {
    await steps(wrapper)[n - 1].trigger('click')
}

/** 모달은 body로 Teleport된다. 그 안의 단추를 글자로 찾는다. */
function modalButton(text) {
    return [...document.querySelectorAll('.modal__foot button')]
        .find((b) => b.textContent.trim() === text)
}

// 모달이 body에 남으면 다음 시험이 앞의 모달을 찾는다.
beforeEach(() => {
    document.body.innerHTML = ''
    setActivePinia(createPinia())

    const app = useAppStore()
    app.booted = true
    app.today = '2026-09-10'
    app.years = [YEAR_2026]
    app.yearId = 2
    app.schools = [{id: 1, name: '한빛고등학교', maxSlot: 7, dueDays: 7, dueSkipOffdays: true}]
    app.classes = []
    app.classId = null

    const school = useSchoolStore()
    school.school = app.schools[0]
})

describe('첫 실행 — 단계 오가기', () => {
    it('다섯 단계를 그리고, 눌러서 앞뒤로 오간다', async () => {
        const wrapper = await render()

        expect(steps(wrapper)).toHaveLength(5)
        expect(wrapper.text()).toContain('설정할 것은 세 가지입니다')

        await goStep(wrapper, 3)
        expect(wrapper.text()).toContain('최대 교시')
        expect(wrapper.text()).not.toContain('설정할 것은 세 가지입니다')

        await goStep(wrapper, 1)
        expect(wrapper.text()).toContain('설정할 것은 세 가지입니다')
    })

    it('지난 단계는 체크로 표시한다', async () => {
        const wrapper = await render()
        await goStep(wrapper, 3)

        const marks = steps(wrapper).map((s) => s.find('.step__no').text())
        expect(marks).toEqual(['✓', '✓', '3', '4', '5'])
        expect(steps(wrapper)[2].classes()).toContain('is-now')
    })

    it('2단계는 오늘 날짜로 채운 학년도를 보여준다 — 교사에게 되묻지 않는다', async () => {
        const wrapper = await render()
        await goStep(wrapper, 2)

        expect(wrapper.text()).toContain('2026학년도')
        // 학년도는 3월에 열린다. 그 경계를 화면이 그대로 말한다.
        expect(wrapper.text()).toContain('2026-03-01')
        expect(wrapper.text()).toContain('2027-02-28')
    })
})

describe('첫 실행 — 학년도와 학교', () => {
    it('학년도를 선택하면 스토어 액션을 거친다 — 상태에 직접 대입하지 않는다', async () => {
        const app = useAppStore()
        app.years = [YEAR_2026, YEAR_2027]
        const select = vi.spyOn(app, 'selectYear').mockResolvedValue()

        const wrapper = await render()
        await goStep(wrapper, 2)

        const pick = wrapper.findAll('button').find((b) => b.text() === '2027학년도')
        await pick.trigger('click')
        await flushPromises()

        // 직접 대입하면 학교 목록도 담당 학급 · 강좌 목록도 지난 학년도의 것으로 남는다.
        expect(select).toHaveBeenCalledWith(3)
        expect(app.yearId).toBe(2)
    })

    it('학년도를 직접 만든다 — 선택만 되면 2월의 교사가 앱을 쓸 수 없다', async () => {
        // 2월에 다음 학년도를 미리 준비하거나 지난해 기록을 옮겨 적는 교사가 있다.
        const app = useAppStore()
        const create = vi.spyOn(app, 'createYear').mockResolvedValue(9)

        const wrapper = await render()
        await goStep(wrapper, 2)

        await wrapper.find('input[placeholder="2027"]').setValue('2027')
        await wrapper.findAll('button').find((b) => b.text() === '학년도 추가').trigger('click')
        await flushPromises()

        // 시작일 · 종료일은 묻지 않는다. 3월 규칙으로 채운다.
        expect(create).toHaveBeenCalledWith(2027)
    })

    it('연도가 아닌 값은 만들지 않고 이유를 적는다', async () => {
        const app = useAppStore()
        const create = vi.spyOn(app, 'createYear').mockResolvedValue(9)

        const wrapper = await render()
        await goStep(wrapper, 2)

        await wrapper.find('input[placeholder="2027"]').setValue('19')
        await wrapper.findAll('button').find((b) => b.text() === '학년도 추가').trigger('click')
        await flushPromises()

        expect(create).not.toHaveBeenCalled()
        expect(wrapper.text()).toContain('1900 이상')
    })

    it('학교를 마감한다 — 한 번 묻고, 지운다고 말하지 않는다', async () => {
        // 되돌리기 어려운 것은 삭제뿐이라 거기에만 한 번 묻는다. 그런데 학교는
        // 지난 기록이 가리키므로 행은 남고 목록에서만 내려간다 — 문구가 그래야 한다.
        const app = useAppStore()
        app.schools = [{id: 1, name: '한빛고등학교'}, {id: 2, name: '푸른중학교'}]
        const schoolStore = useSchoolStore()
        const retire = vi.spyOn(schoolStore, 'retireSchool').mockResolvedValue()

        const wrapper = await render()
        await goStep(wrapper, 3)

        const drops = wrapper.findAll('.drop')
        expect(drops).toHaveLength(2)

        await drops[1].trigger('click')
        await flushPromises()

        // UiModal은 body로 Teleport한다. 화면 나무가 아니라 문서에서 찾는다.
        const modal = document.querySelector('.modal')
        expect(modal.textContent).toContain('마감합니다')
        expect(modal.textContent).toContain('푸른중학교')
        expect(modal.textContent).not.toContain('지웁니다')

        modalButton('마감').click()
        await flushPromises()

        expect(retire).toHaveBeenCalledWith(2)
    })

    it('마감을 취소하면 아무것도 하지 않는다', async () => {
        const app = useAppStore()
        app.schools = [{id: 1, name: '한빛고등학교'}]
        const retire = vi.spyOn(useSchoolStore(), 'retireSchool').mockResolvedValue()

        const wrapper = await render()
        await goStep(wrapper, 3)
        await wrapper.find('.drop').trigger('click')
        await flushPromises()
        modalButton('취소').click()
        await flushPromises()

        expect(retire).not.toHaveBeenCalled()
    })

    it('학교가 하나도 없는 것이 첫 실행의 정상 상태다 — 그 자리에서 만든다', async () => {
        const app = useAppStore()
        app.schools = []
        const school = useSchoolStore()
        school.school = null

        const create = vi.spyOn(school, 'createSchool').mockImplementation(async ({name}) => {
            app.schools = [{id: 1, name, maxSlot: 7, dueDays: 7, dueSkipOffdays: true}]
            return 1
        })
        const select = vi.spyOn(app, 'selectSchool').mockResolvedValue()

        const wrapper = await render()
        await goStep(wrapper, 3)

        expect(wrapper.text()).toContain('아직 없습니다')
        // 고칠 것이 없으면 빈 칸을 나열하지 않는다. 무엇을 고치는지 알 수 없기 때문이다.
        expect(wrapper.findAll('.pick--slot')).toHaveLength(0)
        expect(wrapper.findAll('input[type="number"]')).toHaveLength(0)

        await wrapper.find('input[placeholder="한빛고등학교"]').setValue('한빛고등학교')
        await wrapper.findAll('button').find((b) => b.text() === '학교 추가').trigger('click')
        await flushPromises()

        // 기본 출결 태그와 한도 규정이 함께 들어가는 길은 이 액션 하나뿐이다.
        expect(create).toHaveBeenCalledWith({name: '한빛고등학교'})
        expect(select).toHaveBeenCalledWith(1)
    })

    it('이름 없이 만들지 않는다 — 이름이 학교를 가리키는 값이다', async () => {
        const app = useAppStore()
        app.schools = []
        const school = useSchoolStore()
        school.school = null
        const create = vi.spyOn(school, 'createSchool').mockResolvedValue(1)

        const wrapper = await render()
        await goStep(wrapper, 3)
        await wrapper.findAll('button').find((b) => b.text() === '학교 추가').trigger('click')
        await flushPromises()

        expect(create).not.toHaveBeenCalled()
        expect(wrapper.text()).toContain('학교 이름을 적어주세요.')
    })

    it('학교를 선택하면 그 학교의 설정을 읽는다', async () => {
        const app = useAppStore()
        app.schools = [
            {id: 1, name: '한빛고등학교', maxSlot: 7, dueDays: 7, dueSkipOffdays: true},
            {id: 2, name: '푸른중학교', maxSlot: 6, dueDays: 5, dueSkipOffdays: true},
        ]
        const select = vi.spyOn(app, 'selectSchool').mockResolvedValue()

        const wrapper = await render()
        await goStep(wrapper, 3)
        await wrapper.findAll('button').find((b) => b.text() === '푸른중학교').trigger('click')
        await flushPromises()

        expect(select).toHaveBeenCalledWith(2)
        expect(useSchoolStore().fetchAll).toHaveBeenCalled()
    })
})

/**
 * 학년도 지우기. **학교를 내리는 것과 같은 모양의 휴지통이지만 하는 일이 다르다** —
 * 학교는 행이 남고 목록에서만 내려가지만, 학년도는 행까지 지우고 그 아래가 CASCADE로
 * 함께 사라진다. 문구와 수가 그 차이를 말하지 않으면 교사는 같은 정도의 일로 읽는다.
 *
 * 스토어의 `deleteYear`는 여기서 가로챈다 — 이 화면이 확인하는 것은 "무엇을 보여주고
 * 무엇을 부르는가"까지이고, 커맨드를 제대로 부르는지는 스토어 시험이 본다.
 */
describe('첫 실행 — 학년도 지우기', () => {
    /** 학년도 칩의 휴지통. 학년도 단계에서만 그려진다. */
    const drops = (wrapper) => wrapper.findAll('.drop')

    /**
     * 두 학년도를 두고 학년도 단계를 연다. 하나뿐이면 휴지통이 잠긴다.
     *
     * **가짜를 끼우지 않고 스토어의 액션을 가로챈다** — 이름이 바뀌면 여기서 깨져야 한다.
     */
    async function openYearStep(remove = vi.fn().mockResolvedValue()) {
        const app = useAppStore()
        app.years = [YEAR_2026, YEAR_2027]
        vi.spyOn(app, 'deleteYear').mockImplementation(remove)
        const wrapper = await render()
        await goStep(wrapper, 2)
        return {app, wrapper}
    }

    it('묻기 전에 함께 사라지는 것을 수로 보여준다', async () => {
        const {wrapper} = await openYearStep()
        expect(drops(wrapper)).toHaveLength(2)

        await drops(wrapper)[0].trigger('click')
        await flushPromises()

        // 수는 목록에 이미 실려 온다 — 대화상자를 여는 순간 다시 묻지 않는다.
        const modal = document.querySelector('.modal')
        expect(modal.textContent).toContain('2026학년도')
        expect(modal.textContent).toContain('1개')
        expect(modal.textContent).toContain('31명')
        // 담임 출결과 교과 차시를 한 수로 합치지 않는다. 합치면 어느 쪽인지 알 수 없다.
        expect(modal.textContent).toContain('담임 출결')
        expect(modal.textContent).toContain('12건')
        expect(modal.textContent).toContain('교과 차시')
        expect(modal.textContent).toContain('40개')
    })

    it('학교를 내리는 것과 다른 말을 쓴다 — 학년도는 기록까지 지운다', async () => {
        const {wrapper} = await openYearStep()
        await drops(wrapper)[0].trigger('click')
        await flushPromises()

        const modal = document.querySelector('.modal')
        expect(modal.textContent).toContain('지웁니다')
        expect(modal.textContent).toContain('되돌릴 수 없습니다')
        // 학교 대화상자의 말이다. 돌려 쓰면 같은 정도의 일로 읽힌다.
        expect(modal.textContent).not.toContain('목록에서만 내립니다')
    })

    it('학교와 담당이 0개면 그 사실을 그대로 말한다 — 안전한 것을 알아야 망설이지 않는다', async () => {
        const {wrapper} = await openYearStep()
        await drops(wrapper)[1].trigger('click')
        await flushPromises()

        const modal = document.querySelector('.modal')
        expect(modal.textContent).toContain('2027학년도')
        expect(modal.textContent).toContain('0개')
        expect(modal.textContent).toContain('아직 아무것도 추가하지 않은 학년도입니다')
    })

    it('확인을 누르면 스토어 액션을 거친다', async () => {
        const remove = vi.fn().mockResolvedValue()
        const {wrapper} = await openYearStep(remove)
        await drops(wrapper)[1].trigger('click')
        await flushPromises()

        modalButton('지우기').click()
        await flushPromises()

        expect(remove).toHaveBeenCalledWith(3)
        // 지운 뒤에는 학교 설정을 다시 읽는다. 지금 보던 학년도였으면 옮겨 가 있다.
        expect(useSchoolStore().fetchAll).toHaveBeenCalled()
    })

    it('취소하면 아무것도 하지 않는다', async () => {
        const remove = vi.fn().mockResolvedValue()
        const {wrapper} = await openYearStep(remove)
        await drops(wrapper)[1].trigger('click')
        await flushPromises()

        modalButton('취소').click()
        await flushPromises()

        expect(remove).not.toHaveBeenCalled()
    })

    it('마지막 학년도는 휴지통을 잠근다 — 숨기지 않는다', async () => {
        // 학년도가 없으면 학교도 담당 학급 · 강좌도 만들 수 없다. 커맨드도 거절하지만,
        // 언제나 거절로 끝나는 단추를 그려 두면 교사는 그것을 고장으로 읽는다.
        const app = useAppStore()
        vi.spyOn(app, 'deleteYear').mockResolvedValue()
        const wrapper = await render()
        await goStep(wrapper, 2)

        const drop = wrapper.find('.drop')
        expect(drop.exists()).toBe(true)
        expect(drop.attributes('disabled')).toBeDefined()

        await drop.trigger('click')
        await flushPromises()
        expect(document.querySelector('.modal')).toBeNull()

        // 이유는 화면에 적혀 있다. 잠긴 단추만으로는 왜인지 알 수 없다.
        expect(wrapper.text()).toContain('마지막 하나는 지울 수 없습니다')
    })

    it('지우지 못하면 이유를 화면에 적는다 — 조용히 넘기지 않는다', async () => {
        const remove = vi.fn().mockRejectedValue(new Error('마지막 학년도는 지울 수 없습니다.'))
        const {wrapper} = await openYearStep(remove)
        await drops(wrapper)[0].trigger('click')
        await flushPromises()

        modalButton('지우기').click()
        await flushPromises()

        expect(wrapper.text()).toContain('마지막 학년도는 지울 수 없습니다.')
    })
})

describe('첫 실행 — 담당 학급 · 강좌', () => {
    it('하나도 추가하지 않으면 완료로 갈 수 없다 — 잠글 뿐 숨기지 않는다', async () => {
        const wrapper = await render()
        await goStep(wrapper, 4)

        const [back, next] = acts(wrapper)
        expect(next.text()).toBe('다음')
        expect(next.attributes('disabled')).toBeDefined()
        expect(back.attributes('disabled')).toBeUndefined()
        // 5단계 표시도 함께 잠긴다. 사라지지는 않는다.
        expect(steps(wrapper)[4].attributes('disabled')).toBeDefined()

        useAppStore().classes = [HOMEROOM]
        await flushPromises()

        expect(acts(wrapper)[1].attributes('disabled')).toBeUndefined()
        await acts(wrapper)[1].trigger('click')
        expect(wrapper.text()).toContain('추가한 것')
    })

    it('교과 강좌는 학년 · 반을 묻지 않는다 — 반이 섞이기 때문이다', async () => {
        const wrapper = await render()
        await goStep(wrapper, 4)

        const homeroom = wrapper.find('.mine--homeroom')
        const subject = wrapper.find('.mine--subject')

        expect(homeroom.findAll('input[type="number"]')).toHaveLength(2)
        expect(subject.findAll('input[type="number"]')).toHaveLength(0)
        expect(subject.text()).toContain('반이 섞입니다')
    })

    it('담임은 학년 · 반으로 이름을 짓고, 교과는 이름만 넘긴다', async () => {
        const wrapper = await render()
        const app = useAppStore()
        const create = vi.spyOn(app, 'createClass').mockImplementation(async ({role, select}) => {
            const cls = role === 'homeroom' ? HOMEROOM : SUBJECT
            app.classes = [...app.classes, cls]
            // 진짜 `createClass`는 선택할 때 `classId`를 함께 옮긴다. 가짜가 그것을
            // 하지 않으면 "이미 선택한 것이 있는가"를 보는 코드가 시험에서만 다르게 돈다.
            if (select) app.classId = cls.id
            return cls.id
        })
        vi.spyOn(app, 'selectClass').mockResolvedValue()
        await goStep(wrapper, 4)

        const homeroom = wrapper.find('.mine--homeroom')
        await homeroom.findAll('input[type="number"]')[0].setValue('3')
        await homeroom.findAll('input[type="number"]')[1].setValue('6')
        await homeroom.findAll('button').find((b) => b.text() === '추가').trigger('click')
        await flushPromises()

        expect(create).toHaveBeenCalledWith({
            role: 'homeroom', name: '3학년 6반', grade: 3, classNo: 6, select: true,
        })

        const subject = wrapper.find('.mine--subject')
        await subject.find('input[type="text"]').setValue('인공지능기초A')
        await subject.findAll('button').find((b) => b.text() === '추가').trigger('click')
        await flushPromises()

        // **교과를 더했다고 모드가 넘어가지 않는다.** `selectClass`가 모드를 저장하므로,
        // 교과를 마지막으로 더한 교사는 다음 실행이 교과 모드로 열린다.
        // 담임을 이미 선택해 둔 뒤이므로 여기서는 선택하지 않는다.
        expect(create).toHaveBeenLastCalledWith({
            role: 'subject', name: '인공지능기초A', grade: null, classNo: null, select: false,
        })
    })

    it('명렬표를 열면 그 학급을 함께 선택한다 — 명단이 붙는 곳이 어긋나면 안 된다', async () => {
        const wrapper = await render([HOMEROOM, SUBJECT])
        const select = vi.spyOn(useAppStore(), 'selectClass').mockResolvedValue()
        await goStep(wrapper, 4)

        const open = wrapper.find('.mine--homeroom').findAll('button')
            .find((b) => b.text() === '명렬표 열기')
        await open.trigger('click')
        await flushPromises()

        expect(select).toHaveBeenCalledWith(HOMEROOM.id)
        expect(wrapper.find('.mine--homeroom .mine__panel').exists()).toBe(true)
        expect(wrapper.find('.mine--subject .mine__panel').exists()).toBe(false)
    })

    it('교과 강좌도 명렬표를 연다 — 담임과 같은 화면이다', async () => {
        const wrapper = await render([HOMEROOM, SUBJECT])
        vi.spyOn(useAppStore(), 'selectClass').mockResolvedValue()
        await goStep(wrapper, 4)

        const open = wrapper.find('.mine--subject').findAll('button')
            .find((b) => b.text().includes('명렬표'))
        expect(open.attributes('disabled')).toBeUndefined()

        await open.trigger('click')
        await flushPromises()

        expect(wrapper.find('.mine--subject .mine__panel').exists()).toBe(true)
    })

    it('교과 명렬표를 열어도 그 강좌를 선택하지는 않는다 — 모드가 저장되면 안 된다', async () => {
        // selectClass는 app_config에 mode를 저장한다. 온보딩 끝에 교과 명렬표를 마지막으로
        // 만진 교사는 다음 실행이 교과 모드로 열리는데, 이 화면은 meta.bare라 그 전환이
        // 눈에 보이지도 않는다. 명단이 붙는 곳은 넘기는 classId가 이미 결정한다.
        const wrapper = await render([HOMEROOM, SUBJECT])
        const select = vi.spyOn(useAppStore(), 'selectClass').mockResolvedValue()
        await goStep(wrapper, 4)

        const open = wrapper.find('.mine--subject').findAll('button')
            .find((b) => b.text().includes('명렬표'))
        await open.trigger('click')
        await flushPromises()

        expect(select).not.toHaveBeenCalled()
    })
})

describe('첫 실행 — 완료', () => {
    it('요약은 실제로 등록한 것을 말한다', async () => {
        const wrapper = await render([HOMEROOM, SUBJECT])
        await goStep(wrapper, 5)

        const text = wrapper.text()
        expect(text).toContain('2026학년도')
        expect(text).toContain('한빛고등학교')
        expect(text).toContain('3학년 6반')
        expect(text).toContain('인공지능기초A')
        // 학급마다 따로 센 인원이 그대로 온다. 합계는 명단 단위라 겹치는 학생을 두 번 센다.
        expect(wrapper.find('.mine--homeroom').exists()).toBe(false)
        expect(text).toContain('3명')
    })

    it('한쪽만 등록했으면 반대쪽은 없다고 적는다 — 빈 자리로 두지 않는다', async () => {
        const wrapper = await render([HOMEROOM])
        await goStep(wrapper, 5)

        expect(wrapper.text()).toContain('추가한 교과 강좌가 없습니다')
        expect(wrapper.text()).toContain('3학년 6반')
    })

    it('[개요로 가기]가 개요로 보낸다', async () => {
        const wrapper = await render([HOMEROOM])
        await goStep(wrapper, 5)

        const next = acts(wrapper)[1]
        expect(next.text()).toBe('개요로 가기')
        await next.trigger('click')
        await flushPromises()

        expect(router.currentRoute.value.path).toBe('/')
    })
})
