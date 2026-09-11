/**
 * 앱 스토어 — 범위와 부팅.
 *
 * 범위는 **학년도 → 학교 → 담당 학급 · 강좌**다. 학급을 선택하면 학교가 역산되던 때에는
 * 그 학년도에 아직 담당 학급 · 강좌가 없는 학교를 가리킬 수 없었다 — 순회 교사가 B학교를
 * 추가하고 강좌를 아직 안 넣은 하루가 그 상태다.
 *
 * 이 파일이 지키는 결정이 셋 더 있다.
 *   · **모드와 학급이 어긋나지 않는다.** 담임 화면에 교과 강좌가 걸리면 DB 트리거가
 *     거절할 때까지 아무도 모른다.
 *   · **담당 학급 · 강좌가 없는 모드가 정상 상태다.** 비담임 교사가 담임 모드로 들어오는 것은
 *     고장이 아니다. 말없이 교과로 돌리지 않고, Welcome으로 튕기지도 않는다.
 *   · **첫 실행은 한 번뿐이다.** 담당 학급 · 강좌를 한 번이라도 만들었으면, 지금 0개여도
 *     시작하지 않은 것이 아니다 — 새 학년도로 옮긴 날과 지난해 학급을 전부 마감한
 *     날이 그 상태다. 부팅 실패도 그 모습을 하고 오므로 따로 구별한다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {academicYearOf, yearSpanOf} from '../services/academicYear'
import {useAppStore} from './app'
import {welcomeRedirect} from '../router'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))

const YEAR = {id: 2, year: 2026, startsOn: '2026-03-02', endsOn: '2027-02-28'}
const NEXT_YEAR = {id: 3, year: 2027, startsOn: '2027-03-02', endsOn: '2028-02-29'}

const A = {
    id: 1, yearId: 2, name: '한빛고등학교', maxSlot: 6, dueDays: 7,
    dueSkipOffdays: true, sortOrder: 10, active: true,
}
const B = {
    id: 2, yearId: 2, name: '푸른중학교', maxSlot: 7, dueDays: 5,
    dueSkipOffdays: true, sortOrder: 20, active: true,
}

const HOMEROOM = {
    id: 9, schoolId: 1, role: 'homeroom', name: '3학년 6반', grade: 3, classNo: 6,
    groupTagId: null, groupTagName: null, sortOrder: 10,
    validFrom: '2026-03-02', validTo: null, memberCount: 28,
}
const SUBJECT = {
    id: 21, schoolId: 1, role: 'subject', name: '지구과학Ⅰ', grade: null, classNo: null,
    groupTagId: null, groupTagName: null, sortOrder: 20,
    validFrom: '2026-03-02', validTo: null, memberCount: 24,
}
/** 다른 학교의 강좌. 순회 교사는 B학교에서 교과만 맡는다. */
const SUBJECT_B = {
    ...SUBJECT, id: 31, schoolId: 2, name: '인공지능기초 A반', memberCount: 19,
}

