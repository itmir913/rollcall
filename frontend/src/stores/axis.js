import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'

/**
 * 출결 축 — 구분(질병 · 미인정 · 기타 · 출석인정)과 종류(지각 · 조퇴 · 결석 · 결과).
 *
 * **코드는 데이터다.** 화면이 '결석' 같은 라벨로 분기하지 않는다. 종류가 물어야 하는
 * 기간이 어느 쪽인지는 `slotPrompt`가 말해 주고, 그 값은 DB에서 온다.
 * 라벨로 비교하면 교사가 종류를 추가하는 순간 화면이 틀린다.
 */
export const useAxisStore = defineStore('axis', {
    state: () => ({
        reasons: [],
        types: [],
        codes: [],
        memos: [],
        error: '',
    }),

    getters: {
        typeById: (s) => (id) => s.types.find((t) => t.id === id) ?? null,
        reasonById: (s) => (id) => s.reasons.find((r) => r.id === id) ?? null,
        /** 그 종류가 묻는 기간의 모양. 종류가 미정이면 null이고, 그러면 전부 열어 둔다. */
        slotPromptOf: (s) => (typeId) => s.types.find((t) => t.id === typeId)?.slotPrompt ?? null,
    },

    actions: {
        /** 두 축을 사람이 읽는 말로. 비어 있으면 "미정"이다 — 미완성 기록은 정상 상태다. */
        describe(reasonId, typeId) {
            const reason = this.reasonById(reasonId)?.label ?? '미정'
            const type = this.typeById(typeId)?.label ?? '미정'
            return `${reason} ${type}`
        },

        async fetchAll() {
            this.error = ''
            try {
                const [reasons, types, codes] = await Promise.all([
                    invoke('get_reasons'),
                    invoke('get_types'),
                    invoke('get_codes'),
                ])
                this.reasons = reasons
                this.types = types
                this.codes = codes
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /** 과거에 쓴 사유를 후보 버튼으로 띄우기 위한 것이다. 타이핑을 줄인다. */
        async fetchMemos(limit = 12) {
            const app = useAppStore()
            this.error = ''
            try {
                this.memos = await invoke('get_memo_suggestions', {classId: app.classId, limit})
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})
