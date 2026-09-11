/**
 * 설정 화면 — 학년도 · 학교 · 강좌 묶음.
 *
 * 여기서 고정하는 것은 다섯이다.
 *   1. **학년도를 바꾸는 길이 실행 중에 있다.** 첫 실행 화면은 한 번 지나가면 닿을 수
 *      없어, 거기에만 두면 2027년 3월이 와도 새 학년도로 넘어갈 방법이 없다.
 *   2. **새 학년도에 학교가 없는 것은 정상이다.** 그 사실을 화면이 말하고, 만드는 길이
 *      그 자리에 있다. 학교는 `createSchool`을 거쳐야 기본 태그 · 한도 규정이 함께 들어간다.
 *   3. **학교를 선택하면 아래가 전부 그 학교의 것이다.** 순회 교사는 학교마다 다른 것을 담당한다.
 *   4. **강좌는 묶음별로 모인다.** 묶음이 없는 강좌도 사라지지 않고 맨 뒤에 모인다.
 *   5. **교과 명렬표는 담임과 같은 자리다.** 잠긴 단추를 남겨 두면 두 화면이 같은 것을
 *      다르게 말한다.
 *
 * 데이터는 스토어에 직접 넣는다. invoke는 가짜다 — 이 화면이 커맨드를 제대로 부르는지는
 * 스토어 테스트가 본다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {flushPromises, mount} from '@vue/test-utils'
import {createPinia, setActivePinia} from 'pinia'
import {createRouter, createWebHashHistory} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {useSchoolStore} from '../stores/school'
import SettingsView from './SettingsView.vue'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn().mockResolvedValue([])}))
vi.mock('@tauri-apps/plugin-dialog', () => ({save: vi.fn(), open: vi.fn()}))
// jsdom에는 matchMedia가 없다. 테마 composable이 "시스템 따름"을 확인하려고 부른다.
vi.stubGlobal('matchMedia', () => ({matches: false, addEventListener: () => {}}))

const router = createRouter({
    history: createWebHashHistory(),
    routes: [{path: '/:rest(.*)', component: {template: '<div/>'}}],
})

const SCHOOL = {id: 1, yearId: 2, name: '한빛고등학교', maxSlot: 7, dueDays: 7, dueSkipOffdays: true}
const OTHER_SCHOOL = {id: 2, yearId: 2, name: '푸른중학교', maxSlot: 6, dueDays: 5, dueSkipOffdays: true}

const HOMEROOM = {
    id: 10, schoolId: 1, role: 'homeroom', name: '3학년 6반',
    grade: 3, classNo: 6, groupTagId: null, groupTagName: null, validTo: null,
}
/** 같은 묶음의 두 강좌. 분반이 아니라 저마다 독립된 행이고 태그만 같다. */
const PROG_A = {
    id: 21, schoolId: 1, role: 'subject', name: '프로그래밍A',
    grade: null, classNo: null, groupTagId: 5, groupTagName: '프로그래밍', validTo: null,
}
const PROG_B = {...PROG_A, id: 22, name: '프로그래밍B'}
/** 묶을 것이 없는 강좌. 묶음 없는 과목이 더 많다. */
const SOLO = {
    id: 23, schoolId: 1, role: 'subject', name: '통합사회',
    grade: null, classNo: null, groupTagId: null, groupTagName: null, validTo: null,
}

/** 읽기를 고정한다. 화면을 그린 직후 스토어가 빈 응답으로 덮이면 볼 것이 없다. */
function freezeFetches() {
    vi.spyOn(useSchoolStore(), 'fetchAll').mockResolvedValue()
    vi.spyOn(useAxisStore(), 'fetchAll').mockResolvedValue()
}

async function render() {
    freezeFetches()
    const wrapper = mount(SettingsView, {global: {plugins: [router]}})
    await flushPromises()
    return wrapper
}

/** 이름표로 줄을 찾는다. 설정은 줄마다 왼쪽에 무엇을 고치는지 적혀 있다. */
function row(wrapper, label) {
    return wrapper.findAll('.set__row').find((r) => r.find('.set__label').text() === label)
}

/** 그 안의 단추를 글자로 찾는다. 교사가 하는 것과 같은 방법이다. */
function button(scope, text) {
    return scope.findAll('button').find((b) => b.text() === text)
}

/** 열린 대화상자의 단추. UiModal은 본문 밖으로 옮겨 그린다. */
function modalButton(text) {
    return [...document.querySelectorAll('.modal__foot button')]
        .find((b) => b.textContent.trim() === text)
}