/** 부팅이 부르는 커맨드를 한꺼번에 흉내 낸다. 학교마다 담당 학급 · 강좌를 따로 묻는다. */
function mockBoot({created = false, config = {}, schools = [A], classes = [HOMEROOM], years = [YEAR]} = {}) {
    // 만든 학년도가 목록에 실제로 들어가야 첫 실행 경로가 그대로 재현된다.
    const rows = [...years]
    invoke.mockImplementation(async (command, args) => {
        switch (command) {
            case 'init_db':
                return {path: '어딘가/rollcall.db', created, dbVersion: 1, appVersion: 1, needsMigration: false}
            case 'get_years':
                return rows
            case 'create_year': {
                const id = 90 + rows.length
                rows.push({id, year: args.year, startsOn: args.startsOn, endsOn: args.endsOn})
                return id
            }
            case 'get_schools':
                return schools.filter((s) => s.yearId === args.yearId)
            case 'get_teaching_classes':
                return classes.filter((c) => c.schoolId === args.schoolId)
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

/** 담임 · 교과를 함께 맡고 둘 다 본 적이 있는 상태. */
const BOTH = {
    classes: [HOMEROOM, SUBJECT],
    config: {yearId: '2', schoolId: '1', mode: 'homeroom', homeroomClassId: '9', subjectClassId: '21'},
}

/** 그 커맨드에 넘어간 인자들. 어느 값이 빠졌는지까지 본다. */
function argsOf(command) {
    return invoke.mock.calls.filter((c) => c[0] === command).map((c) => c[1])
}

/** app_config에 적힌 것. 열쇠 다섯이 한 번에 나가는지 본다. */
function savedConfig() {
    return Object.fromEntries(argsOf('set_config').map((a) => [a.key, a.value]))
}

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
})

describe('부팅', () => {
    it('첫 실행 여부를 버리지 않는다 — init_db가 알려준 것을 들고 있는다', async () => {
        mockBoot({created: true, schools: [], classes: []})
        const app = useAppStore()

        await app.boot()

        expect(app.firstRun).toBe(true)
        expect(app.booted).toBe(true)
    })

    it('있던 파일을 연 것은 첫 실행이 아니다', async () => {
        mockBoot({created: false, schools: [], classes: []})
        const app = useAppStore()

        await app.boot()

        expect(app.firstRun).toBe(false)
    })

    it('학교는 학년도로, 담당 학급 · 강좌는 학교로 읽는다 — 계층이 그대로 질의가 된다', async () => {
        mockBoot({...BOTH, schools: [A, B], classes: [HOMEROOM, SUBJECT, SUBJECT_B]})
        const app = useAppStore()

        await app.boot()

        expect(argsOf('get_schools')).toEqual([{yearId: 2}])
        // 학교마다 한 번씩 묻는다. 한 학교만 읽으면 "이 학교에 아직 담당 학급 · 강좌가 없다"와
        // "앱을 처음 켰다"가 같은 모양으로 보인다.
        expect(argsOf('get_teaching_classes')).toEqual([{schoolId: 1}, {schoolId: 2}])
        expect(app.classes).toHaveLength(3)
    })

    it('설정에 적힌 학교를 복원한다 — 최대 교시가 그 학교의 것이다', async () => {
        mockBoot({
            schools: [A, B], classes: [HOMEROOM, SUBJECT_B],
            config: {yearId: '2', schoolId: '2', mode: 'subject', subjectClassId: '31'},
        })
        const app = useAppStore()

        await app.boot()

        expect(app.schoolId).toBe(2)
        expect(app.school.name).toBe('푸른중학교')
        expect(app.maxSlot).toBe(7)
        expect(app.classId).toBe(31)
    })

    it('목록에 없는 학교 번호는 첫 학교로 되돌린다 — 지난해 번호가 설정에 남는다', async () => {
        mockBoot({...BOTH, config: {...BOTH.config, schoolId: '99'}})
        const app = useAppStore()

        await app.boot()

        expect(app.schoolId).toBe(1)
        expect(app.maxSlot).toBe(6)
    })

    it('마지막에 본 학급을 복원한다 — 매일 아침 다시 선택하지 않게 한다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()

        await app.boot()

        expect(app.mode).toBe('homeroom')
        expect(app.classId).toBe(9)
        expect(app.currentClass.name).toBe('3학년 6반')
        expect(app.ready).toBe(true)
    })

    it('목록에 없는 학급 번호는 버린다 — 지난해 학급으로 물으면 전부 빈다', async () => {
        mockBoot({...BOTH, config: {...BOTH.config, homeroomClassId: '4'}})
        const app = useAppStore()

        await app.boot()

        expect(app.classId).toBe(9)
    })

    it('실패는 error에 담고 다시 던진다 — 삼키면 빈 화면과 구별되지 않는다', async () => {
        invoke.mockRejectedValue('데이터 파일이 열려 있지 않습니다.')
        const app = useAppStore()

        await expect(app.boot()).rejects.toBeTruthy()

        expect(app.error).toContain('데이터 파일이 열려 있지 않습니다.')
        expect(app.booted).toBe(true)
    })

    it('부팅 전에는 판단하지 않는다 — 아직 아무것도 모른다', () => {
        const app = useAppStore()
        expect(app.booted).toBe(false)
        expect(app.needsWelcome).toBe(false)
    })
})

describe('학년도 → 학교 → 담당 학급 · 강좌', () => {
    it('담당 학급 · 강좌가 아직 없는 학교도 가리킬 수 있다 — 등록한 날의 정상 상태다', async () => {
        mockBoot({
            schools: [A, B], classes: [HOMEROOM, SUBJECT],
            config: {yearId: '2', schoolId: '2', mode: 'subject'},
        })
        const app = useAppStore()

        await app.boot()

        expect(app.schoolId).toBe(2)
        expect(app.schoolClasses).toEqual([])
        expect(app.classId).toBe(null)
        // 담당 학급 · 강좌가 다른 학교에 있으므로 아직 시작하지 않은 상태가 아니다.
        expect(app.needsWelcome).toBe(false)
    })

    it('역할은 학교가 결정한다 — A학교는 담임 + 교과, B학교는 교과만', async () => {
        mockBoot({
            schools: [A, B], classes: [HOMEROOM, SUBJECT, SUBJECT_B],
            config: {yearId: '2', schoolId: '1', mode: 'homeroom', homeroomClassId: '9'},
        })
        const app = useAppStore()
        await app.boot()

        expect(app.hasClassIn('homeroom')).toBe(true)
        expect(app.hasClassIn('subject')).toBe(true)

        await app.selectSchool(2)

        expect(app.hasClassIn('homeroom')).toBe(false)
        expect(app.hasClassIn('subject')).toBe(true)
        expect(app.subjectClasses.map((c) => c.id)).toEqual([31])
    })

    it('학교를 옮기면 담당 학급 · 강좌도 그 학교 것으로 다시 선택한다', async () => {
        mockBoot({
            schools: [A, B], classes: [HOMEROOM, SUBJECT, SUBJECT_B],
            config: {yearId: '2', schoolId: '1', mode: 'subject', subjectClassId: '21'},
        })
        const app = useAppStore()
        await app.boot()
        expect(app.classId).toBe(21)

        await app.selectSchool(2)

        // A학교의 강좌가 남아 있으면 B학교 화면에 A학교 명단이 그려진다.
        expect(app.classId).toBe(31)
        expect(app.mode).toBe('subject')
        expect(savedConfig().schoolId).toBe('2')
    })

    it('등록된 학교가 아니면 선택할 수 없다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        await expect(app.selectSchool(404)).rejects.toBeTruthy()
        expect(app.error).toContain('없는 학교입니다')
        expect(app.schoolId).toBe(1)
    })

    it('학년도를 바꾸면 학교부터 다시 선택한다 — 2026=A학교, 2027=B·C학교', async () => {
        const NEXT = {...B, id: 5, yearId: 3, name: '새빛고등학교'}
        mockBoot({...BOTH, schools: [A, NEXT], years: [YEAR, NEXT_YEAR]})
        const app = useAppStore()
        await app.boot()
        expect(app.schoolId).toBe(1)

        await app.selectYear(3)

        expect(app.yearId).toBe(3)
        expect(app.schools.map((s) => s.id)).toEqual([5])
        expect(app.schoolId).toBe(5)
        // 그 학년도에 담당 학급 · 강좌가 아직 없다 — 학교 설정부터 처음부터 하는 자리다.
        expect(app.classes).toEqual([])
        // **추가할 차례이지 첫 실행이 아니다.** 마법사는 사이드바가 없는 전체 화면이라,
        // 3월에 학년도를 옮긴 교사가 거기로 튕기면 설정으로 돌아갈 길이 막힌다.
        expect(app.needsWelcome).toBe(false)
        expect(savedConfig().yearId).toBe('3')
    })

    it('첫 실행에 학년도를 채운다 — 오늘이 언제인지 DB는 모른다', async () => {
        mockBoot({created: true, years: [], schools: [], classes: []})
        const app = useAppStore()

        await app.boot()

        const [made] = argsOf('create_year')
        expect(made.year).toBe(academicYearOf(app.today))
        expect(made.startsOn).toBe(yearSpanOf(made.year).startsOn)
        expect(app.yearId).toBe(90)
        // 부팅은 설정을 쓰지 않는다. 아직 학교도 담당 학급 · 강좌도 읽기 전이라,
        // 여기서 적으면 교사가 선택한 적 없는 자리가 설정에 남는다.
        expect(argsOf('set_config')).toEqual([])
    })

    it('학년도를 만들면 그리로 옮긴다 — 만들어 두고 쓰지 않을 이유가 없다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        await app.createYear(2027)

        expect(argsOf('create_year')[0].year).toBe(2027)
        expect(app.yearId).toBe(91)
        // 2027학년도에는 아직 학교가 없다. 학교 설정부터 처음부터 하는 자리다.
        expect(app.schools).toEqual([])
        expect(app.needsWelcome).toBe(false)
        expect(savedConfig().schoolId).toBe('')
    })

    it('이미 있는 해는 만들지 않고 그리로 옮긴다 — UNIQUE가 거절한다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()
        invoke.mockClear()

        const id = await app.createYear(2026)

        expect(id).toBe(2)
        expect(invoke.mock.calls.map((c) => c[0])).not.toContain('create_year')
    })

    it('등록된 학년도가 아니면 선택할 수 없다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        await expect(app.selectYear(99)).rejects.toBeTruthy()
        expect(app.error).toContain('없는 학년도입니다')
        expect(app.yearId).toBe(2)
    })

    it('맡지 않은 학급은 선택할 수 없다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        await expect(app.selectClass(999)).rejects.toBeTruthy()
        expect(app.error).toContain('담당 학급 · 강좌가 아닙니다')
        expect(app.classId).toBe(9)
    })
})

