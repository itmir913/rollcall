import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'
import {MONTHS, calendarYearOf} from '../services/academicYear'

/**
 * 출결 기록 — 한 달을 **날짜별 카드**로 본다.
 *
 * 한 카드에 한 달을 전부 넣으면 날짜 머리글이 목록 중간에 섞여 어디까지가 그날인지
 * 흐려진다. 그래서 Rust가 날짜별로 묶어 내려준다.
 *
 * 월 필터는 3월부터 시작한다 — 학년도가 3월에 열리므로 1월·2월이 뒤에 온다.
 */
export {MONTHS}

export const useLogStore = defineStore('log', {
    state: () => ({
        year: null,
        month: null,
        days: [],
        error: '',
    }),

    getters: {
        spans: (s) => s.days.flatMap((d) => d.spans),
        /** 그 달의 종류별 건수. 화면이 세지 않게 한다. */
        counts(state) {
            const out = {}
            for (const span of state.days.flatMap((d) => d.spans)) {
                const key = span.typeLabel ?? '미정'
                out[key] = (out[key] ?? 0) + 1
            }
            return out
        },
    },

    actions: {
        /** 달을 고른다. 1월과 2월은 학년도의 이듬해다. */
        setMonth(month, academicYear) {
            this.month = month
            this.year = calendarYearOf(academicYear, month)
        },

        async fetchMonth() {
            const app = useAppStore()
            if (!app.ready || !this.year || !this.month) return
            this.error = ''
            try {
                this.days = await invoke('get_month_log', {
                    ...app.scope,
                    year: this.year,
                    month: this.month,
                })
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})
