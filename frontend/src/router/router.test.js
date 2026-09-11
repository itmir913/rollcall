/**
 * 길 — 사이드바 구성과 모드 가드.
 *
 * **한 줄도 새지 않는다.** 담임 모드의 목록에 교과 화면이 오면, 비담임 교사가
 * 매일 남의 일을 보게 되고 그 화면은 교과 강좌의 범위로 담임 질의를 던진다.
 * 사이드바에서 지우는 것만으로는 겉모양뿐이라 주소로 들어오는 길도 함께 막는다.
 *
 * 잠긴 화면은 **지우지 않고 개요로 되돌린다** — 담임 학급이 없는 담임 모드는
 * 정상 상태이고, 개요가 그 자리에서 무엇을 등록해야 하는지 알린다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import router, {NAV_FOOT, NAV_GROUPS, modeRedirect, navGroups, welcomeRedirect} from './index'
import {useAppStore} from '../stores/app'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn().mockResolvedValue([])}))

const SCHOOL = {id: 1, yearId: 2, name: '한빛고등학교', maxSlot: 7, dueDays: 7}
const HOMEROOM = {id: 9, schoolId: 1, role: 'homeroom', name: '3학년 6반', validTo: null}
const SUBJECT = {id: 21, schoolId: 1, role: 'subject', name: '인공지능기초 A반', validTo: null}

/** 라우터를 띄우지 않고 판단만 시험한다. 가드는 이 함수들을 읽을 뿐이다. */
function scope({classes = [HOMEROOM], classId = 9, mode = 'homeroom'} = {}) {
    const app = useAppStore()
    app.booted = true
    app.years = [{id: 2, year: 2026}]
    app.yearId = 2
    app.schools = [SCHOOL]
    app.pickedSchoolId = 1
    app.classes = classes
    app.classId = classId
    app.lastMode = mode
    return app
}

/** 라우트 경로 → meta. 사이드바가 가리키는 곳이 실제로 있는지 본다. */
const META = new Map(router.getRoutes().map((r) => [r.path, r.meta ?? {}]))

beforeEach(() => {
    setActivePinia(createPinia())
})

describe('사이드바 구성', () => {
    it('가리키는 화면이 모두 있다 — 이름과 라우트가 여기 한 곳에서 이어진다', () => {
        const links = [...Object.values(NAV_GROUPS).flat(2), ...NAV_FOOT]
        for (const link of links) {
            expect(META.has(link.to), `없는 화면을 가리킨다: ${link.to}`).toBe(true)
        }
    })

    it('모드 목록에 다른 모드의 화면이 한 줄도 없다', () => {
        for (const [mode, groups] of Object.entries(NAV_GROUPS)) {
            for (const link of groups.flat()) {
                const need = META.get(link.to).mode
                expect(need ?? mode, `${mode} 목록에 ${link.to}가 있다`).toBe(mode)
                // 두 모드가 함께 쓰는 줄만 `always`다. 담당 학급 · 강좌가 없어도 열려야 하는
                // 줄이 그 둘이고, 나머지는 잠긴다.
                expect(Boolean(link.always), `always 표시가 어긋난다: ${link.to}`)
                    .toBe(need === undefined)
            }
        }
    })

    it('교과 모드에는 교과 화면이 온다 — 개요 · 이동만 있던 자리다', () => {
        const labels = navGroups('subject').flat().map((l) => l.label)
        expect(labels).toEqual(['개요', '이동', '오늘 수업', '수업 기록'])
    })

    it('담임 모드의 목록은 세 덩어리다 — 매일 · 월말 · 함께 쓰는 것', () => {
        expect(navGroups('homeroom')).toHaveLength(3)
    })

    it('모르는 모드에는 함께 쓰는 줄만 돌려준다 — 담임 목록을 돌려주면 통째로 샌다', () => {
        expect(navGroups('부담임').flat().map((l) => l.label)).toEqual(['개요', '이동'])
        expect(navGroups(undefined).flat().map((l) => l.label)).toEqual(['개요', '이동'])
    })

    it('맨 아래 둘은 업무가 아니므로 모드를 가리지 않는다', () => {
        for (const link of NAV_FOOT) {
            expect(META.get(link.to).mode).toBeUndefined()
        }
    })
})

describe('모드 가드', () => {
    it('부팅 전에는 판단하지 않는다 — 그때는 모드가 기본값이다', () => {
        const app = useAppStore()
        expect(modeRedirect(app, {meta: {mode: 'subject'}})).toBe(true)
    })

    it('다른 모드의 화면은 개요로 되돌린다', () => {
        const app = scope({classes: [HOMEROOM, SUBJECT], classId: 21, mode: 'subject'})

        expect(modeRedirect(app, {meta: {mode: 'homeroom'}})).toEqual({name: 'overview'})
        expect(modeRedirect(app, {meta: {mode: 'subject'}})).toBe(true)
        // 두 모드가 함께 쓰는 화면은 그대로 통과한다.
        expect(modeRedirect(app, {meta: {}})).toBe(true)
    })

    it('그 모드에 담당 학급 · 강좌가 없으면 되돌린다 — 범위 없이 질의를 던지지 않는다', () => {
        const app = scope({classes: [SUBJECT], classId: null, mode: 'homeroom'})

        expect(app.mode).toBe('homeroom')
        expect(modeRedirect(app, {meta: {mode: 'homeroom'}})).toEqual({name: 'overview'})
        // 개요 · 이동 · 설정은 그대로 열린다. 등록하러 갈 길이 막히면 안 된다.
        expect(modeRedirect(app, {meta: {}})).toBe(true)
    })

    it('교과 강좌를 등록하지 않은 교과 모드도 같다', () => {
        const app = scope({classes: [HOMEROOM], classId: null, mode: 'subject'})

        expect(modeRedirect(app, {meta: {mode: 'subject'}})).toEqual({name: 'overview'})
    })
})

describe('Welcome 가드', () => {
    it('담당 학급 · 강좌가 있으면 붙잡지 않는다 — 고른 학급이 없어도', () => {
        const app = scope({classes: [SUBJECT], classId: null, mode: 'homeroom'})

        expect(welcomeRedirect(app, {name: 'overview'})).toBe(true)
    })

    it('담당 학급 · 강좌가 하나도 없으면 Welcome으로 보낸다', () => {
        const app = scope({classes: [], classId: null})

        expect(welcomeRedirect(app, {name: 'today'})).toEqual({name: 'welcome'})
        expect(welcomeRedirect(app, {name: 'welcome'})).toBe(true)
    })
})
