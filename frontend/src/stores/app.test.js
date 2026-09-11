/**
 * 앱 스토어 — 부팅과 범위.
 *
 * 부팅이 하는 일은 셋이다. 맡은 것을 읽어오는 것, 마지막에 본 학급을 되살리는 것,
 * 아직 시작하지 않은 상태인지 판단하는 것. 마지막을 버리면 처음 켠 교사가 안내 없이
 * 개요에 놓이고, 거기서 할 수 있는 일이 없다.
 *
 * 이 파일이 붙들고 있는 결정은 하나 더 있다 — **모드와 학급이 어긋나지 않는다.**
 * 담임 화면에 교과 강좌가 걸리면 DB 트리거가 거절할 때까지 아무도 모른다.
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

const HOMEROOM = {
    id: 9, schoolId: 1, yearId: 2, role: 'homeroom', name: '3학년 6반',
    grade: 3, classNo: 6, sortOrder: 10, validFrom: '2026-03-02', validTo: null,
}
const SUBJECT = {
    id: 21, schoolId: 1, yearId: 2, role: 'subject', name: '지구과학Ⅰ',
    grade: null, classNo: null, sortOrder: 20, validFrom: '2026-03-02', validTo: null,
}

/** 부팅이 부르는 커맨드를 한꺼번에 흉내 낸다. */
function mockBoot({created = false, config = {}, classes = [HOMEROOM]} = {}) {
    invoke.mockImplementation(async (command, args) => {
        switch (command) {
            case 'init_db':
                return {path: '어딘가/rollcall.db', created, dbVersion: 1, appVersion: 1, needsMigration: false}
            case 'get_schools':
                return [{id: 1, name: '한빛고등학교', maxSlot: 6, dueDays: 7}]
            case 'get_years':
                return [{id: 2, year: 2026}]
            case 'get_teaching_classes':
                return classes
            case 'get_config':
                return config[args.key] ?? null
            case 'create_teaching_class':
                // 만든 직후의 목록을 그대로 흉내 낸다 — 새 학급이 그 목록의 첫 줄이다.
                return classes[0]?.id ?? null
            default:
                return null
        }
    })
}

/** 담임 · 교과를 함께 맡고 둘 다 본 적이 있는 상태. 스위치가 필요한 교사다. */
const BOTH = {
    classes: [HOMEROOM, SUBJECT],
    config: {yearId: '2', mode: 'homeroom', homeroomClassId: '9', subjectClassId: '21'},
}

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
})

