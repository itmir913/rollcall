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
 * 기한 뒤에 재촉하는 것은 이미 늦은 일이라, 여유가 있을 때 보이는 쪽이
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
        /**
         * 개요의 숫자. **담임 요약이다.**
         *
         * 실패하면 앞의 요약을 **반드시 비운다.** 남겨 두면 담임에서 교과로 넘어갔을 때
         * 교과 화면에 담임 반 학생 이름과 미제출 건수가 그대로 그려진다 — 두 모드가
         * 서로 유출되지 않는다는 규칙이 화면에서 깨지는 자리가 바로 여기다.
         */
        async fetchSummary(limit = 5) {
            const app = useAppStore()
            if (!app.ready || !app.isHomeroom) {
                this.summary = null
                return
            }
            this.error = ''
            try {
                this.summary = await invoke('get_home_summary', {
                    classId: app.classId,
                    date: app.today,
                    limit,
                })
            } catch (e) {
                this.summary = null
                this.error = String(e)
                throw e
            }
        },
    },
})
