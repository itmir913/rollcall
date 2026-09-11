/**
 * 교과 차시 스토어.
 *
 * 여기서 지키는 것은 넷이다.
 *   1. **담임 모드에서는 아무것도 읽지 않는다.** 한쪽의 숫자가 다른 쪽에 유출되면 안 된다.
 *   2. 교시를 여럿 선택해도 칸은 교시마다 하나씩 만들어진다(연강도 두 차시다).
 *   3. 입력은 같은 학생을 다시 누르면 취소이고, 돌려주는 값이 입력한 뒤의 상태다.
 *   4. 실패를 삼키지 않는다 — `error`에 담고 다시 던진다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useSubjectStore} from './subject'
import {useAppStore} from './app'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))

const SESSION = (over = {}) => ({
    id: 1, date: '2026-09-11', slot: '1', memo: '', absentCount: 0, total: 28, ...over,
})

const ROLL = (over = {}) => ({
    studentId: 11, grade: 3, classNo: 6, number: 5, name: '김하늘',
    absent: false, memo: '', homeroomNote: null, ...over,
})

/** 교과 강좌 하나를 맡은 교사. 모드는 선택한 학급의 역할이 결정한다. */
function subjectTeacher() {
    const app = useAppStore()
    app.schools = [{id: 1, yearId: 2, name: '한빛고등학교', maxSlot: 7}]
    app.yearId = 2
    app.today = '2026-09-11'
    app.classes = [{
        id: 20, schoolId: 1, role: 'subject', name: '인공지능기초 A반',
        grade: null, classNo: null, validTo: null,
    }]
    app.classId = 20
    return app
}

/** 커맨드마다 다른 응답이 필요하다. 이름으로 구별한다. */
function answers({sessions = [], roll = [], created = 77, toggled = true} = {}) {
    invoke.mockImplementation(async (command) => {
        if (command === 'get_subject_sessions') return sessions
        if (command === 'get_session_roll') return roll
        if (command === 'create_subject_session') return created
        if (command === 'toggle_subject_absence') return toggled
        return null
    })
}

beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
    invoke.mockResolvedValue([])
})

describe('읽기', () => {
    it('강좌와 기간으로 묻는다', async () => {
        subjectTeacher()
        answers({sessions: [SESSION()]})

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        await subject.fetchDay()

        expect(invoke).toHaveBeenCalledWith('get_subject_sessions', {
            classId: 20, from: '2026-09-11', to: '2026-09-11',
        })
        expect(subject.daySessions).toHaveLength(1)
    })

    it('담임 모드에서는 읽지 않고 비운다 — 담임 화면에 교과 숫자가 새면 안 된다', async () => {
        const app = subjectTeacher()
        app.classes = [{
            id: 10, schoolId: 1, role: 'homeroom', name: '3학년 6반',
            grade: 3, classNo: 6, validTo: null,
        }]
        app.classId = 10

        const subject = useSubjectStore()
        subject.sessions = [SESSION()]
        subject.roll = [ROLL({absent: true})]
        subject.sessionId = 1
        subject.setDate('2026-09-11')
        await subject.fetchDay()

        expect(invoke).not.toHaveBeenCalled()
        expect(subject.sessions).toEqual([])
        expect(subject.roll).toEqual([])
        expect(subject.sessionId).toBeNull()
    })

    it('강좌를 옮기면 앞 강좌의 명단을 들고 있지 않는다', async () => {
        const app = subjectTeacher()
        app.classes = [...app.classes, {
            id: 21, schoolId: 1, role: 'subject', name: '인공지능기초 B반',
            grade: null, classNo: null, validTo: null,
        }]
        answers({sessions: [SESSION()]})

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        await subject.fetchDay()
        await subject.select(1)
        expect(subject.sessionId).toBe(1)

        app.classId = 21
        await subject.fetchDay()

        expect(subject.sessionId).toBeNull()
        expect(subject.roll).toEqual([])
    })

    it('달은 말일까지 읽는다 — 30일까지만 읽으면 31일 수업이 사라진다', async () => {
        subjectTeacher()
        answers()

        const subject = useSubjectStore()
        subject.setMonth(1, 2026)
        await subject.fetchMonth()

        // 2026학년도 1월은 2027년이다.
        expect(invoke).toHaveBeenCalledWith('get_subject_sessions', {
            classId: 20, from: '2027-01-01', to: '2027-01-31',
        })
    })
})