beforeEach(() => {
    setActivePinia(createPinia())
    document.body.innerHTML = ''

    const app = useAppStore()
    app.booted = true
    app.today = '2026-09-11'
    app.years = [{id: 2, year: 2026}, {id: 3, year: 2027}]
    app.yearId = 2
    app.schools = [SCHOOL]
    app.pickedSchoolId = 1
    app.classes = [HOMEROOM, PROG_A, PROG_B, SOLO]
    app.classId = 10

    const school = useSchoolStore()
    school.school = SCHOOL
    school.offDays = []
    school.tags = [{id: 7, name: '체험학습'}]
    school.rules = []
    school.classTags = [{id: 5, name: '프로그래밍', sortOrder: 0, classCount: 2}]
})

describe('설정 — 학년도', () => {
    it('학년도를 나열하고, 선택하면 스토어 액션을 거친다', async () => {
        const app = useAppStore()
        const select = vi.spyOn(app, 'selectYear').mockResolvedValue()
        const wrapper = await render()

        const picker = row(wrapper, '이번 학년도')
        expect(picker.findAll('button').map((b) => b.text()))
            .toEqual(['2026학년도', '2027학년도'])
        expect(button(picker, '2026학년도').classes()).toContain('is-on')

        await button(picker, '2027학년도').trigger('click')
        expect(select).toHaveBeenCalledWith(3)
    })

    it('이미 보고 있는 학년도는 다시 선택하지 않는다', async () => {
        const select = vi.spyOn(useAppStore(), 'selectYear').mockResolvedValue()
        const wrapper = await render()

        await button(row(wrapper, '이번 학년도'), '2026학년도').trigger('click')
        expect(select).not.toHaveBeenCalled()
    })

    it('새 학년도를 만든다 — 다음 해가 미리 적혀 있다', async () => {
        const app = useAppStore()
        // 스토어에 아직 없는 액션이다. 화면이 무엇을 부르는지 여기서 못박는다.
        app.createYear = vi.fn().mockResolvedValue(4)
        const wrapper = await render()

        const adding = row(wrapper, '학년도 추가')
        expect(adding.find('input').element.value).toBe('2027')
        // 3월에 열린다는 것을 화면이 그대로 말한다.
        expect(adding.text()).toContain('2027-03-01')

        await button(adding, '추가').trigger('click')
        await flushPromises()

        expect(app.createYear).toHaveBeenCalledWith(2027)
    })
})

describe('설정 — 학교', () => {
    it('학교가 없으면 그 사실을 말하고, 그 자리에서 만든다', async () => {
        const app = useAppStore()
        app.schools = []
        app.classes = []
        const school = useSchoolStore()
        school.school = null

        const create = vi.spyOn(school, 'createSchool').mockImplementation(async ({name}) => {
            app.schools = [{...SCHOOL, name}]
            return 1
        })
        const select = vi.spyOn(app, 'selectSchool').mockResolvedValue()

        const wrapper = await render()
        expect(wrapper.text()).toContain('2026학년도에 추가한 학교가 없습니다')
        // 학교가 없으면 고칠 것도 없다. 빈 칸을 늘어놓지 않는다.
        expect(row(wrapper, '최대 교시')).toBeUndefined()

        const picker = row(wrapper, '2026학년도의 학교')
        await picker.find('input[type="text"]').setValue('푸른중학교')
        await button(picker, '학교 추가').trigger('click')
        await flushPromises()

        // 기본 태그 · 한도 규정이 함께 들어가는 길은 이 액션 하나뿐이다.
        expect(create).toHaveBeenCalledWith({name: '푸른중학교'})
        // 만든 학교로 옮겨야 아래의 최대 교시 · 제출 기한이 그 학교를 가리킨다.
        expect(select).toHaveBeenCalledWith(1)
    })

    it('학교를 선택하면 그 학교로 옮긴다', async () => {
        const app = useAppStore()
        app.schools = [SCHOOL, OTHER_SCHOOL]
        const select = vi.spyOn(app, 'selectSchool').mockResolvedValue()

        const wrapper = await render()
        const picker = row(wrapper, '2026학년도의 학교')
        expect(button(picker, '한빛고등학교').classes()).toContain('is-on')

        await button(picker, '푸른중학교').trigger('click')
        expect(select).toHaveBeenCalledWith(2)
    })

    it('학교 마감은 확인을 거치고, 지운다고 말하지 않는다', async () => {
        const school = useSchoolStore()
        const retire = vi.spyOn(school, 'retireSchool').mockResolvedValue()

        const wrapper = await render()
        await button(row(wrapper, '학교 마감'), '마감').trigger('click')

        expect(retire).not.toHaveBeenCalled()
        const modal = document.querySelector('.modal')
        expect(modal.textContent).toContain('지우는 것이 아닙니다')
        expect(modal.textContent).toContain('한빛고등학교')

        modalButton('마감').click()
        await flushPromises()

        expect(retire).toHaveBeenCalledWith(1)
        wrapper.unmount()
    })

    it('학교가 바뀌면 그 학교의 설정을 다시 읽는다', async () => {
        const app = useAppStore()
        app.schools = [SCHOOL, OTHER_SCHOOL]
        const wrapper = await render()

        const school = useSchoolStore()
        school.fetchAll.mockClear()
        app.pickedSchoolId = 2
        await flushPromises()

        // 다시 읽지 않으면 옮기기 전 학교의 값이 칸에 남고, 저장하면 그 값이 적힌다.
        expect(school.fetchAll).toHaveBeenCalled()
        wrapper.unmount()
    })
})

