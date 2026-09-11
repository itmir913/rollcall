import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {academicYearOf, yearSpanOf} from '../services/academicYear'

/**
 * 맡은 것의 두 갈래. 담임은 구분 · 종류 · 기간을 기록하고, 교과는 그 교시에
 * 있었는지만 기록한다. 그래서 화면이 나뉘고, 한쪽의 숫자가 다른 쪽에 새면 안 된다.
 */
export const MODES = ['homeroom', 'subject']

/**
 * 앱이 지금 무엇을 보고 있는가 — **맡은 학급 하나와 오늘.**
 *
 * 범위는 `classId` 하나다. 전에는 학교 · 학년도 · 학년 · 반 네 값으로 걸러냈지만,
 * 같은 학생이 내 담임 반에도 내 교과 강좌에도 있을 수 있어 학적으로는 두 기록이
 * 구분되지 않는다. 지금 무엇을 보고 있는지는 `teaching_class` 행 하나가 말한다.
 *
 * **모드는 고른 학급의 역할이다.** 따로 담지 않고 게터로 둔 이유는, 둘을 나란히
 * 담으면 담임 모드에 교과 강좌가 걸린 화면이 나올 수 있고 그때는 DB 트리거가
 * 거절할 때까지 아무도 모르기 때문이다. `lastMode`는 아직 아무 학급도 고르지
 * 않았을 때만 서는 값이다.
 *
 * 이 값들은 **앱 설정**(app_config)에 남는다. 담는 열쇠는 —
 * `yearId` · `mode` · `homeroomClassId` · `subjectClassId`.
 * 최대 교시와 제출 기한은 여기가 아니라 **학교**가 들고 있고, 그 학교는 학급이 안다.
 */
