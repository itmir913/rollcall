/**
 * 앱 스토어 — 부팅과 첫 실행 흐름.
 *
 * 부팅이 하는 일은 둘이다. 마지막에 연 학급을 복원하는 것과, 아직 시작하지 않은
 * 상태인지 판단하는 것. 뒤의 것을 버리면 처음 켠 교사가 안내 없이 개요에 놓이고,
 * 거기서 할 수 있는 일이 없다.
 *
 * 흐름(`Welcome → 개요`)의 판단은 라우터가 아니라 이 스토어가 쥔다.
 * 가드는 그 값을 읽을 뿐이라 여기서 함께 확인한다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'
import {welcomeRedirect} from '../router'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))

/** 부팅이 부르는 커맨드를 한꺼번에 흉내 낸다. */
function mockBoot({created = false, config = {}} = {}) {
    invoke.mockImplementation(async (command, args) => {
        switch (command) {
            case 'init_db':
                return {path: '어딘가/rollcall.db', created, dbVersion: 1, appVersion: 1, needsMigration: false}
            case 'get_schools':
                return [{id: 1, name: '한빛고등학교', maxSlot: 7}]
            case 'get_years':
                return [{id: 2, year: 2026}]
            case 'get_config':
                return config[args.key] ?? null
            default:
                return null
        }
    })
}

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
})

describe('부팅', () => {
    it('첫 실행 여부를 버리지 않는다 — init_db가 알려준 것을 들고 있는다', async () => {
        mockBoot({created: true})
        const app = useAppStore()

        await app.boot()

        expect(app.firstRun).toBe(true)
        expect(app.booted).toBe(true)
    })

    it('있던 파일을 연 것은 첫 실행이 아니다', async () => {
        mockBoot({created: false})
        const app = useAppStore()

        await app.boot()

        expect(app.firstRun).toBe(false)
    })

    it('학급이 정해졌으면 Welcome으로 보내지 않는다', async () => {
        mockBoot({created: false, config: {yearId: '2', grade: '3', classNo: '6'}})
        const app = useAppStore()

        await app.boot()

        expect(app.needsWelcome).toBe(false)
    })

    it('온보딩을 끝내지 않고 닫았어도 다시 Welcome으로 보낸다', async () => {
        // `firstRun`으로 판단하면 DB 파일을 만든 그 한 번의 실행에서만 참이라,
        // 학급을 정하기 전에 앱을 닫으면 사이드바에 항목도 없는 Welcome에
        // 다시 닿을 길이 없어진다.
        mockBoot({created: false})
        const app = useAppStore()

        await app.boot()

        expect(app.firstRun).toBe(false)
        expect(app.needsWelcome).toBe(true)
    })

    it('부팅 전에는 판단하지 않는다 — 아직 아무것도 모른다', () => {
        const app = useAppStore()
        expect(app.booted).toBe(false)
        expect(app.needsWelcome).toBe(false)
    })

    it('마지막에 연 학급을 복원한다 — 매일 아침 다시 고르지 않게 한다', async () => {
        mockBoot({config: {yearId: '2', grade: '3', classNo: '6'}})
        const app = useAppStore()

        await app.boot()

        expect(app.schoolId).toBe(1)
        expect(app.yearId).toBe(2)
        expect(app.grade).toBe(3)
        expect(app.classNo).toBe(6)
        expect(app.ready).toBe(true)
    })

    it('실패는 error에 담고 다시 던진다 — 삼키면 빈 화면과 구분되지 않는다', async () => {
        invoke.mockRejectedValue('데이터 파일이 열려 있지 않습니다.')
        const app = useAppStore()

        await expect(app.boot()).rejects.toBeTruthy()

        expect(app.error).toContain('데이터 파일이 열려 있지 않습니다.')
        expect(app.booted).toBe(true)
    })
})

describe('첫 실행 흐름', () => {
    it('새 파일이면 Welcome으로 보낸다', async () => {
        mockBoot({created: true})
        const app = useAppStore()
        await app.boot()

        expect(app.needsWelcome).toBe(true)
        expect(welcomeRedirect(app, {name: 'today'})).toEqual({name: 'welcome'})
    })

    it('학급을 정하면 더는 붙잡지 않는다 — [시작하기]가 개요로 넘어간다', async () => {
        mockBoot({created: true})
        const app = useAppStore()
        await app.boot()

        await app.selectClass({yearId: 2, grade: 3, classNo: 6})

        expect(app.needsWelcome).toBe(false)
        expect(welcomeRedirect(app, {name: 'overview'})).toBe(true)
    })

    it('Welcome 자신은 되돌리지 않는다 — 그 자리에서 이동이 끝나지 않는다', async () => {
        mockBoot({created: true})
        const app = useAppStore()
        await app.boot()

        expect(welcomeRedirect(app, {name: 'welcome'})).toBe(true)
    })

    it('부팅 전에는 판단하지 않는다 — 아직 아무것도 모른다', () => {
        const app = useAppStore()

        expect(welcomeRedirect(app, {name: 'today'})).toBe(true)
    })
})