describe('부팅', () => {
    it('첫 실행 여부를 버리지 않는다 — init_db가 알려준 것을 들고 있는다', async () => {
        mockBoot({created: true, classes: []})
        const app = useAppStore()

        await app.boot()

        expect(app.firstRun).toBe(true)
        expect(app.booted).toBe(true)
    })

    it('있던 파일을 연 것은 첫 실행이 아니다', async () => {
        mockBoot({created: false, classes: []})
        const app = useAppStore()

        await app.boot()

        expect(app.firstRun).toBe(false)
    })

    it('학급이 골라졌으면 Welcome으로 보내지 않는다', async () => {
        mockBoot({config: {yearId: '2', mode: 'homeroom', homeroomClassId: '9'}})
        const app = useAppStore()

        await app.boot()

        expect(app.needsWelcome).toBe(false)
    })

    it('온보딩을 끝내지 않고 닫았어도 다시 Welcome으로 보낸다', async () => {
        // `firstRun`으로 판단하면 DB 파일을 만든 그 한 번의 실행에서만 참이라,
        // 학급을 만들기 전에 앱을 닫으면 사이드바에 항목도 없는 Welcome에
        // 다시 닿을 길이 없어진다.
        mockBoot({created: false, classes: []})
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

    it('마지막에 본 학급을 복원한다 — 매일 아침 다시 고르지 않게 한다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()

        await app.boot()

        expect(app.mode).toBe('homeroom')
        expect(app.classId).toBe(9)
        expect(app.currentClass.name).toBe('3학년 6반')
        expect(app.ready).toBe(true)
    })

    it('목록에 없는 번호는 버린다 — 지난해 학급으로 물으면 전부 빈다', async () => {
        // 학년도가 바뀌면 지난해 학급은 마감되어 목록에서 빠진다. 설정에 적힌
        // 번호를 그대로 믿으면 모든 화면이 "기록 없음"으로 보인다.
        mockBoot({config: {yearId: '2', mode: 'homeroom', homeroomClassId: '4'}})
        const app = useAppStore()

        await app.boot()

        expect(app.classId).toBe(9)
    })

    it('맡은 것이 없는 모드로 열리면 있는 쪽으로 옮긴다 — 비담임 교사', async () => {
        // 담임 모드로 적혀 있어도 담임 학급이 없으면 아무것도 없는 화면이 뜬다.
        mockBoot({config: {mode: 'homeroom', subjectClassId: '21'}, classes: [SUBJECT]})
        const app = useAppStore()

        await app.boot()

        expect(app.mode).toBe('subject')
        expect(app.classId).toBe(21)
        expect(app.needsWelcome).toBe(false)
    })

    it('실패는 error에 담고 다시 던진다 — 삼키면 빈 화면과 구분되지 않는다', async () => {
        invoke.mockRejectedValue('데이터 파일이 열려 있지 않습니다.')
        const app = useAppStore()

        await expect(app.boot()).rejects.toBeTruthy()

        expect(app.error).toContain('데이터 파일이 열려 있지 않습니다.')
        expect(app.booted).toBe(true)
    })
})

describe('범위는 학급 하나다', () => {
    it('학교 설정은 학급에서 얻는다 — 최대 교시를 앱이 들고 있지 않는다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        expect(app.schoolId).toBe(1)
        expect(app.maxSlot).toBe(6)
    })

    it('학급이 아직 없어도 학교는 읽힌다 — Welcome이 최대 교시를 먼저 정한다', async () => {
        mockBoot({classes: []})
        const app = useAppStore()
        await app.boot()

        expect(app.schoolId).toBe(1)
        expect(app.maxSlot).toBe(6)
    })

    it('맡지 않은 학급은 고를 수 없다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        await expect(app.selectClass(999)).rejects.toBeTruthy()
        expect(app.error).toContain('맡은 학급이 아닙니다')
        expect(app.classId).toBe(9)
    })
})

describe('모드와 학급', () => {
    it('학급을 고르면 모드가 그 학급의 역할로 따라온다 — 어긋나지 않는다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        await app.selectClass(21)

        expect(app.mode).toBe('subject')
        expect(app.classId).toBe(21)
        expect(app.isHomeroom).toBe(false)
    })

    /**
     * 모드를 따로 담으면 담임 화면에 교과 강좌가 걸린 상태가 만들어지고, 그때는
     * DB 트리거가 거절할 때까지 아무도 모른다. 그래서 모드는 담지 않고 읽는다.
     */
    it('모드는 담아 두는 값이 아니라 고른 학급이 말하는 값이다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()
        expect(app.mode).toBe('homeroom')

        // 액션을 거치지 않고 학급만 옮겨도 모드가 따라온다.
        app.classId = 21

        expect(app.mode).toBe('subject')
    })

    it('모드를 바꾸면 그 모드에서 마지막에 본 학급으로 돌아온다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        await app.setMode('subject')
        expect(app.classId).toBe(21)

        await app.setMode('homeroom')
        expect(app.classId).toBe(9)
    })

    it('교과에서 다른 강좌를 보고 돌아와도 담임 학급은 그대로다', async () => {
        // 오가며 쓰는 값이라 한쪽에서 고른 것이 다른 쪽을 덮으면 매번 다시 골라야 한다.
        const 둘째강좌 = {...SUBJECT, id: 22, name: '지구과학Ⅱ'}
        mockBoot({...BOTH, classes: [HOMEROOM, SUBJECT, 둘째강좌]})
        const app = useAppStore()
        await app.boot()

        await app.selectClass(22)
        await app.setMode('homeroom')

        expect(app.classId).toBe(9)
        await app.setMode('subject')
        expect(app.classId).toBe(22)
    })

    it('모르는 모드는 받지 않는다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        await expect(app.setMode('부담임')).rejects.toBeTruthy()
        expect(app.mode).toBe('homeroom')
    })

    it('모드마다 마지막 자리를 따로 적는다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()
        invoke.mockClear()

        await app.selectClass(21)

        const keys = invoke.mock.calls
            .filter((c) => c[0] === 'set_config')
            .map((c) => c[1].key)
        expect(keys).toContain('mode')
        expect(keys).toContain('subjectClassId')
        expect(keys).not.toContain('homeroomClassId')
        expect(keys).not.toContain('grade')
        expect(keys).not.toContain('classNo')
    })
})