/**
 * 학년도 지우기 — **그 아래가 통째로 사라진다.**
 *
 * 되돌릴 수 없는 동작이라 화면이 묻고 지운다. 스토어가 지키는 것은 지운 **뒤**다 —
 * 보고 있던 학년도가 사라졌는데 그 번호를 그대로 들고 있으면, 화면마다 빈 결과가
 * 돌아오는데 이유는 어디에도 표시되지 않는다. 다시 켜면 회복되지만 그때까지
 * 교사는 자기 기록이 통째로 사라졌다고 읽는다.
 */
describe('학년도 지우기', () => {
    /** 2027학년도와 그 아래 학교 · 담임 학급. 목록은 최신 학년도가 먼저다. */
    const NEXT_SCHOOL = {...B, id: 5, yearId: 3, name: '새빛고등학교'}
    const NEXT_CLASS = {...HOMEROOM, id: 41, schoolId: 5, name: '2학년 1반', grade: 2, classNo: 1}
    const TWO_YEARS = {
        years: [NEXT_YEAR, YEAR], schools: [A, NEXT_SCHOOL], classes: [HOMEROOM, NEXT_CLASS],
        config: {yearId: '2', schoolId: '1', mode: 'homeroom', homeroomClassId: '9'},
    }

    it('보고 있던 학년도를 지우면 남은 학년도로 옮긴다 — 사라진 번호로 질의하지 않는다', async () => {
        mockBoot(TWO_YEARS)
        const app = useAppStore()
        await app.boot()
        expect(app.yearId).toBe(2)

        // 지운 뒤의 목록. 2026학년도와 그 아래 학교 · 학급이 함께 사라진다.
        mockBoot({years: [NEXT_YEAR], schools: [NEXT_SCHOOL], classes: [NEXT_CLASS]})
        invoke.mockClear()

        await app.deleteYear(2)

        expect(argsOf('delete_year')).toEqual([{yearId: 2}])
        expect(app.yearId).toBe(3)
        // 학교와 담당 학급 · 강좌를 남은 학년도 것으로 다시 읽는다. 사라진 학년도의
        // 번호로 물으면 전부 비는데, 화면에는 "기록이 없다"로 보인다.
        expect(argsOf('get_schools')).toEqual([{yearId: 3}])
        expect(app.schoolId).toBe(5)
        expect(app.classId).toBe(41)
        expect(app.maxSlot).toBe(7)
        // Rust가 설정의 범위 열쇠를 비웠으므로 옮긴 자리를 여기서 다시 적는다.
        expect(savedConfig().yearId).toBe('3')
        expect(savedConfig().schoolId).toBe('5')
    })

    it('남은 학년도에 학교가 없으면 범위를 비운 채로 연다 — 추가할 차례이지 첫 실행이 아니다', async () => {
        mockBoot(TWO_YEARS)
        const app = useAppStore()
        await app.boot()

        mockBoot({years: [NEXT_YEAR], schools: [], classes: []})
        await app.deleteYear(2)

        expect(app.yearId).toBe(3)
        expect(app.schools).toEqual([])
        expect(app.classes).toEqual([])
        expect(app.schoolId).toBe(null)
        expect(app.classId).toBe(null)
        expect(app.lastClassId).toEqual({homeroom: null, subject: null})
        // 마법사는 사이드바가 없는 전체 화면이다. 지운 교사가 거기로 튕기면
        // 설정으로 돌아갈 길이 막힌다.
        expect(app.needsWelcome).toBe(false)
    })

    it('다른 학년도를 지우면 보고 있던 자리는 그대로다', async () => {
        mockBoot(TWO_YEARS)
        const app = useAppStore()
        await app.boot()

        mockBoot({years: [YEAR], schools: [A], classes: [HOMEROOM]})
        invoke.mockClear()

        await app.deleteYear(3)

        expect(argsOf('delete_year')).toEqual([{yearId: 3}])
        expect(app.years.map((y) => y.id)).toEqual([2])
        expect(app.yearId).toBe(2)
        expect(app.schoolId).toBe(1)
        expect(app.classId).toBe(9)
        // 옮기지 않았으므로 설정에 다시 쓸 것이 없다. 보던 화면이 이유 없이
        // 처음으로 돌아가면 교사는 무엇을 지운 것인지 알 수 없다.
        expect(argsOf('set_config')).toEqual([])
    })

    it('마지막 학년도는 거절당한다 — 문구를 그대로 담고 다시 던진다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()
        invoke.mockRejectedValueOnce(
            '마지막 학년도는 지울 수 없습니다. 학년도가 없으면 학교도 담당 학급 · 강좌도 ' +
            '만들 수 없습니다. 새 학년도를 먼저 만든 뒤에 지우세요.',
        )

        await expect(app.deleteYear(2)).rejects.toBeTruthy()

        expect(app.error).toContain('마지막 학년도는 지울 수 없습니다')
        // 거절당했으면 보던 자리도 그대로다. 여기서 비우면 지워지지도 않은 학년도의
        // 화면이 통째로 빈다.
        expect(app.years.map((y) => y.id)).toEqual([2])
        expect(app.yearId).toBe(2)
        expect(app.classId).toBe(9)
    })

    it('목록이 비어 와도 사라진 학년도를 가리킨 채로 두지 않는다', async () => {
        // Rust가 마지막 하나를 거절하므로 닿지 않는 자리다. 그래도 없는 번호를 들고
        // 있으면 화면마다 빈 결과가 돌아오고, 그 이유는 어디에도 표시되지 않는다.
        mockBoot({config: {yearId: '2', schoolId: '1', homeroomClassId: '9'}})
        const app = useAppStore()
        await app.boot()

        mockBoot({years: [], schools: [], classes: []})
        await app.deleteYear(2)

        expect(app.yearId).toBe(null)
        expect(app.schools).toEqual([])
        expect(app.classes).toEqual([])
        expect(app.classId).toBe(null)
    })
})

