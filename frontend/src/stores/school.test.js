/**
 * 학교 스토어 — 학교에 종속된 값들과 담당 학급 · 강좌.
 *
 * 이 파일이 지키는 결정은 셋이다.
 *   · **커맨드는 평평한 인자를 받는다.** 화면은 고친 칸만 넘기고 나머지는 지금 값으로
 *     채운다. 화면마다 다섯 값을 다 들고 있게 하면 그중 하나가 낡아 소리 없이 옛 값으로
 *     되돌아간다.
 *   · **마감은 삭제가 아니다.** 담당 학급 · 강좌를 마감해도 지난 기록은 그 학급을 가리킨 채 남고,
 *     보고 있던 학급이 사라지면 같은 학교 · 같은 모드의 남은 것으로 옮긴다.
 *     모드를 말없이 넘기지 않는다.
 *   · **가리키는 학교가 없으면 아무것도 들고 있지 않는다.** 지난 학교의 값이 남으면
 *     그 화면의 [저장]이 지난해 학교를 고친다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'
import {useSchoolStore} from './school'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))

const A = {
    id: 1, yearId: 2, name: '한빛고등학교', maxSlot: 6, dueDays: 7,
    dueSkipOffdays: true, sortOrder: 10, active: true,
}
const B = {...A, id: 2, name: '푸른중학교', maxSlot: 7, sortOrder: 20}

const HOMEROOM = {
    id: 9, schoolId: 1, role: 'homeroom', name: '3학년 6반', grade: 3, classNo: 6,
    groupTagId: null, groupTagName: null, sortOrder: 10,
    validFrom: '2026-03-02', validTo: null, memberCount: 28,
}
const SUBJECT = {
    ...HOMEROOM, id: 21, role: 'subject', name: '프로그래밍A',
    grade: null, classNo: null, groupTagId: 3, groupTagName: '프로그래밍', sortOrder: 20,
}

/** 부팅을 흉내 내지 않고 범위만 설정한다 — 이 테스트가 보는 것은 학교 쪽 커맨드다. */
function scope({schools = [A], classes = [HOMEROOM, SUBJECT], classId = 9, mode = 'homeroom'} = {}) {
    const app = useAppStore()
    app.booted = true
    app.today = '2026-09-11'
    app.years = [{id: 2, year: 2026, startsOn: '2026-03-02', endsOn: '2027-02-28'}]
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

/** 읽기 커맨드는 목록으로, 쓰기 커맨드는 조용히 답한다. */
function mockInvoke({classes = [HOMEROOM, SUBJECT]} = {}) {
    invoke.mockImplementation(async (command, args) => {
        switch (command) {
            case 'get_school':
                return A
            case 'get_off_days':
                return [{id: 1, date: '2026-09-15', label: '개교기념일'}]
            case 'get_tags':
                return [{id: 7, name: '체험학습'}]
            case 'get_quota_rules':
                return [{id: 1, name: '체험학습 연 20일'}]
            case 'get_class_tags':
                return [{id: 3, name: '프로그래밍', sortOrder: 10, classCount: 2}]
            case 'get_schools':
                return [A]
            case 'get_teaching_classes':
                return classes.filter((c) => c.schoolId === args.schoolId)
            default:
                return null
        }
    })
}

function argsOf(command) {
    return invoke.mock.calls.filter((c) => c[0] === command).map((c) => c[1])
}

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
})

