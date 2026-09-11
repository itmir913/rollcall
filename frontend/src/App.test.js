/**
 * 앱 껍데기 — 사이드바가 지금 어디에 있는지 말한다.
 *
 * 이 파일이 지키는 것은 셋이다.
 *   · **스위치는 늘 보인다.** 한쪽만 맡은 교사에게 지우면 담임 학급을 등록하러 갈
 *     길 자체가 사라지고, 이 앱이 자기 것의 절반만이라고 말하는 것이 된다.
 *   · **담당 학급 · 강좌가 없는 모드에서 항목을 지우지 않고 잠근다.** 사라지면 무엇이
 *     없어졌는지 보이지 않는다. 잠긴 줄은 눌리지 않고 이유가 바로 아래 적힌다.
 *   · **맨 아래에 지금 학교가 있다.** 학년도 → 학교가 결정되어야 역할이 결정된다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {flushPromises, mount} from '@vue/test-utils'
import {nextTick} from 'vue'
import {createPinia, setActivePinia} from 'pinia'
import {createRouter, createWebHashHistory} from 'vue-router'
import App from './App.vue'
import {useAppStore} from './stores/app'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn().mockResolvedValue([])}))
vi.stubGlobal('matchMedia', () => ({matches: false, addEventListener: () => {}}))

const router = createRouter({
    history: createWebHashHistory(),
    routes: [{path: '/:rest(.*)', component: {template: '<div/>'}}],
})

const SCHOOL = {id: 1, yearId: 2, name: '한빛고등학교', maxSlot: 7, dueDays: 7}
const OTHER = {...SCHOOL, id: 2, name: '푸른중학교'}
const HOMEROOM = {
    id: 9, schoolId: 1, role: 'homeroom', name: '3학년 6반', grade: 3, classNo: 6, validTo: null,
}
const SUBJECT = {
    id: 21, schoolId: 1, role: 'subject', name: '인공지능기초 A반',
    grade: null, classNo: null, validTo: null,
}

/**
 * 범위를 세운다. 부팅은 막는다 — App.vue가 onMounted에서 부르는 `boot()`는
 * 가짜 invoke의 빈 응답으로 여기서 넣어 둔 것을 통째로 덮는다.
 */
function scope({schools = [SCHOOL], classes = [HOMEROOM], classId = 9, mode = 'homeroom'} = {}) {
    const app = useAppStore()
    vi.spyOn(app, 'boot').mockResolvedValue()
    app.booted = true
    app.today = '2026-09-11'
    app.years = [{id: 2, year: 2026}]
    app.yearId = 2
    app.schools = schools
    app.pickedSchoolId = schools[0]?.id ?? null
    app.classes = classes
    app.classId = classId
    app.lastMode = mode
    app.lastClassId = {
        homeroom: classes.find((c) => c.role === 'homeroom')?.id ?? null,
        subject: classes.find((c) => c.role === 'subject')?.id ?? null,
    }
    return app
}

function renderApp() {
    return mount(App, {global: {plugins: [router]}})
}

/** 사이드바에 실제로 보이는 항목. 잠긴 줄도 자리를 지키므로 함께 나온다. */
const railLabels = (w) => w.findAll('.rail__link').map((link) => link.text())
const lockedLabels = (w) => w.findAll('.rail__link.is-off').map((link) => link.text())

beforeEach(() => {
    setActivePinia(createPinia())
})