describe('담당 학급 · 강좌가 없는 모드', () => {
    it('비담임 교사가 담임 모드로 열려도 말없이 교과로 돌리지 않는다', async () => {
        // 돌려 버리면 교사는 자기가 무엇을 눌렀는지 모른 채 다른 화면을 본다.
        // 담임 학급이 없는 담임 모드는 정상 상태이고, 화면이 그것을 말한다.
        mockBoot({config: {mode: 'homeroom', subjectClassId: '21'}, classes: [SUBJECT]})
        const app = useAppStore()

        await app.boot()

        expect(app.mode).toBe('homeroom')
        expect(app.classId).toBe(null)
        expect(app.hasClassIn('homeroom')).toBe(false)
        expect(app.hasClassIn('subject')).toBe(true)
    })

    it('담당 학급 · 강좌가 있으면 Welcome으로 튕기지 않는다 — 선택한 학급이 없어도', async () => {
        mockBoot({config: {mode: 'homeroom'}, classes: [SUBJECT]})
        const app = useAppStore()

        await app.boot()

        expect(app.ready).toBe(false)
        expect(app.needsWelcome).toBe(false)
        expect(welcomeRedirect(app, {name: 'overview'})).toBe(true)
    })

    it('비어 있는 모드로도 넘어간다 — 스위치는 늘 보인다', async () => {
        mockBoot({config: {mode: 'subject', subjectClassId: '21'}, classes: [SUBJECT]})
        const app = useAppStore()
        await app.boot()

        await app.setMode('homeroom')

        expect(app.mode).toBe('homeroom')
        expect(app.classId).toBe(null)
        expect(savedConfig().mode).toBe('homeroom')
    })
})