describe('학교 설정', () => {
    it('학교 하나로 다섯 목록을 읽는다 — 강좌 묶음도 학교의 것이다', async () => {
        scope()
        mockInvoke()
        const school = useSchoolStore()

        await school.fetchAll()

        expect(argsOf('get_class_tags')).toEqual([{schoolId: 1}])
        expect(school.school.name).toBe('한빛고등학교')
        expect(school.classTags).toHaveLength(1)
        expect(school.tags).toHaveLength(1)
        expect(school.rules).toHaveLength(1)
        expect(school.offDays).toHaveLength(1)
    })

    it('학교가 없으면 읽지 않는다 — 가리킬 곳이 없다', async () => {
        scope({schools: [], classes: []})
        mockInvoke()
        const school = useSchoolStore()

        await school.fetchAll()

        expect(invoke).not.toHaveBeenCalled()
    })

    it('학교가 0개가 되면 지난 학교의 값을 비운다 — 그대로 두면 지난해 학교를 고친다', async () => {
        // 2027학년도로 옮긴 아침이 이 상태다. 값이 남아 있으면 2027년 화면에서
        // 최대 교시를 고쳤는데 2026년 A학교의 값이 바뀐다.
        const app = scope()
        mockInvoke()
        const school = useSchoolStore()
        await school.fetchAll()
        expect(school.school.name).toBe('한빛고등학교')

        app.schools = []
        app.pickedSchoolId = null
        await school.fetchAll()

        expect(school.school).toBe(null)
        expect(school.tags).toEqual([])
        expect(school.rules).toEqual([])
        expect(school.offDays).toEqual([])
        expect(school.classTags).toEqual([])
        // 학교가 없는 것은 고장이 아니다. 읽기 실패와 구별되어야 한다.
        expect(school.error).toBe('')
    })

    it('읽기 실패도 비운다 — 남아 있으면 지금 학교의 값으로 읽힌다', async () => {
        scope()
        mockInvoke()
        const school = useSchoolStore()
        await school.fetchAll()

        invoke.mockRejectedValue('학교를 찾을 수 없습니다: 1')
        await expect(school.fetchAll()).rejects.toBeTruthy()

        expect(school.school).toBe(null)
        expect(school.tags).toEqual([])
        // 비어 있는 까닭은 error가 말한다 — 학교가 없는 것과 읽지 못한 것이 구별된다.
        expect(school.error).toContain('학교를 찾을 수 없습니다')
    })

    it('가리키는 학교가 없으면 고치지 않는다 — 번호 없이 보내면 무엇이 잘못됐는지 알 수 없다', async () => {
        scope({schools: [], classes: []})
        mockInvoke()
        const school = useSchoolStore()

        await expect(school.saveSchool({maxSlot: 7})).rejects.toBeTruthy()

        expect(school.error).toContain('고칠 학교가 없습니다')
        expect(invoke.mock.calls.map((c) => c[0])).not.toContain('update_school')
    })

    it('고친 칸만 넘겨도 나머지는 지금 값으로 채워 보낸다', async () => {
        scope()
        mockInvoke()
        const school = useSchoolStore()
        await school.fetchAll()
        invoke.mockClear()
        mockInvoke()

        await school.saveSchool({maxSlot: 7})

        expect(argsOf('update_school')).toEqual([{
            schoolId: 1, name: '한빛고등학교', maxSlot: 7, dueDays: 7, dueSkipOffdays: true,
        }])
    })

    it('학교를 만들 때 학년도를 함께 넘긴다 — 학교는 학년도 안에 있다', async () => {
        const app = scope()
        mockInvoke()
        const school = useSchoolStore()

        await school.createSchool({name: '푸른중학교', maxSlot: 7, dueDays: 5})

        expect(argsOf('create_school')).toEqual([{
            yearId: 2, name: '푸른중학교', maxSlot: 7, dueDays: 5, dueSkipOffdays: true,
        }])
        // 만든 뒤 목록을 다시 읽는다. 읽지 않으면 새 학교로 옮길 길이 없다.
        expect(argsOf('get_schools')).toEqual([{yearId: 2}])
        expect(app.schools).toHaveLength(1)
    })

    it('학교를 내려도 지우지 않는다 — 학생과 출결이 그 학교에 종속되어 있다', async () => {
        scope({schools: [A, B]})
        mockInvoke()
        const school = useSchoolStore()

        await school.retireSchool(2)

        expect(argsOf('retire_school')).toEqual([{schoolId: 2}])
        expect(invoke.mock.calls.map((c) => c[0])).not.toContain('delete_school')
    })

    it('실패는 error에 담고 다시 던진다', async () => {
        scope()
        invoke.mockRejectedValue('학교를 찾을 수 없습니다: 1')
        const school = useSchoolStore()

        await expect(school.fetchAll()).rejects.toBeTruthy()
        expect(school.error).toContain('학교를 찾을 수 없습니다')
    })
})