describe('모드 스위치', () => {
    it('한쪽만 맡아도 그린다 — 비담임 교사도 담임 모드로 들어갈 수 있어야 한다', async () => {
        scope({classes: [SUBJECT], classId: 21, mode: 'subject'})
        const wrapper = renderApp()
        await flushPromises()

        const modes = wrapper.findAll('.rail__mode')
        expect(modes.map((b) => b.text())).toEqual(['담임', '교과'])
        expect(modes[1].classes()).toContain('is-on')
    })

    it('누르면 그 모드로 옮긴다', async () => {
        const app = scope({classes: [HOMEROOM, SUBJECT]})
        const setMode = vi.spyOn(app, 'setMode').mockImplementation(async (mode) => {
            app.lastMode = mode
            app.classId = app.lastClassId[mode]
        })

        const wrapper = renderApp()
        await flushPromises()
        await wrapper.findAll('.rail__mode')[1].trigger('click')
        await flushPromises()

        expect(setMode).toHaveBeenCalledWith('subject')
        expect(railLabels(wrapper)).toEqual([
            '개요', '이동', '오늘 수업', '수업 기록', '설정', '업데이트 확인',
        ])
    })

    it('모드마다 목록이 통째로 분리된다 — 한 줄도 새지 않는다', async () => {
        const app = scope({classes: [HOMEROOM, SUBJECT]})
        const wrapper = renderApp()
        await flushPromises()

        expect(railLabels(wrapper)).toEqual([
            '개요', '이동', '오늘의 출결', '출결 기록', '서류 미제출자', 'NEIS 미등재',
            '통계', 'NEIS 검증', '설정', '업데이트 확인',
        ])

        app.classId = 21
        await nextTick()

        for (const homeroomOnly of ['오늘의 출결', '출결 기록', '서류 미제출자', 'NEIS 미등재', '통계', 'NEIS 검증']) {
            expect(wrapper.text()).not.toContain(homeroomOnly)
        }
    })
})

describe('담당 학급 · 강좌가 없는 모드', () => {
    it('항목을 지우지 않고 잠근다 — 이유가 그 자리에 적힌다', async () => {
        scope({classes: [SUBJECT], classId: null, mode: 'homeroom'})
        const wrapper = renderApp()
        await flushPromises()

        // 담임 화면은 자리를 지키되 눌리지 않는다.
        expect(lockedLabels(wrapper)).toEqual([
            '오늘의 출결', '출결 기록', '서류 미제출자', 'NEIS 미등재', '통계', 'NEIS 검증',
        ])
        // 개요 · 이동 · 설정은 열려 있다. 등록하러 갈 길이 막히면 안 된다.
        expect(wrapper.findAll('a.rail__link').map((l) => l.text()))
            .toEqual(['개요', '이동', '설정', '업데이트 확인'])
        expect(wrapper.find('.rail__where').text()).toContain('담임 학급이 없습니다')
        expect(wrapper.find('.rail__here-name').text()).toBe('담임 학급 없음')
    })

    it('수업을 등록하지 않은 교과 모드도 같다', async () => {
        scope({classes: [HOMEROOM], classId: null, mode: 'subject'})
        const wrapper = renderApp()
        await flushPromises()

        expect(lockedLabels(wrapper)).toEqual(['오늘 수업', '수업 기록'])
        expect(wrapper.find('.rail__where').text()).toContain('수업을 등록하지 않았습니다')
    })

    it('담당 학급 · 강좌가 있으면 잠그지 않는다', async () => {
        scope()
        const wrapper = renderApp()
        await flushPromises()

        expect(lockedLabels(wrapper)).toEqual([])
        expect(wrapper.find('.rail__where').exists()).toBe(false)
    })
})

describe('지금 학교', () => {
    it('맨 아래에 학년도와 학교를 적는다', async () => {
        scope()
        const wrapper = renderApp()
        await flushPromises()

        const here = wrapper.find('.rail__school')
        expect(here.text()).toContain('2026학년도')
        expect(here.text()).toContain('한빛고등학교')
        expect(here.attributes('href')).toContain('/move')
    })

    it('학교가 둘 이상이면 옮겨 갈 수 있다고 말한다 — 그것이 순회 교사다', async () => {
        const app = scope()
        const wrapper = renderApp()
        await flushPromises()

        // 학교가 하나뿐인 교사에게는 그 줄이 없다. 옮길 곳이 없기 때문이다.
        expect(wrapper.find('.rail__school-more').exists()).toBe(false)

        app.schools = [SCHOOL, OTHER]
        await nextTick()

        expect(wrapper.find('.rail__school-more').text()).toBe('학교 바꾸기')
    })

    it('학교가 없으면 없다고 적는다 — 빈 자리로 두지 않는다', async () => {
        scope({schools: [], classes: [], classId: null})
        const wrapper = renderApp()
        await flushPromises()

        expect(wrapper.find('.rail__school').text()).toContain('학교 없음')
    })
})