export const useAppStore = defineStore('app', {
    state: () => ({
        booted: false,
        /** 이번 실행에서 DB 파일이 새로 만들어졌는가. `init_db`가 알려준다. */
        firstRun: false,
        error: '',
        schools: [],
        years: [],
        yearId: null,
        /** 지금 보고 있는 학급. 담임 커맨드가 받는 단 하나의 범위다. */
        classId: null,
        /** 내가 맡은 것 전부. 담임 학급과 교과 강좌가 한 목록에 온다. */
        classes: [],
        /**
         * 모드마다 마지막에 본 학급. **오가며 쓰는 값이라 따로 기억한다** —
         * 돌아왔을 때 있던 자리가 아니면 매번 다시 골라야 한다.
         */
        lastClassId: {homeroom: null, subject: null},
        /** 학급을 고르기 전까지의 모드. 고른 뒤에는 학급의 역할이 우선한다. */
        lastMode: 'homeroom',
        today: isoToday(),
    }),

    getters: {
        /** 지금 보고 있는 학급 행. 이름 · 역할 · 학교가 여기 붙어 있다. */
        currentClass: (s) => classOf(s),
        /**
         * homeroom | subject. **고른 학급이 정한다.**
         *
         * 화면이 이 값으로 갈리므로, 학급과 어긋나는 순간 담임 화면이 교과 강좌의
         * 명단을 그린다. 그래서 따로 담지 않고 여기서 읽는다.
         */
        mode: (s) => classOf(s)?.role ?? s.lastMode,
        /** 학급이 골라졌는가. 아니면 아직 시작하지 않은 것이다. */
        ready: (s) => s.classId != null,
        /**
         * Welcome 화면으로 보내야 하는가. 흐름은 `Welcome → 개요`다.
         *
         * **첫 실행 여부가 아니라 학급이 골라졌는지로 판단한다.** `firstRun`은 DB 파일을
         * 이번 실행에서 만들었다는 뜻이라, 교사가 Welcome을 끝내지 않고 앱을 닫으면
         * 다음 실행부터 영영 거짓이 된다. 그러면 사이드바에 항목도 없는 Welcome에
         * 다시 닿을 길이 없어진다. 학급이 없으면 할 수 있는 일도 없으므로, 그 상태가
         * 곧 "아직 시작하지 않았다"는 뜻이다.
         */
        needsWelcome: (s) => s.booted && s.classId == null,
        /** 지금 담임 모드인가. 화면이 문자열을 직접 비교하지 않게 한다. */
        isHomeroom: (s) => (classOf(s)?.role ?? s.lastMode) === 'homeroom',
        homeroomClasses: (s) => s.classes.filter((c) => c.role === 'homeroom'),
        subjectClasses: (s) => s.classes.filter((c) => c.role === 'subject'),
        /**
         * 모드 스위치를 그려야 하는가.
         *
         * 어떤 교사는 담임만, 어떤 교사는 교과만 한다. 한쪽만 맡은 교사에게 스위치를
         * 보여주면 누를 때마다 아무것도 없는 화면으로 넘어간다 — 그 자리가 비어 있는
         * 것이 정상인지 고장인지 알 방법이 없다.
         */
        needsModeSwitch: (s) =>
            s.classes.some((c) => c.role === 'homeroom') &&
            s.classes.some((c) => c.role === 'subject'),
        /** 지금 학급이 속한 학교. 학교 설정을 읽고 쓰는 화면이 이것을 본다. */
        schoolId: (s) => schoolOf(s)?.id ?? null,
        school: (s) => schoolOf(s),
        /** 하루의 마지막 교시. 화면이 앱 상수를 알지 않게 한다. */
        maxSlot: (s) => schoolOf(s)?.maxSlot ?? 7,
        currentYear: (s) => s.years.find((y) => y.id === s.yearId) ?? null,
    },

    actions: {
        async boot() {
            this.error = ''
            try {
                // 파일이 이번에 만들어졌는지는 `init_db`만 안다. 버리면 첫 실행인지
                // 알 방법이 없어, 처음 켠 교사가 안내 없이 개요에 놓인다.
                const status = await invoke('init_db')
                this.firstRun = Boolean(status?.created)

                this.schools = await invoke('get_schools')
                this.years = await invoke('get_years')
                // **첫 실행에는 학년도 행이 없다.** 시드가 만들지 않는 이유는 오늘이
                // 언제인지 DB가 모르기 때문이다. 여기서 오늘로 채운다 — 교사에게
                // 되묻지 않는다. 3월에 열리는 학년도라 1 · 2월은 지난해 것이다.
                if (this.years.length === 0) {
                    const year = academicYearOf(this.today)
                    const {startsOn, endsOn} = yearSpanOf(year)
                    await invoke('create_year', {year, startsOn, endsOn})
                    this.years = await invoke('get_years')
                }

                // app_config는 키 하나씩 읽는다. 담는 것은 아래가 전부다.
                const [year, mode, homeroom, subject] = await Promise.all([
                    invoke('get_config', {key: 'yearId'}),
                    invoke('get_config', {key: 'mode'}),
                    invoke('get_config', {key: 'homeroomClassId'}),
                    invoke('get_config', {key: 'subjectClassId'}),
                ])
                this.yearId = toNumber(year) ?? this.years[0]?.id ?? null
                this.lastMode = MODES.includes(mode) ? mode : 'homeroom'
                await this.fetchClasses()
                this.restore(toNumber(homeroom), toNumber(subject))
                this.booted = true
            } catch (e) {
                this.error = String(e)
                this.booted = true
                throw e
            }
        },

        /**
         * 설정에 적힌 학급을 복원한다. **목록에 없는 번호는 버린다** —
         * 학년도가 바뀌어 지난해 학급이 마감되면 그 번호로 묻는 질의가 전부 빈다.
         * 적힌 것이 없으면 그 역할의 첫 학급을 고른다. 맡은 것이 있는데 Welcome으로
         * 되돌리면, 이미 쌓인 기록을 두고 학급을 다시 만들게 된다.
         *
         * 마지막에 본 모드에 맡은 것이 없으면 있는 쪽을 연다. 비담임 교사가 담임
         * 모드로 열려 아무것도 없는 화면을 보는 것을 여기서 막는다.
         */
        restore(homeroomId, subjectId) {
            const pick = (id, role) => {
                const own = this.classes.filter((c) => c.role === role)
                return own.find((c) => c.id === id)?.id ?? own[0]?.id ?? null
            }
            this.lastClassId = {
                homeroom: pick(homeroomId, 'homeroom'),
                subject: pick(subjectId, 'subject'),
            }
            this.classId =
                this.lastClassId[this.lastMode] ??
                MODES.map((m) => this.lastClassId[m]).find((id) => id != null) ??
                null
        },

        async fetchClasses() {
            this.error = ''
            try {
                this.classes = await invoke('get_teaching_classes', {yearId: this.yearId})
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async refreshSchool() {
            this.error = ''
            try {
                this.schools = await invoke('get_schools')
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 지금 볼 학급을 고른다. **모드가 그 학급의 역할로 따라온다** —
         * 고른 것과 보고 있는 것이 어긋날 자리를 남기지 않는다.
         *
         * 학년도도 학급이 들고 있으므로 함께 맞춘다. 따로 고르게 하면 3월에
         * 학년도만 바꾸고 학급은 지난해 것으로 남는 상태가 생긴다.
         */
        async selectClass(classId) {
            this.error = ''
            const target = this.classes.find((c) => c.id === classId)
            if (!target) {
                this.error = `맡은 학급이 아닙니다: ${classId}`
                throw new Error(this.error)
            }
            this.classId = target.id
            this.lastMode = target.role
            this.lastClassId = {...this.lastClassId, [target.role]: target.id}
            this.yearId = target.yearId
            try {
                await Promise.all([
                    invoke('set_config', {key: 'mode', value: target.role}),
                    invoke('set_config', {key: `${target.role}ClassId`, value: String(target.id)}),
                    invoke('set_config', {key: 'yearId', value: String(target.yearId)}),
                ])
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 담임과 교과를 오간다. 학급은 **그 모드에서 마지막에 본 것**으로 돌아온다 —
         * 매번 다시 고르게 하면 하루에도 몇 번씩 오가는 교사에게 그만큼 클릭이 는다.
         */
        async setMode(mode) {
            this.error = ''
            if (!MODES.includes(mode)) {
                this.error = `모르는 모드입니다: ${mode}`
                throw new Error(this.error)
            }
            this.lastMode = mode
            this.classId = this.lastClassId[mode] ?? null
            try {
                await invoke('set_config', {key: 'mode', value: mode})
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 맡은 것을 새로 만들고 그것으로 옮긴다. Welcome과 설정이 부른다.
         *
         * **이미 맡고 있는 것이면 만들지 않고 고른다.** 교사가 [저장]을 두 번 누르는 것은
         * 흔한 일이고, 그때마다 학급이 늘면 명단이 어느 쪽에 붙었는지 알 수 없게 된다.
         * 담임은 학년 · 반이, 교과는 이름이 같은 것을 가리킨다.
         */
        async createClass({role = 'homeroom', name = null, grade = null, classNo = null}) {
            this.error = ''
            const same = this.classes.find(
                (c) =>
                    c.role === role &&
                    !c.validTo &&
                    (role === 'homeroom'
                        ? c.grade === grade && c.classNo === classNo
                        : c.name === name),
            )
            if (same) {
                await this.selectClass(same.id)
                return same.id
            }
            try {
                const id = await invoke('create_teaching_class', {
                    schoolId: this.schoolId,
                    yearId: this.yearId,
                    role,
                    name,
                    grade,
                    classNo,
                    // 학급이 언제부터 내 것인가. 학년도가 열리는 날이 기본이고,
                    // 학기 중에 맡게 되면 그날부터다. 마감은 valid_to가 맡는다.
                    validFrom: this.currentYear?.startsOn ?? this.today,
                })
                await this.fetchClasses()
                await this.selectClass(id)
                return id
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})

/** 지금 보고 있는 학급 행. 게터 여럿이 같은 것을 묻기에 한 곳에 둔다. */
function classOf(state) {
    return state.classes.find((c) => c.id === state.classId) ?? null
}

/**
 * 지금 학급이 속한 학교. 아직 학급이 없으면 첫 학교다 —
 * Welcome에서 학교 이름과 최대 교시를 먼저 정하기 때문이다.
 */
function schoolOf(state) {
    const id = classOf(state)?.schoolId ?? state.schools[0]?.id ?? null
    return state.schools.find((s) => s.id === id) ?? null
}

/** 오늘 날짜를 ISO로. 저장은 언제나 ISO다. */
export function isoToday(now = new Date()) {
    const pad = (n) => String(n).padStart(2, '0')
    return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

/** 화면·내보내기 표기. `2026.09.10.(목)` */
export function formatKorean(iso) {
    if (!iso) return ''
    const [y, m, d] = iso.split('-').map(Number)
    const week = ['일', '월', '화', '수', '목', '금', '토'][new Date(y, m - 1, d).getDay()]
    return `${y}.${String(m).padStart(2, '0')}.${String(d).padStart(2, '0')}.(${week})`
}

function toNumber(v) {
    const n = Number(v)
    return Number.isFinite(n) && v !== undefined && v !== null && v !== '' ? n : null
}