describe('담당 학급 · 강좌', () => {
    it('이름과 묶음을 함께 고친다 — 묶음이 빠지면 이름표가 조용히 풀린다', async () => {
        scope()
        mockInvoke()
        const school = useSchoolStore()

        await school.renameClass(21, {name: '프로그래밍B', groupTagId: 3})

        expect(argsOf('update_teaching_class')).toEqual([{
            classId: 21, name: '프로그래밍B', grade: null, classNo: null, groupTagId: 3,
        }])
    })

    it('묶음을 넘기지 않으면 지금 것을 그대로 보낸다 — 이름만 고치다 이름표가 풀린다', async () => {
        scope()
        mockInvoke()
        const school = useSchoolStore()

        await school.renameClass(21, {name: '프로그래밍B'})

        expect(argsOf('update_teaching_class')[0].groupTagId).toBe(3)
    })

    it('묶음을 비우는 것은 그대로 전한다 — null은 "풀어 달라"는 뜻이다', async () => {
        scope()
        mockInvoke()
        const school = useSchoolStore()

        await school.renameClass(21, {name: '프로그래밍B', groupTagId: null})

        expect(argsOf('update_teaching_class')[0].groupTagId).toBe(null)
    })

    it('마감한 것이 보고 있던 학급이면 같은 모드의 남은 것으로 옮긴다', async () => {
        const 둘째반 = {...HOMEROOM, id: 10, name: '3학년 7반', classNo: 7}
        const app = scope({classes: [HOMEROOM, 둘째반, SUBJECT]})
        mockInvoke({classes: [둘째반, SUBJECT]})
        const school = useSchoolStore()

        await school.retireClass(9)

        expect(argsOf('retire_teaching_class')).toEqual([{classId: 9, validTo: '2026-09-11'}])
        expect(app.classId).toBe(10)
        expect(app.mode).toBe('homeroom')
    })

    it('그 모드에 남은 것이 없으면 학급 없이 남는다 — 교과로 넘기지 않는다', async () => {
        // 넘겨 버리면 교사는 담임 학급을 마감했을 뿐인데 다른 모드의 화면을 보게 된다.
        const app = scope({classes: [HOMEROOM, SUBJECT]})
        mockInvoke({classes: [SUBJECT]})
        const school = useSchoolStore()

        await school.retireClass(9)

        expect(app.classId).toBe(null)
        expect(app.mode).toBe('homeroom')
        expect(app.hasClassIn('homeroom')).toBe(false)
    })
})

describe('강좌 묶음', () => {
    it('묶음은 학교에 만든다', async () => {
        scope()
        mockInvoke()
        const school = useSchoolStore()

        await school.createClassTag('프로그래밍')

        expect(argsOf('create_class_tag')).toEqual([{schoolId: 1, name: '프로그래밍'}])
    })

    it('이름은 UPDATE로 고친다 — 세는 대상이 아니라 화면의 이름표다', async () => {
        scope()
        mockInvoke()
        const school = useSchoolStore()

        await school.renameClassTag(3, '프로그래밍 기초')

        expect(argsOf('rename_class_tag')).toEqual([{id: 3, name: '프로그래밍 기초'}])
    })

    it('묶음을 지우면 강좌는 남고 이름표만 풀린다 — 목록을 다시 읽는다', async () => {
        scope()
        mockInvoke()
        const school = useSchoolStore()
        invoke.mockClear()
        mockInvoke()

        await school.deleteClassTag(3)

        expect(argsOf('delete_class_tag')).toEqual([{id: 3}])
        // 강좌에 붙어 있던 이름표가 풀렸으므로 담당 학급 · 강좌 목록도 낡았다.
        expect(argsOf('get_teaching_classes')).toEqual([{schoolId: 1}])
    })
})