describe('날짜 옮기기', () => {
    /** 9월 11일 차시를 열어 둔 상태. 여기서 날짜를 옮기는 것이 문제의 자리다. */
    async function openedOn11() {
        subjectTeacher()
        answers({sessions: [SESSION()], roll: [ROLL({absent: true})]})

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        await subject.fetchDay()
        await subject.select(1)
        expect(subject.sessionId).toBe(1)
        expect(subject.roll).toHaveLength(1)
        return subject
    }

    it('하루 옮기면 선택한 차시와 명단을 함께 비운다 — 보이지 않는 날에 입력되면 안 된다', async () => {
        const subject = await openedOn11()

        await subject.move(1)

        expect(subject.date).toBe('2026-09-12')
        expect(subject.sessionId).toBeNull()
        expect(subject.roll).toEqual([])
    })

    it('날짜를 직접 선택해도 앞 날짜의 차시가 남지 않는다', async () => {
        const subject = await openedOn11()

        subject.setDate('2026-09-12')

        expect(subject.sessionId).toBeNull()
        expect(subject.roll).toEqual([])
    })

    it('같은 날짜를 다시 선택해도 열어 둔 차시는 그대로다 — 옮긴 것이 아니다', async () => {
        const subject = await openedOn11()

        subject.setDate('2026-09-11')

        expect(subject.sessionId).toBe(1)
        expect(subject.roll).toHaveLength(1)
    })

    it('옮긴 뒤에는 입력하지 않는다 — 어느 칸에 들어갔는지 모르는 기록을 만들지 않는다', async () => {
        const subject = await openedOn11()
        await subject.move(1)
        invoke.mockClear()

        await expect(subject.toggle(11)).rejects.toBeTruthy()
        await expect(subject.setAbsenceMemo(11, '병원')).rejects.toBeTruthy()

        expect(subject.error).toContain('교시를 선택하세요')
        expect(invoke).not.toHaveBeenCalled()
    })

    it('지난 차시를 열 때는 날짜부터 옮기고 선택한다 — 순서가 뒤바뀌면 방금 선택한 것이 지워진다', async () => {
        subjectTeacher()
        answers({
            sessions: [SESSION({id: 5, date: '2026-09-03', slot: '2'})],
            roll: [ROLL()],
        })

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        subject.setMonth(9, 2026)
        await subject.fetchMonth()

        // 수업 기록 화면이 하는 일이다.
        subject.setDate('2026-09-03')
        await subject.select(5)

        expect(subject.sessionId).toBe(5)
        expect(subject.current.date).toBe('2026-09-03')
        expect(subject.daySessions.map((s) => s.id)).toEqual([5])
    })
})

describe('차시 만들기 · 지우기', () => {
    it('선택한 교시마다 칸을 하나씩 만든다 — 연강도 두 차시다', async () => {
        subjectTeacher()
        let next = 100
        invoke.mockImplementation(async (command) => {
            if (command === 'create_subject_session') return (next += 1)
            if (command === 'get_subject_sessions') {
                return [SESSION({id: 101, slot: '3'}), SESSION({id: 102, slot: '4'})]
            }
            if (command === 'get_session_roll') return [ROLL()]
            return null
        })

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        const ids = await subject.addSessions([3, 4])

        expect(ids).toEqual([101, 102])
        const made = invoke.mock.calls.filter(([command]) => command === 'create_subject_session')
        expect(made.map(([, args]) => args)).toEqual([
            {classId: 20, date: '2026-09-11', slot: '3'},
            {classId: 20, date: '2026-09-11', slot: '4'},
        ])
        // 여럿을 선택해도 여는 것은 하나다. 화면이 스스로 옮겨 다니면 어디를 보고 있었는지 놓친다.
        expect(subject.sessionId).toBe(101)
    })

    it('열어 둔 차시를 지우면 명단도 함께 닫는다', async () => {
        subjectTeacher()
        answers({sessions: [SESSION()], roll: [ROLL()]})

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        await subject.fetchDay()
        await subject.select(1)
        await subject.removeSession(1)

        expect(invoke).toHaveBeenCalledWith('delete_subject_session', {sessionId: 1})
        expect(subject.sessionId).toBeNull()
        expect(subject.roll).toEqual([])
    })
})

