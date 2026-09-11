/**
 * 이동 화면 — 학년도 → 학교 → 담당 학급 · 강좌.
 *
 * 여기서 고정하는 것은 넷이다.
 *   1. **계층 그대로 보여준다.** 학교가 결정되어야 담당 학급 · 강좌가 결정된다 —
 *      A학교에서는 담임 + 교과, B학교에서는 교과만. 그것이 순회 교사다.
 *   2. **학교를 옮기면 목록이 그 학교의 것으로 바뀐다.** 학급과 강좌는 학교에 소속되어 있다.
 *   3. **학년도는 여기서 바꾸지 않는다.** 해가 바뀌면 학교부터 다시 골라야 하므로
 *      설정이 담당한다. 어디로 가야 하는지는 화면이 적는다.
 *   4. **강좌는 묶음별로 모인다.** 담임 학급은 묶지 않는다 — 묶을 것이 없다.
 *
 * 모드가 분리되는 것(담임에 교과가 새지 않는다)은 `views.test.js`가 이미 본다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {flushPromises, mount} from '@vue/test-utils'
import {createPinia, setActivePinia} from 'pinia'
import {createRouter, createWebHashHistory} from 'vue-router'
import {useAppStore} from '../stores/app'
import MoveView from './MoveView.vue'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn().mockResolvedValue([])}))

const router = createRouter({
    history: createWebHashHistory(),
    routes: [{path: '/:rest(.*)', component: {template: '<div/>'}}],
})

const SCHOOL = {id: 1, yearId: 2, name: '한빛고등학교', maxSlot: 7, dueDays: 7}
const OTHER_SCHOOL = {id: 2, yearId: 2, name: '푸른중학교', maxSlot: 6, dueDays: 5}

const HOMEROOM = {
    id: 10, schoolId: 1, role: 'homeroom', name: '3학년 6반',
    grade: 3, classNo: 6, groupTagId: null, groupTagName: null, validTo: null,
}
const PROG_A = {
    id: 21, schoolId: 1, role: 'subject', name: '프로그래밍A',
    grade: null, classNo: null, groupTagId: 5, groupTagName: '프로그래밍', validTo: null,
}
const PROG_B = {...PROG_A, id: 22, name: '프로그래밍B'}
const SOLO = {
    id: 23, schoolId: 1, role: 'subject', name: '통합사회',
    grade: null, classNo: null, groupTagId: null, groupTagName: null, validTo: null,
}

const render = () => mount(MoveView, {global: {plugins: [router]}})

/** 이름표로 줄을 찾는다. */
function row(wrapper, label) {
    return wrapper.findAll('.set__row').find((r) => r.find('.set__label').text() === label)
}

function button(scope, text) {
    return scope.findAll('button').find((b) => b.text() === text)
}

beforeEach(() => {
    setActivePinia(createPinia())

    const app = useAppStore()
    app.booted = true
    app.today = '2026-09-11'
    app.years = [{id: 2, year: 2026}]
    app.yearId = 2
    app.schools = [SCHOOL]
    app.pickedSchoolId = 1
    app.classes = [HOMEROOM, PROG_A, PROG_B, SOLO]
    app.classId = 10
    app.lastClassId = {homeroom: 10, subject: 21}
})

describe('이동 — 어디에서', () => {
    it('학년도 · 학교 · 담당 학급 · 강좌를 그 차례로 보여준다', () => {
        const wrapper = render()
        const heads = wrapper.findAll('.ledger__head h4').map((h) => h.text())
        expect(heads).toEqual(['어디에서', '담당 학급 · 강좌'])

        const labels = wrapper.findAll('.set__label').map((l) => l.text())
        expect(labels).toEqual(['학년도', '학교'])
        expect(row(wrapper, '학년도').text()).toContain('2026')
    })

    it('학년도는 여기서 바꾸지 않는다 — 어디로 가야 하는지 적는다', () => {
        const wrapper = render()
        const year = row(wrapper, '학년도')

        expect(year.findAll('button')).toHaveLength(0)
        expect(year.text()).toContain('설정에서 바꿉니다')
    })

    it('학교를 누르면 그 학교로 옮긴다 — 목록이 통째로 바뀐다', async () => {
        const app = useAppStore()
        app.schools = [SCHOOL, OTHER_SCHOOL]
        const select = vi.spyOn(app, 'selectSchool').mockResolvedValue()

        const wrapper = render()
        const picker = row(wrapper, '학교')
        expect(button(picker, '한빛고등학교').classes()).toContain('is-on')

        await button(picker, '푸른중학교').trigger('click')
        await flushPromises()
        expect(select).toHaveBeenCalledWith(2)

        // 이미 보고 있는 학교는 누를 것이 없다.
        await button(picker, '한빛고등학교').trigger('click')
        expect(select).toHaveBeenCalledTimes(1)
    })

    it('학교가 하나도 없으면 어디서 만드는지 알린다', () => {
        const app = useAppStore()
        app.schools = []
        app.classes = []

        expect(render().text()).toContain('설정에서 만들어 주세요')
    })
})

