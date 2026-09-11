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

        /**
         * 임의 기간의 기록. **NEIS 검증이 파일의 기간으로 대조하려고 쓴다.**
         *
         * 달로 묻지 않는 이유는, 나이스 파일의 기간이 달에 맞춰 떨어지지 않기
         * 때문이다. 화면이 마지막에 보던 달로 맞춰 보면 6월 파일을 9월 기록과
         * 대조하고는 "전부 앱에 없음"이라고 말한다.
         */
        async fetchBetween(from, to) {
            const app = useAppStore()
            this.error = ''
            try {
                return await invoke('get_spans_between', {
                    classId: app.classId,
                    from,
                    to,
                    today: app.today,
                })
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async fetchMonth() {
            const app = useAppStore()
            if (!app.ready || !this.year || !this.month) return
            this.error = ''
            try {
                this.days = await invoke('get_month_log', {
                    classId: app.classId,
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
