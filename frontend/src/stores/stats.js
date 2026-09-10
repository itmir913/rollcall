import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'

/**
 * 통계 — 한도를 지켜보는 곳.
 *
 * 체험학습 연 20일, 생리통 조퇴 월 1회처럼 **세어야 하는 규정**이 있고, 지금까지
 * 손으로 적은 표로 관리해 온 것들이다. 세는 것은 Rust가 한다.
 *
 * **한도는 입력을 막지 않는다.** 넘겼다는 이유로 기록을 거부하면 정작 넘긴 날을
 * 남길 수 없다. 프로그램은 판정하지 않는다.
 */
export const useStatsStore = defineStore('stats', {
    state: () => ({
        reports: [],
        ruleId: null,
        from: null,
        to: null,
        error: '',
    }),

    getters: {
        /** 태그가 없어 세지 못한 구간. 조용히 넘기면 학년말에 발견된다. */
        untagged: (s) => s.reports.flatMap((r) => r.untagged ?? []),
    },

    actions: {
        async fetchReports() {
            const app = useAppStore()
            if (!app.ready) return
            this.error = ''
            try {
                this.reports = await invoke('get_quota_reports', {
                    ...app.scope,
                    ruleId: this.ruleId,
                    from: this.from,
                    to: this.to,
                })
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /** 태그가 빠진 구간에 그 자리에서 태그를 붙인다. */
        async setTag(spanId, tagId) {
            this.error = ''
            try {
                await invoke('set_span_tag', {spanId, tagId})
                await this.fetchReports()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})