describe('이동 — 담당 학급 · 강좌', () => {
    /** 지금 그려진 줄의 이름. 차례가 곧 목록의 차례다. */
    const names = (wrapper) => wrapper.findAll('.move__name').map((n) => n.text())

    it('교과 모드에서는 묶음별로 모이고, 묶음 없는 것이 맨 뒤에 온다', () => {
        const app = useAppStore()
        app.classId = 21

        const wrapper = render()
        expect(wrapper.findAll('.movegrp b').map((b) => b.text()))
            .toEqual(['프로그래밍', '묶음 없음'])
        // 어느 줄도 사라지지 않는다. 묶음이 없는 강좌도 목록에 남는다.
        expect(names(wrapper)).toEqual(['프로그래밍A', '프로그래밍B', '통합사회'])
    })

    it('담임 학급은 묶지 않는다 — 묶을 것이 없고 머리글만 는다', () => {
        const wrapper = render()

        expect(wrapper.find('.movegrp').exists()).toBe(false)
        expect(names(wrapper)).toEqual(['3학년 6반'])
    })

    it('묶음이 하나뿐이면 머리글을 그리지 않는다', () => {
        const app = useAppStore()
        app.classes = [HOMEROOM, PROG_A, PROG_B]
        app.classId = 21

        const wrapper = render()
        expect(wrapper.find('.movegrp').exists()).toBe(false)
        expect(names(wrapper)).toEqual(['프로그래밍A', '프로그래밍B'])
    })

    it('담당 학급 · 강좌가 하나도 없으면 하나뿐이라고 적지 않는다', () => {
        const app = useAppStore()
        app.classes = []
        app.classId = null

        // 0개일 때 "하나뿐입니다"라고 적으면 바로 아래의 빈 상태 문구와 어긋난다.
        const text = render().text()
        expect(text).toContain('이 모드에서 담당하는 학급이나 강좌가 없습니다')
        expect(text).not.toContain('하나뿐입니다')
        expect(text).toContain('설정에서 만들 수 있습니다')
    })

    it('담당 학급 · 강좌가 하나뿐이면 어디서 더하는지 적는다', () => {
        // 담임 학급 하나뿐인 상태. 이 화면에서 할 일이 없으므로 가는 길을 적는다.
        expect(render().text()).toContain('담당 학급 · 강좌가 하나뿐입니다')
    })

    /**
     * 통칭을 새로 만들지 않는다. `맡은 것`은 담임 학급과 교과 강좌를 한 낱말로 묶으려다
     * 나온 말인데 교사는 그 둘을 묶어 부르지 않는다 — 목록이 비었든 하나뿐이든 여럿이든
     * 이 화면 어디에도 그 말이 남으면 안 된다.
     */
    it('어느 상태에서도 `맡은 것`이라고 적지 않는다', () => {
        const app = useAppStore()
        expect(render().text()).not.toContain('맡은 것')

        app.classes = []
        app.classId = null
        expect(render().text()).not.toContain('맡은 것')

        app.classes = [HOMEROOM, PROG_A, PROG_B, SOLO]
        app.classId = 21
        expect(render().text()).not.toContain('맡은 것')
    })

    it('둘 이상이면 아무 말도 하지 않는다', () => {
        const app = useAppStore()
        app.classId = 21

        const text = render().text()
        expect(text).not.toContain('하나뿐입니다')
        expect(text).not.toContain('설정에서 만들 수 있습니다')
    })

    it('줄을 누르면 그 강좌로 옮긴다', async () => {
        const app = useAppStore()
        app.classId = 21
        const select = vi.spyOn(app, 'selectClass').mockResolvedValue()

        const wrapper = render()
        const rows = wrapper.findAll('.move')
        expect(rows[0].classes()).toContain('is-on')

        await rows[2].trigger('click')
        expect(select).toHaveBeenCalledWith(23)
    })
})
