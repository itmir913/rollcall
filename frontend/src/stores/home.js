import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'

/**
 * 개요가 한 번에 받아 가는 요약.
 *
 * 화면이 커맨드 대여섯 개를 조합해 숫자를 만들면 그 조합 규칙이 프런트엔드의
 * 비즈니스 로직이 된다. 숫자는 전부 Rust가 세어 준다.
 *
 * 사이드바 배지도 이 요약에서 온다. 배지는 **안 받은 것을 전부** 센다 —
 * 마감 뒤에 재촉하는 것은 이미 늦은 일이라, 여유가 있을 때 보이는 쪽이
 * 서류를 실제로 받게 한다.
 */
export const useHomeStore = defineStore('home', {
    state: () => ({
        summary: null,
        error: '',
    }),

    getters: {
        docPending: (s) => s.summary?.docPending ?? 0,
        neisPending: (s) => s.summary?.neisPending ?? 0,
    },

    actions: {
        async fetchSummary(limit = 5) {
            const app = useAppStore()
            if (!app.ready) return
            this.error = ''
            try {
                this.summary = await invoke('get_home_summary', {
                    classId: app.classId,
                    date: app.today,
                    limit,
                })
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})
