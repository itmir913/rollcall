import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'

/**
 * 앱이 지금 무엇을 보고 있는가 — 학교 · 학년도 · 학급 · 오늘.
 *
 * 이 값들은 **앱 설정**(app_config)에 남는다. 마지막에 연 학급으로 다시 열려야
 * 매일 아침 같은 것을 다시 고르지 않는다.
 * 최대 교시와 제출 기한은 여기가 아니라 **학교**가 들고 있다(school 행).
 */
export const useAppStore = defineStore('app', {
    state: () => ({
        booted: false,
        /** 이번 실행에서 DB 파일이 새로 만들어졌는가. `init_db`가 알려준다. */
        firstRun: false,
        error: '',
        schoolId: null,
        school: null,
        years: [],
        yearId: null,
        grade: null,
        classNo: null,
        today: isoToday(),
    }),

    getters: {
        /** 학생 명단과 학급이 정해졌는가. 아니면 첫 실행 흐름으로 보낸다. */
        ready: (s) => Boolean(s.schoolId && s.yearId && s.grade && s.classNo),
        /**
         * Welcome 화면으로 보내야 하는가. 흐름은 `Welcome → 개요`다.
         *
         * **첫 실행 여부가 아니라 학급이 정해졌는지로 판단한다.** `firstRun`은 DB 파일을
         * 이번 실행에서 만들었다는 뜻이라, 교사가 Welcome을 끝내지 않고 앱을 닫으면
         * 다음 실행부터 영영 거짓이 된다. 그러면 사이드바에 항목도 없는 Welcome에
         * 다시 닿을 길이 없어진다. 학급이 없으면 할 수 있는 일도 없으므로, 그 상태가
         * 곧 "아직 시작하지 않았다"는 뜻이다.
         */
        needsWelcome: (s) => s.booted && !s.ready,
        /** 하루의 마지막 교시. 화면이 앱 상수를 알지 않게 한다. */
        maxSlot: (s) => s.school?.maxSlot ?? 7,
        currentYear: (s) => s.years.find((y) => y.id === s.yearId) ?? null,
        /** 커맨드 대부분이 함께 받는 네 값. 화면이 매번 조합하지 않게 한다. */
        scope: (s) => ({
            schoolId: s.schoolId,
            yearId: s.yearId,
            grade: s.grade,
            classNo: s.classNo,
        }),
    },

    actions: {
        async boot() {
            this.error = ''
            try {
                // 파일이 이번에 만들어졌는지는 `init_db`만 안다. 버리면 첫 실행인지
                // 알 방법이 없어, 처음 켠 교사가 안내 없이 개요에 놓인다.
                const status = await invoke('init_db')
                this.firstRun = Boolean(status?.created)

                const schools = await invoke('get_schools')
                this.schoolId = schools[0]?.id ?? null
                this.school = schools[0] ?? null
                this.years = await invoke('get_years')

                // app_config는 키 하나씩 읽는다. 담는 것은 마지막에 연 학급뿐이다.
                const [year, grade, classNo] = await Promise.all([
                    invoke('get_config', {key: 'yearId'}),
                    invoke('get_config', {key: 'grade'}),
                    invoke('get_config', {key: 'classNo'}),
                ])
                this.yearId = toNumber(year) ?? this.years[0]?.id ?? null
                this.grade = toNumber(grade)
                this.classNo = toNumber(classNo)
                this.booted = true
            } catch (e) {
                this.error = String(e)
                this.booted = true
                throw e
            }
        },

        async refreshSchool() {
            this.error = ''
            try {
                this.school = await invoke('get_school', {schoolId: this.schoolId})
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async selectClass({yearId, grade, classNo}) {
            this.error = ''
            this.yearId = yearId
            this.grade = grade
            this.classNo = classNo
            try {
                await Promise.all([
                    invoke('set_config', {key: 'yearId', value: String(yearId)}),
                    invoke('set_config', {key: 'grade', value: String(grade)}),
                    invoke('set_config', {key: 'classNo', value: String(classNo)}),
                ])
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})

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