describe('입력', () => {
    it('입력한 뒤의 결석 여부를 돌려주고 명단을 다시 읽는다', async () => {
        subjectTeacher()
        answers({sessions: [SESSION()], roll: [ROLL({absent: true})], toggled: true})

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        await subject.fetchDay()
        await subject.select(1)
        invoke.mockClear()

        await expect(subject.toggle(11)).resolves.toBe(true)
        expect(invoke).toHaveBeenCalledWith('toggle_subject_absence', {sessionId: 1, studentId: 11})
        expect(invoke).toHaveBeenCalledWith('get_session_roll', {sessionId: 1})
        expect(subject.absentRows).toHaveLength(1)
    })

    it('메모는 지금 열어 둔 차시에 붙는다', async () => {
        subjectTeacher()
        answers({sessions: [SESSION()], roll: [ROLL({absent: true})]})

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        await subject.fetchDay()
        await subject.select(1)
        await subject.setAbsenceMemo(11, '병원')

        expect(invoke).toHaveBeenCalledWith('set_subject_absence_memo', {
            sessionId: 1, studentId: 11, memo: '병원',
        })
    })
})

describe('묶음과 세기', () => {
    it('날짜별로 묶고 최신이 위에 온다 — 차례는 날짜 · 교시 순으로 센다', async () => {
        subjectTeacher()
        answers({
            sessions: [
                SESSION({id: 1, date: '2026-09-10', slot: '1', absentCount: 2}),
                SESSION({id: 2, date: '2026-09-10', slot: '3', absentCount: 1}),
                SESSION({id: 3, date: '2026-09-11', slot: '2', absentCount: 0}),
            ],
        })

        const subject = useSubjectStore()
        subject.setMonth(9, 2026)
        await subject.fetchMonth()

        expect(subject.days.map((g) => g.date)).toEqual(['2026-09-11', '2026-09-10'])
        expect(subject.days[0].dateLabel).toBe('2026.09.11.(금)')
        // 세는 차례는 Rust가 준 날짜 · 교시 순 그대로다. 저장하지 않으므로 세면 나온다.
        expect(subject.days[1].sessions.map((s) => s.nth)).toEqual([1, 2])
        expect(subject.days[0].sessions.map((s) => s.nth)).toEqual([3])

        expect(subject.sessionCount).toBe(3)
        expect(subject.dayCount).toBe(2)
        expect(subject.absentTotal).toBe(3)
    })

    it('오늘 몫은 날짜로 선택해 센다 — 달을 읽어 두고도 오늘만 본다', async () => {
        subjectTeacher()
        answers({
            sessions: [
                SESSION({id: 1, date: '2026-09-10', slot: '1', absentCount: 2}),
                SESSION({id: 2, date: '2026-09-11', slot: '2', absentCount: 1}),
            ],
        })

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        subject.setMonth(9, 2026)
        await subject.fetchMonth()

        expect(subject.daySessions.map((s) => s.id)).toEqual([2])
        expect(subject.dayAbsentTotal).toBe(1)
    })
})

describe('실패', () => {
    it('읽기 실패를 삼키지 않는다', async () => {
        subjectTeacher()
        invoke.mockRejectedValue('끝 날짜가 시작 날짜보다 앞입니다.')

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        await expect(subject.fetchDay()).rejects.toBeTruthy()
        expect(subject.error).toContain('끝 날짜')
    })

    it('만들기 실패를 삼키지 않고 busy도 풀어 준다', async () => {
        subjectTeacher()
        invoke.mockRejectedValue('1교시부터 7교시까지만 만들 수 있습니다: 9')

        const subject = useSubjectStore()
        subject.setDate('2026-09-11')
        await expect(subject.addSessions([9])).rejects.toBeTruthy()
        expect(subject.error).toContain('만들 수 있습니다')
        expect(subject.busy).toBe(false)
    })
})
