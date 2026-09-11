/**
 * 첫 실행 화면 — 다섯 단계.
 *
 * 여기서 고정하는 것은 다섯이다.
 *   1. 단계를 눌러 앞뒤로 오간다. 되돌아갈 길이 없으면 잘못 적은 교사가 앱을 껐다 켠다.
 *   2. 맡은 것을 하나도 등록하지 않으면 완료로 갈 수 없다. **단추는 잠글 뿐 숨기지 않는다.**
 *   3. 교과 강좌에는 학년 · 반 칸이 없다. 선택과목은 반이 섞여 가리킬 반이 없다.
 *   4. 담임과 교과가 **같은 명렬표 화면**을 쓴다. 다만 교과는 그 강좌를 고르지 않는다 —
 *      고르면 모드가 저장되어 다음 실행이 교과 모드로 열린다.
 *   5. 완료 단계의 요약은 **실제로 등록한 것**을 말한다. 고정 문구를 그리면 아무도 모른다.
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

/** 학급마다 인원을 다르게 둔다. 요약이 실제로 센 값인지 보려면 둘이 달라야 한다. */
const STUDENTS = {
    10: [{id: 1, number: 1, name: '김하늘'}, {id: 2, number: 2, name: '박서연'}],
    21: [{id: 3, number: 4, name: '이도윤'}],
}

/** 학급 목록을 정해 두고 화면을 그린다. 읽기는 전부 멈춰 세운다. */
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
    app.years = [{id: 2, year: 2026}]
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
        expect(wrapper.text()).toContain('정할 것은 세 가지입니다')

        await goStep(wrapper, 3)
        expect(wrapper.text()).toContain('최대 교시')
        expect(wrapper.text()).not.toContain('정할 것은 세 가지입니다')

        await goStep(wrapper, 1)
        expect(wrapper.text()).toContain('정할 것은 세 가지입니다')
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
    it('학년도를 고르면 스토어 액션을 거친다 — 상태에 직접 대입하지 않는다', async () => {
        const app = useAppStore()
        app.years = [{id: 2, year: 2026}, {id: 3, year: 2027}]
        const select = vi.spyOn(app, 'selectYear').mockResolvedValue()

        const wrapper = await render()
        await goStep(wrapper, 2)

        const pick = wrapper.findAll('button').find((b) => b.text() === '2027학년도')
        await pick.trigger('click')
        await flushPromises()

        // 직접 대입하면 학교 목록도 맡은 것 목록도 지난 학년도의 것으로 남는다.
        expect(select).toHaveBeenCalledWith(3)
        expect(app.yearId).toBe(2)
    })

    it('학년도를 직접 만든다 — 고르기만 되면 2월의 교사가 앱을 쓸 수 없다', async () => {
        // 2월에 다음 학년도를 미리 준비하거나 지난해 기록을 옮겨 적는 교사가 있다.
        const app = useAppStore()
        const create = vi.spyOn(app, 'createYear').mockResolvedValue(9)

        const wrapper = await render()
        await goStep(wrapper, 2)

        await wrapper.find('input[placeholder="2027"]').setValue('2027')
        await wrapper.findAll('button').find((b) => b.text() === '학년도 만들기').trigger('click')
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
        await wrapper.findAll('button').find((b) => b.text() === '학년도 만들기').trigger('click')
        await flushPromises()

        expect(create).not.toHaveBeenCalled()
        expect(wrapper.text()).toContain('1900 이상')
    })

    it('학교를 목록에서 내린다 — 한 번 묻고, 지운다고 말하지 않는다', async () => {
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
        expect(modal.textContent).toContain('목록에서 내립니다')
        expect(modal.textContent).toContain('푸른중학교')
        expect(modal.textContent).not.toContain('지웁니다')

        modalButton('내리기').click()
        await flushPromises()

        expect(retire).toHaveBeenCalledWith(2)
    })

    it('내리기를 취소하면 아무것도 하지 않는다', async () => {
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
        // 고칠 것이 없으면 빈 칸을 늘어놓지 않는다. 무엇을 고치는지 알 수 없기 때문이다.
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

    it('학교를 고르면 그 학교의 설정을 읽는다', async () => {
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

describe('첫 실행 — 맡은 것', () => {
    it('하나도 등록하지 않으면 완료로 갈 수 없다 — 잠글 뿐 숨기지 않는다', async () => {
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
        expect(wrapper.text()).toContain('등록한 것')
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
            // 진짜 `createClass`는 고를 때 `classId`를 함께 옮긴다. 가짜가 그것을
            // 하지 않으면 "이미 고른 것이 있는가"를 보는 코드가 시험에서만 다르게 돈다.
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
        // 담임을 이미 골라 둔 뒤이므로 여기서는 고르지 않는다.
        expect(create).toHaveBeenLastCalledWith({
            role: 'subject', name: '인공지능기초A', grade: null, classNo: null, select: false,
        })
    })

    it('명렬표를 펼치면 그 학급을 함께 고른다 — 명단이 붙는 곳이 어긋나면 안 된다', async () => {
        const wrapper = await render([HOMEROOM, SUBJECT])
        const select = vi.spyOn(useAppStore(), 'selectClass').mockResolvedValue()
        await goStep(wrapper, 4)

        const open = wrapper.find('.mine--homeroom').findAll('button')
            .find((b) => b.text() === '명렬표 넣기')
        await open.trigger('click')
        await flushPromises()

        expect(select).toHaveBeenCalledWith(HOMEROOM.id)
        expect(wrapper.find('.mine--homeroom .mine__panel').exists()).toBe(true)
        expect(wrapper.find('.mine--subject .mine__panel').exists()).toBe(false)
    })

    it('교과 강좌도 명렬표를 펼친다 — 담임과 같은 화면이다', async () => {
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

    it('교과 명렬표를 펼쳐도 그 강좌를 고르지는 않는다 — 모드가 저장되면 안 된다', async () => {
        // selectClass는 app_config에 mode를 저장한다. 온보딩 끝에 교과 명렬표를 마지막으로
        // 만진 교사는 다음 실행이 교과 모드로 열리는데, 이 화면은 meta.bare라 그 전환이
        // 눈에 보이지도 않는다. 명단이 붙는 곳은 넘기는 classId가 이미 정한다.
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

    it('맡은 것이 한쪽뿐이면 반대쪽은 없다고 적는다 — 빈 자리로 두지 않는다', async () => {
        const wrapper = await render([HOMEROOM])
        await goStep(wrapper, 5)

        expect(wrapper.text()).toContain('맡은 교과 강좌가 없습니다')
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
