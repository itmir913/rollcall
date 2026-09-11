import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {formatKorean, useAppStore} from './app'
import {calendarYearOf} from '../services/academicYear'

/**
 * 교과 차시 — **내 수업에 있었는가 하나만** 기록한다.
 *
 * 담임 기록과 표부터 다르다. 여기에는 구분 · 종류 · 기간 · 서류 · 나이스가 없다.
 * 수업을 시작할 때 [교시 추가]로 칸 하나를 만들고, 그 칸에서 빠진 학생만 적는다 —
 * **칸이 있다는 것이 곧 그 교시를 불렀다는 뜻이다.** 결석자 행만으로는 "빠진 사람이
 * 없는 날"과 "아직 안 부른 날"이 구별되지 않는다.
 *
 * **담임 모드에서는 아무것도 읽지 않는다.** 한쪽의 숫자가 다른 쪽 화면에 유출되면
 * 비담임 교사가 매일 남의 일을 보게 되고, 담임만 하는 교사는 평생 빈 칸을 본다.
 * 단 하나의 예외가 `homeroomNote`인데, 그것은 Rust가 차시 명단에 실어 주는
 * **읽기 전용 참고**다 — 이 스토어는 그것을 쓰지도 고치지도 않는다.
 */
export const useSubjectStore = defineStore('subject', {
    state: () => ({
        /** 오늘 수업이 보고 있는 날짜. 저장은 언제나 ISO다. */
        date: null,
        /** 마지막으로 읽은 기간. 찍은 뒤 그 자리를 다시 읽으려고 들고 있다. */
        from: null,
        to: null,
        /** 그 기간의 차시. 날짜 오름차순 · 교시 오름차순으로 온다. */
        sessions: [],
        /** 마지막으로 읽은 강좌. 강좌를 옮기면 앞 강좌의 명단을 지운다. */
        classId: null,
        /** 지금 선택한 차시. 그 칸의 명단만 화면에 뜬다. */
        sessionId: null,
        /** 그 차시의 명단 전원. 빠진 학생만 오지 않는다. */
        roll: [],
        /** 수업 기록의 월 필터. */
        year: null,
        month: null,
        error: '',
        busy: false,
    }),

    getters: {
        /** 지금 보는 날짜의 차시. 기간을 넓게 읽어 두었어도 그날만 고른다. */
        daySessions: (s) => s.sessions.filter((x) => x.date === s.date),

        /**
         * 날짜별 묶음. **최신이 위에 온다** — 지난 기록을 확인하는 화면이라
         * 방금 한 수업이 맨 위에 있어야 한다. Rust는 오름차순으로 주므로 여기서 뒤집는다.
         *
         * `nth`는 **그 기간에서 센 차례**다. 차시 번호를 저장하지 않으므로 날짜 · 교시
         * 순으로 세면 나오는 값이고, 세는 범위가 달라지면 값도 달라진다. 그래서 화면은
         * 이것을 `N차시`가 아니라 `이 달 N번째`로 적는다.
         */
        days(state) {
            const groups = []
            const byDate = new Map()
            state.sessions.forEach((session, index) => {
                if (!byDate.has(session.date)) {
                    const group = {date: session.date, dateLabel: formatKorean(session.date), sessions: []}
                    byDate.set(session.date, group)
                    groups.push(group)
                }
                byDate.get(session.date).sessions.push({...session, nth: index + 1})
            })
            return groups.reverse()
        },

        /** 지금 선택한 차시 한 행. */
        current: (s) => s.sessions.find((x) => x.id === s.sessionId) ?? null,

        /** 이 차시에 빠진 학생. 출석은 저장하지 않으므로 나머지가 곧 출석이다. */
        absentRows: (s) => s.roll.filter((r) => r.absent),

        /**
         * 담임으로 적어 둔 기록이 있는 학생. **읽기 전용 참고다.**
         * 내가 이 화면에서 찍은 결석과 섞이면 안 되므로 목록부터 따로 둔다.
         */
        notedRows: (s) => s.roll.filter((r) => r.homeroomNote),

        /** 읽어 둔 기간의 차시 수 · 수업한 날 수 · 결석 연인원. 화면이 세지 않게 한다. */
        sessionCount: (s) => s.sessions.length,
        dayCount: (s) => new Set(s.sessions.map((x) => x.date)).size,
        absentTotal: (s) => s.sessions.reduce((sum, x) => sum + (x.absentCount ?? 0), 0),

        /** 지금 날짜의 결석 연인원. 개요가 오늘 몫만 셀 때 쓴다. */
        dayAbsentTotal(state) {
            return state.sessions
                .filter((x) => x.date === state.date)
                .reduce((sum, x) => sum + (x.absentCount ?? 0), 0)
        },

        /** 고른 달의 첫날과 끝날. 말일은 달마다 다르므로 여기서 계산한다. */
        monthRange(state) {
            if (!state.year || !state.month) return null
            const pad = (n) => String(n).padStart(2, '0')
            const last = new Date(state.year, state.month, 0).getDate()
            return {
                from: `${state.year}-${pad(state.month)}-01`,
                to: `${state.year}-${pad(state.month)}-${pad(last)}`,
            }
        },
    },

    actions: {
        /**
         * 고른 차시와 그 명단을 함께 비운다. **둘은 한 쌍이다** —
         * 차시만 비우고 명단을 남기면 화면에 아무 칸에도 속하지 않는 번호가 뜨고,
         * 명단만 비우면 그 뒤의 찍기가 보이지 않는 칸으로 들어간다.
         */
        clearPick() {
            this.sessionId = null
            this.roll = []
        },

        /**
         * 보는 날짜를 옮긴다. **날짜가 실제로 바뀌면 고른 차시와 명단을 비운다.**
         *
         * 비우지 않으면 9월 11일 차시를 열어 둔 채 9월 12일로 옮겼을 때, 번호를 누른
         * 결석이 화면에 표시된 9월 12일이 아니라 **9월 11일 차시에** 들어간다.
         * 교사는 보이지 않는 날의 기록이 바뀐 것을 확인할 방법이 없다.
         *
         * 같은 날짜를 다시 고른 것은 옮긴 것이 아니므로 열어 둔 칸을 그대로 둔다.
         */
        setDate(iso) {
            if (iso === this.date) return
            this.date = iso
            this.clearPick()
        },

        /** 하루씩 옮긴다. 어제 · 내일 버튼이 부른다. */
        async move(delta) {
            const base = new Date(`${this.date}T00:00:00`)
            base.setDate(base.getDate() + delta)
            const pad = (n) => String(n).padStart(2, '0')
            this.setDate(`${base.getFullYear()}-${pad(base.getMonth() + 1)}-${pad(base.getDate())}`)
            await this.fetchDay()
        },

        /** 달을 고른다. 1월과 2월은 학년도의 이듬해다. */
        setMonth(month, academicYear) {
            this.month = month
            this.year = calendarYearOf(academicYear, month)
        },

        /**
         * 기간 안의 차시를 읽는다.
         *
         * **담임 모드면 읽지 않고 비운다.** 남겨 두면 담임 화면이 교과 강좌의 숫자를
         * 그대로 그린다 — 두 모드가 서로 유출되지 않는다는 규칙이 깨지는 자리가 여기다.
         */
        async fetchRange(from, to) {
            const app = useAppStore()
            if (!app.ready || app.isHomeroom) {
                this.sessions = []
                this.clearPick()
                return []
            }
            // 강좌를 옮겼으면 앞 강좌의 명단을 들고 있지 않는다.
            if (this.classId !== app.classId) {
                this.classId = app.classId
                this.clearPick()
            }
            this.error = ''
            try {
                this.sessions = await invoke('get_subject_sessions', {
                    classId: app.classId,
                    from,
                    to,
                })
                this.from = from
                this.to = to
                return this.sessions
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /** 지금 보는 날짜 하루치. */
        async fetchDay() {
            if (!this.date) return []
            return await this.fetchRange(this.date, this.date)
        },

        /** 고른 달 한 달치. */
        async fetchMonth() {
            const range = this.monthRange
            if (!range) return []
            return await this.fetchRange(range.from, range.to)
        },

        /** 마지막으로 읽은 기간을 다시 읽는다. 찍은 뒤 결석 수를 맞추는 자리다. */
        async refresh() {
            if (!this.from || !this.to) return []
            return await this.fetchRange(this.from, this.to)
        },

        /**
         * 고른 교시들로 차시를 만든다. **연강(3 · 4교시)도 두 칸이다** —
         * 중간에 나간 학생이 실제로 있다. 누르는 수고는 교시를 여럿 고르게 해 줄인다.
         *
         * 같은 칸이 이미 있으면 Rust가 그 id를 돌려주므로 두 번 눌러도 칸이 분리되지 않는다.
         * 만든 뒤에는 **첫 칸 하나만** 선택한다 — 화면이 스스로 옮겨 다니면 어디를 보고
         * 있었는지 매번 다시 찾게 된다.
         */
        async addSessions(slots) {
            const app = useAppStore()
            this.error = ''
            this.busy = true
            try {
                const ids = []
                for (const slot of slots) {
                    ids.push(await invoke('create_subject_session', {
                        classId: app.classId,
                        date: this.date,
                        slot: String(slot),
                    }))
                }
                await this.fetchDay()
                if (ids.length > 0) await this.select(ids[0])
                return ids
            } catch (e) {
                this.error = String(e)
                throw e
            } finally {
                this.busy = false
            }
        },

        /** 차시를 지운다. 그 칸의 결석 기록도 함께 사라진다. */
        async removeSession(sessionId) {
            this.error = ''
            try {
                await invoke('delete_subject_session', {sessionId})
                if (this.sessionId === sessionId) this.clearPick()
                await this.refresh()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /** 차시 메모. 자유 문장이고 앱은 해석하지 않는다(`수행평가` · `보강`). */
        async setSessionMemo(sessionId, memo) {
            this.error = ''
            try {
                await invoke('set_session_memo', {sessionId, memo})
                await this.refresh()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /** 차시 하나를 선택한다. 그 칸의 명단이 뒤따라온다. */
        async select(sessionId) {
            this.sessionId = sessionId
            await this.fetchRoll()
        },

        async fetchRoll() {
            if (this.sessionId == null) {
                this.roll = []
                return []
            }
            this.error = ''
            try {
                this.roll = await invoke('get_session_roll', {sessionId: this.sessionId})
                return this.roll
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 학생 하나를 찍는다. 같은 학생을 다시 찍으면 취소다 —
         * 담임 쪽 "같은 조합을 다시 찍으면 취소"와 같은 규칙이라 확인을 묻지 않는다.
         * 돌려주는 값이 **찍은 뒤의** 결석 여부다.
         *
         * **고른 차시가 없으면 찍지 않는다.** 화면이 명단을 그리지 않으니 닿을 일이
         * 없어 보이지만, 닿는 순간 어느 칸에 들어갔는지 아무도 모르는 기록이 된다.
         */
        async toggle(studentId) {
            if (this.sessionId == null) {
                this.error = '먼저 교시를 고르세요.'
                throw new Error(this.error)
            }
            this.error = ''
            this.busy = true
            try {
                const absent = await invoke('toggle_subject_absence', {
                    sessionId: this.sessionId,
                    studentId,
                })
                await this.fetchRoll()
                await this.refresh()
                return absent
            } catch (e) {
                this.error = String(e)
                throw e
            } finally {
                this.busy = false
            }
        },

        /**
         * 빠진 학생 한 줄의 메모. **결석을 먼저 찍어야 한다** — 메모는 결석에 붙는
         * 말이라, 빠지지 않은 학생에게 메모만 남으면 그 줄이 결석인지 아닌지 알 수 없다.
         */
        async setAbsenceMemo(studentId, memo) {
            if (this.sessionId == null) {
                this.error = '먼저 교시를 고르세요.'
                throw new Error(this.error)
            }
            this.error = ''
            try {
                await invoke('set_subject_absence_memo', {
                    sessionId: this.sessionId,
                    studentId,
                    memo,
                })
                await this.fetchRoll()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})