describe('모드와 학급', () => {
    it('학급을 선택하면 모드가 그 학급의 역할로 따라온다 — 어긋나지 않는다', async () => {
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
    it('모드는 담아 두는 값이 아니라 선택한 학급이 말하는 값이다', async () => {
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
        // 오가며 쓰는 값이라 한쪽에서 선택한 것이 다른 쪽을 덮으면 매번 다시 선택해야 한다.
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

    it('다른 학교의 강좌를 선택하면 학교도 함께 옮긴다', async () => {
        // 학교가 뒤에 남으면 그 학교의 최대 교시로 다른 학교의 출결을 입력하게 된다.
        mockBoot({
            schools: [A, B], classes: [HOMEROOM, SUBJECT, SUBJECT_B],
            config: {yearId: '2', schoolId: '1', mode: 'homeroom', homeroomClassId: '9'},
        })
        const app = useAppStore()
        await app.boot()

        await app.selectClass(31)

        expect(app.schoolId).toBe(2)
        expect(app.maxSlot).toBe(7)
        expect(app.classId).toBe(31)
        // B학교에는 담임 학급이 없다. A학교의 담임 기억이 남으면 스위치를 눌렀을 때
        // 남의 학교 명단이 열린다.
        expect(app.lastClassId.homeroom).toBe(null)
    })

    it('범위 다섯을 한 번에 적는다 — 하나라도 빠지면 다음 실행에서 엉뚱한 자리가 열린다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()
        invoke.mockClear()

        await app.selectClass(21)

        const saved = savedConfig()
        expect(Object.keys(saved).sort())
            .toEqual(['homeroomClassId', 'mode', 'schoolId', 'subjectClassId', 'yearId'])
        expect(saved.mode).toBe('subject')
        expect(saved.subjectClassId).toBe('21')
        // 모드마다 마지막 자리를 따로 적는다. 교과로 옮겼다고 담임 자리가 덮이면
        // 돌아올 때마다 다시 선택해야 한다.
        expect(saved.homeroomClassId).toBe('9')
        expect(saved.schoolId).toBe('1')
    })
})

describe('담당 학급 · 강좌 만들기', () => {
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

    it('학년도가 아니라 학교에 만든다 — 묶음 이름표도 함께 넘긴다', async () => {
        mockBoot({classes: []})
        const app = useAppStore()
        await app.boot()
        invoke.mockClear()
        mockBoot({classes: [{...SUBJECT, id: 77, name: '프로그래밍A', groupTagId: 3}]})

        await app.createClass({role: 'subject', name: '프로그래밍A', groupTagId: 3})

        expect(argsOf('create_teaching_class')[0]).toEqual({
            schoolId: 1, role: 'subject', name: '프로그래밍A',
            grade: null, classNo: null, groupTagId: 3, validFrom: '2026-03-02',
        })
    })

    it('같은 학급을 두 번 만들지 않는다 — [저장]을 두 번 누르는 것은 흔한 일이다', async () => {
        mockBoot({config: {schoolId: '1', mode: 'homeroom', homeroomClassId: '9'}})
        const app = useAppStore()
        await app.boot()
        invoke.mockClear()

        const id = await app.createClass({role: 'homeroom', name: '3학년 6반', grade: 3, classNo: 6})

        expect(id).toBe(9)
        // 늘어나면 명단이 어느 쪽에 붙었는지 알 수 없게 된다.
        expect(invoke.mock.calls.map((c) => c[0])).not.toContain('create_teaching_class')
    })

    it('다른 학교의 같은 이름은 가로채지 않는다', async () => {
        mockBoot({schools: [A, B], classes: [SUBJECT]})
        const app = useAppStore()
        await app.boot()
        invoke.mockClear()
        mockBoot({schools: [A, B], classes: [SUBJECT, {...SUBJECT, id: 77, schoolId: 2}]})

        await app.createClass({schoolId: 2, role: 'subject', name: '지구과학Ⅰ', select: false})

        expect(argsOf('create_teaching_class')[0]?.schoolId).toBe(2)
    })

    it('학교가 없으면 거절한다 — 담당 학급 · 강좌는 학교 안에 있다', async () => {
        mockBoot({schools: [], classes: []})
        const app = useAppStore()
        await app.boot()

        await expect(app.createClass({role: 'homeroom', name: '3학년 6반', grade: 3, classNo: 6}))
            .rejects.toBeTruthy()
        expect(app.error).toContain('학교를 먼저 추가해야')
    })
})

describe('기억해 둔 학급', () => {
    it('선택하지 않고 만든 첫 강좌도 그 모드의 자리에 기억한다', async () => {
        // 설정에서 더한 강좌는 화면을 통째로 바꾸지 않는다(`select: false`). 그때 기억까지
        // 비워 두면 교과 스위치를 눌러도 학급이 없어, 사이드바는 열려 있는데 화면은
        // "아직 교과 강좌를 추가하지 않았습니다"라고 잘못 말한다.
        mockBoot({config: {schoolId: '1', mode: 'homeroom', homeroomClassId: '9'}, classes: [HOMEROOM]})
        const app = useAppStore()
        await app.boot()
        expect(app.lastClassId.subject).toBe(null)

        const 새강좌 = {...SUBJECT, id: 55}
        mockBoot({classes: [새강좌, HOMEROOM]})
        await app.createClass({role: 'subject', name: '지구과학Ⅰ', select: false})

        // 선택하는 것과 기억하는 것은 다른 일이다. 화면은 담임 그대로 남는다.
        expect(app.classId).toBe(9)
        expect(app.mode).toBe('homeroom')
        expect(app.lastClassId.subject).toBe(55)

        await app.setMode('subject')
        expect(app.classId).toBe(55)
        expect(app.ready).toBe(true)
    })

    it('보고 있는 모드의 첫 강좌를 만들면 그 자리에서 열린다 — ready와 사이드바가 어긋나지 않는다', async () => {
        // 교과 모드로 들어와 강좌를 처음 더한 자리다. 기억만 채우고 보고 있는 학급을
        // 비워 두면 `hasClassIn`은 참인데 `ready`가 거짓이라, 앱을 껐다 켜야 회복한다.
        mockBoot({config: {schoolId: '1', mode: 'subject'}, classes: [HOMEROOM]})
        const app = useAppStore()
        await app.boot()
        expect(app.ready).toBe(false)

        const 새강좌 = {...SUBJECT, id: 55}
        mockBoot({classes: [새강좌, HOMEROOM]})
        await app.createClass({role: 'subject', name: '지구과학Ⅰ', select: false})

        expect(app.hasClassIn('subject')).toBe(true)
        expect(app.ready).toBe(true)
        expect(app.classId).toBe(55)
        expect(app.mode).toBe('subject')
    })

    it('빈 자리는 목록을 다시 읽을 때 채운다 — 사라진 번호만 지우지 않는다', async () => {
        mockBoot({config: {schoolId: '1', mode: 'homeroom', homeroomClassId: '9'}, classes: [HOMEROOM]})
        const app = useAppStore()
        await app.boot()

        mockBoot({classes: [HOMEROOM, SUBJECT]})
        await app.fetchClasses()

        expect(app.lastClassId).toEqual({homeroom: 9, subject: 21})
    })

    it('그 모드에 담당 학급 · 강좌가 없으면 비운 채로 둔다 — 남의 역할을 채우지 않는다', async () => {
        mockBoot({config: {schoolId: '1', mode: 'subject', subjectClassId: '21'}, classes: [SUBJECT]})
        const app = useAppStore()

        await app.boot()

        expect(app.lastClassId).toEqual({homeroom: null, subject: 21})
        expect(app.classId).toBe(21)
    })
})

describe('첫 실행 흐름', () => {
    it('담당 학급 · 강좌가 하나도 없으면 Welcome으로 보낸다 — 학교만 있는 것은 시작이 아니다', async () => {
        mockBoot({created: true, classes: []})
        const app = useAppStore()
        await app.boot()

        expect(app.schools).toHaveLength(1)
        expect(app.needsWelcome).toBe(true)
        expect(welcomeRedirect(app, {name: 'today'})).toEqual({name: 'welcome'})
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

    it('학급을 만들면 더는 붙잡지 않는다 — [시작하기]가 개요로 넘어간다', async () => {
        mockBoot({created: true, classes: []})
        const app = useAppStore()
        await app.boot()

        mockBoot({classes: [HOMEROOM]})
        await app.createClass({role: 'homeroom', name: '3학년 6반', grade: 3, classNo: 6})

        expect(app.needsWelcome).toBe(false)
        expect(welcomeRedirect(app, {name: 'overview'})).toBe(true)
    })

    it('시작한 것을 설정에 적어 둔다 — 담당 학급 · 강좌의 수로는 판단할 수 없다', async () => {
        mockBoot({created: true, classes: []})
        const app = useAppStore()
        await app.boot()
        mockBoot({classes: [HOMEROOM]})
        invoke.mockClear()

        await app.createClass({role: 'homeroom', name: '3학년 6반', grade: 3, classNo: 6})

        expect(argsOf('set_config').filter((a) => a.key === 'onboarded'))
            .toEqual([{key: 'onboarded', value: '1'}])
    })

    it('앱을 껐다 켜도 시작한 것을 기억한다 — 담당 학급 · 강좌가 0개여도 마법사가 아니다', async () => {
        // 3월에 지난해 학급을 전부 마감하고 아직 새로 등록하지 않은 아침이 이 상태다.
        mockBoot({config: {onboarded: '1'}, classes: []})
        const app = useAppStore()

        await app.boot()

        expect(app.classes).toEqual([])
        expect(app.needsWelcome).toBe(false)
        expect(welcomeRedirect(app, {name: 'overview'})).toBe(true)
    })

    it('열쇠가 없던 설정은 부팅이 채운다 — 담당 학급 · 강좌가 있으면 이미 시작한 것이다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()

        await app.boot()

        expect(app.onboarded).toBe(true)
        expect(argsOf('set_config')).toEqual([{key: 'onboarded', value: '1'}])

        // 이미 적혀 있으면 다시 쓰지 않는다. 부팅마다 같은 값을 쓸 이유가 없다.
        await app.markStarted()
        expect(argsOf('set_config')).toHaveLength(1)
    })

    it('담당 학급 · 강좌를 전부 마감해도 마법사에 갇히지 않는다', async () => {
        mockBoot(BOTH)
        const app = useAppStore()
        await app.boot()

        mockBoot({classes: []})
        await app.fetchClasses()

        expect(app.classes).toEqual([])
        expect(app.ready).toBe(false)
        expect(app.needsWelcome).toBe(false)
        expect(welcomeRedirect(app, {name: 'overview'})).toBe(true)
    })

    it('부팅이 실패하면 마법사로 보내지 않는다 — 읽지 못한 것과 아직 없는 것은 다르다', async () => {
        // 보내 버리면 교사는 이미 넣은 학교와 명렬표가 사라졌다고 읽는다.
        invoke.mockRejectedValue('데이터 파일이 열려 있지 않습니다.')
        const app = useAppStore()

        await expect(app.boot()).rejects.toBeTruthy()

        expect(app.bootFailed).toBe(true)
        expect(app.needsWelcome).toBe(false)
        expect(welcomeRedirect(app, {name: 'overview'})).toBe(true)
    })

    it('부팅 실패는 뒤이은 액션이 지우지 못한다 — error 한 칸으로는 남지 않는다', async () => {
        invoke.mockRejectedValue('데이터 파일이 열려 있지 않습니다.')
        const app = useAppStore()
        await expect(app.boot()).rejects.toBeTruthy()

        // 화면이 목록을 한 번 다시 읽는 것만으로 error가 비워진다.
        await app.fetchSchools()

        expect(app.error).toBe('')
        expect(app.bootFailed).toBe(true)
        expect(app.needsWelcome).toBe(false)
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