describe('설정 — 한도 규정', () => {
    /** 기간 단추가 모인 줄. 태그 · 기간 · 단위가 한 줄에 함께 있다. */
    const adding = (wrapper) => row(wrapper, '규정 추가')

    it('기간은 학년도 · 달 둘뿐이다 — 학기는 누르면 반드시 실패한다', async () => {
        const wrapper = await render()

        // 1 · 2학기 경계가 학교마다 달라 뺀 단위다. Rust도 이 둘만 받으므로
        // 여기에 셋째를 두면 누를 수는 있는데 저장이 반드시 실패한다.
        expect(button(adding(wrapper), '학기')).toBeUndefined()
        expect(button(adding(wrapper), '학년도')).toBeDefined()
        expect(button(adding(wrapper), '달')).toBeDefined()
    })

    it('선택한 기간을 그대로 보낸다', async () => {
        const create = vi.spyOn(useSchoolStore(), 'createRule').mockResolvedValue()
        const wrapper = await render()

        await adding(wrapper).find('input[type="text"]').setValue('생리통 월 1회')
        await button(adding(wrapper), '달').trigger('click')
        await button(adding(wrapper), '추가').trigger('click')
        await flushPromises()

        expect(create).toHaveBeenCalledWith({
            name: '생리통 월 1회', tagId: null, period: 'month', limitN: 20, unit: 'day',
        })
    })

    it('저장이 실패하면 적은 것을 비우지 않고 이유를 말한다', async () => {
        vi.spyOn(useSchoolStore(), 'createRule').mockRejectedValue(new Error('기간 값이 틀렸습니다'))
        const wrapper = await render()

        await adding(wrapper).find('input[type="text"]').setValue('체험학습 연 20일')
        await button(adding(wrapper), '추가').trigger('click')
        await flushPromises()

        // 네 번 눌러 채운 칸이다. 비우면 처음부터 다시 입력해야 한다.
        expect(adding(wrapper).find('input[type="text"]').element.value).toBe('체험학습 연 20일')
        expect(wrapper.text()).toContain('한도 규정을 저장하지 못했습니다')
    })

    it('모르는 기간도 빈칸으로 두지 않는다', async () => {
        useSchoolStore().rules = [{
            id: 9, name: '옛 규정', tagId: null, tagName: null,
            period: 'semester', limitN: 1, unit: 'day',
        }]
        const wrapper = await render()

        // 비우면 `마다 1일`이 되어 무엇을 세는 규정인지 화면에서 사라진다.
        expect(row(wrapper, '옛 규정').text()).toContain('semester마다')
    })
})