describe('모드 스위치', () => {
    it('맡은 것이 한쪽뿐이면 그릴 필요가 없다', async () => {
        mockBoot({classes: [HOMEROOM], config: {mode: 'homeroom', homeroomClassId: '9'}})
        const app = useAppStore()
        await app.boot()

        // 누를 때마다 아무것도 없는 화면으로 넘어가는 단추를 두지 않는다.
        expect(app.needsModeSwitch).toBe(false)
    })

    it('둘 다 맡았으면 그린다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        expect(app.needsModeSwitch).toBe(true)
        expect(app.homeroomClasses).toHaveLength(1)
        expect(app.subjectClasses).toHaveLength(1)
    })
})

describe('맡은 것 만들기', () => {
    it('만들면 그 학급으로 옮긴다', async () => {
        mockBoot({classes: []})
        const app = useAppStore()
        await app.boot()

        // 만든 뒤의 목록에는 새 학급이 들어 있다.
        const 새학급 = {...HOMEROOM, id: 77, name: '2학년 1반', grade: 2, classNo: 1}
        mockBoot({classes: [새학급]})

        await app.createClass({role: 'homeroom', name: '2학년 1반', grade: 2, classNo: 1})

        expect(app.classId).toBe(77)
        expect(app.mode).toBe('homeroom')
        expect(app.needsWelcome).toBe(false)
    })

    it('같은 학급을 두 번 만들지 않는다 — [저장]을 두 번 누르는 것은 흔한 일이다', async () => {
        mockBoot({config: {mode: 'homeroom', homeroomClassId: '9'}})
        const app = useAppStore()
        await app.boot()
        invoke.mockClear()

        const id = await app.createClass({role: 'homeroom', name: '3학년 6반', grade: 3, classNo: 6})

        expect(id).toBe(9)
        // 늘어나면 명단이 어느 쪽에 붙었는지 알 수 없게 된다.
        expect(invoke.mock.calls.map((c) => c[0])).not.toContain('create_teaching_class')
    })
})

describe('첫 실행 흐름', () => {
    it('맡은 것이 없으면 Welcome으로 보낸다', async () => {
        mockBoot({created: true, classes: []})
        const app = useAppStore()
        await app.boot()

        expect(app.needsWelcome).toBe(true)
        expect(welcomeRedirect(app, {name: 'today'})).toEqual({name: 'welcome'})
    })

    it('학급을 만들면 더는 붙잡지 않는다 — [시작하기]가 개요로 넘어간다', async () => {
        mockBoot({created: true, classes: []})
        const app = useAppStore()
        await app.boot()

        mockBoot({classes: [HOMEROOM]})
        await app.createClass({role: 'homeroom', name: '3학년 6반', grade: 3, classNo: 6})

        expect(app.needsWelcome).toBe(false)
        expect(welcomeRedirect(app, {name: 'overview'})).toBe(true)
    })

    it('Welcome 자신은 되돌리지 않는다 — 그 자리에서 이동이 끝나지 않는다', async () => {
        mockBoot({created: true, classes: []})
        const app = useAppStore()
        await app.boot()

        expect(welcomeRedirect(app, {name: 'welcome'})).toBe(true)
    })

    it('부팅 전에는 판단하지 않는다 — 아직 아무것도 모른다', () => {
        const app = useAppStore()

        expect(welcomeRedirect(app, {name: 'today'})).toBe(true)
    })
})
