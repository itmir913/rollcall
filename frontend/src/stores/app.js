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
                await invoke('init_db')
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