describe('설정 — 담당 학급 · 강좌와 강좌 묶음', () => {
    /** 교과 강좌 줄. 이름표가 '교과'인 줄이 그것이다. */
    function subjectRows(wrapper) {
        return wrapper.findAll('.set__row').filter((r) => r.find('.set__label').text() === '교과')
    }

    it('강좌가 묶음별로 모이고, 묶음 없는 것이 맨 뒤에 온다', async () => {
        const wrapper = await render()

        expect(wrapper.findAll('.grp b').map((b) => b.text()))
            .toEqual(['프로그래밍', '묶음 없음'])
        // 어느 줄도 사라지지 않는다. 묶음이 없는 강좌도 목록에 남는다.
        expect(subjectRows(wrapper).map((r) => r.find('input').element.value))
            .toEqual(['프로그래밍A', '프로그래밍B', '통합사회'])
    })

    it('강좌의 묶음을 바꾸면 이름을 그대로 실어 보낸다', async () => {
        const rename = vi.spyOn(useSchoolStore(), 'renameClass').mockResolvedValue()
        const wrapper = await render()

        const first = subjectRows(wrapper)[0]
        expect(button(first, '프로그래밍').classes()).toContain('is-on')

        await button(first, '없음').trigger('click')
        await flushPromises()

        // 이름을 빼면 묶음만 고쳤는데 강좌 이름이 비워진다.
        expect(rename).toHaveBeenCalledWith(21, {
            name: '프로그래밍A', grade: null, classNo: null, groupTagId: null,
        })
    })

    it('이름만 고쳐도 묶음이 벗겨지지 않는다', async () => {
        const rename = vi.spyOn(useSchoolStore(), 'renameClass').mockResolvedValue()
        const wrapper = await render()

        const input = subjectRows(wrapper)[0].find('input')
        await input.setValue('프로그래밍 A반')
        await input.trigger('change')
        await flushPromises()

        expect(rename).toHaveBeenCalledWith(21, {
            name: '프로그래밍 A반', grade: null, classNo: null, groupTagId: 5,
        })
    })

    it('새 묶음을 그 자리에서 만들고, 만든 묶음이 곧바로 선택된다', async () => {
        const school = useSchoolStore()
        const create = vi.spyOn(school, 'createClassTag').mockImplementation(async (name) => {
            school.classTags = [...school.classTags, {id: 9, name, sortOrder: 1, classCount: 0}]
            return 9
        })
        const wrapper = await render()

        const tags = row(wrapper, '강좌 묶음')
        await tags.find('input[placeholder="새 묶음"]').setValue('인공지능')
        await button(tags, '묶음 추가').trigger('click')
        await flushPromises()

        expect(create).toHaveBeenCalledWith('인공지능')
        // 묶음을 만드는 이유가 지금 더하는 강좌를 거기 넣으려는 것이다.
        expect(button(row(wrapper, '교과 추가'), '인공지능').classes()).toContain('is-on')
    })

    it('교과를 더할 때 선택한 묶음이 함께 간다', async () => {
        const create = vi.spyOn(useSchoolStore(), 'createClass').mockResolvedValue(30)
        const wrapper = await render()

        const adding = row(wrapper, '교과 추가')
        await adding.find('input[type="text"]').setValue('프로그래밍C')
        await button(adding, '프로그래밍').trigger('click')
        await button(adding, '추가').trigger('click')
        await flushPromises()

        expect(create).toHaveBeenCalledWith({
            schoolId: 1, role: 'subject', name: '프로그래밍C', groupTagId: 5, select: false,
        })
    })

    it('묶음 삭제는 확인을 거치고, 강좌는 남는다고 말한다', async () => {
        const drop = vi.spyOn(useSchoolStore(), 'deleteClassTag').mockResolvedValue()
        const wrapper = await render()

        await row(wrapper, '강좌 묶음').find('[title="프로그래밍 묶음 지우기"]').trigger('click')

        expect(drop).not.toHaveBeenCalled()
        expect(document.querySelector('.modal').textContent).toContain('강좌는 지워지지 않습니다')

        modalButton('지우기').click()
        await flushPromises()

        expect(drop).toHaveBeenCalledWith(5)
        wrapper.unmount()
    })

    /**
     * 통칭을 새로 만들지 않는다. `맡은 것`은 담임 학급과 교과 강좌를 한 낱말로 묶으려다
     * 나온 말인데, 교사는 그 둘을 묶어 부르지 않는다. 이 화면은 그 목록을 들고 있는
     * 유일한 화면이라 여기서 갈라지면 다른 화면이 그것을 보고 따라 쓴다.
     */
    it('담당 학급 · 강좌라고 적는다 — `맡은 것`이라는 통칭을 쓰지 않는다', async () => {
        const wrapper = await render()

        const heads = wrapper.findAll('.ledger__head h4').map((h) => h.text())
        expect(heads).toContain('담당 학급 · 강좌')
        expect(wrapper.text()).not.toContain('맡은 것')
    })

    /** 마감 확인 모달도 같은 말을 쓴다. 모달만 옛말이 남으면 두 화면이 갈라진다. */
    it('마감 확인 모달에도 `맡은 것`이 남지 않는다', async () => {
        const wrapper = await render()

        await button(subjectRows(wrapper)[0], '마감').trigger('click')
        await flushPromises()

        const modal = document.querySelector('.modal').textContent
        expect(modal).toContain('교과 강좌')
        expect(modal).not.toContain('맡은 것')
        wrapper.unmount()
    })

    it('교과 명렬표는 담임과 같은 자리다 — 잠긴 단추를 남기지 않는다', async () => {
        const wrapper = await render()

        const roster = button(subjectRows(wrapper)[0], '명렬표')
        expect(roster.attributes('disabled')).toBeUndefined()
        expect(wrapper.text()).not.toContain('명렬표는 아직')

        await roster.trigger('click')
        expect(wrapper.text()).toContain('프로그래밍A 명단에 등록합니다')
    })
})
