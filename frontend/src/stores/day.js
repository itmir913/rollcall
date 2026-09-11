import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'

/**
 * 오늘의 출결 — 격자와 그날의 구간들.
 *
 * `draft`는 **지금 찍을 조합**이다. 학생 번호를 누르면 그대로 저장되고, 같은 조합을
 * 다시 누르면 취소된다. 그 판단은 Rust가 한다 — 화면은 결과를 다시 그릴 뿐이다.
 */
export const useDayStore = defineStore('day', {
    state: () => ({
        date: null,
        grid: null,
        draft: {reasonId: null, typeId: null, slots: []},
        error: '',
        busy: false,
    }),

    getters: {
        rows: (s) => s.grid?.rows ?? [],
        spans: (s) => s.grid?.spans ?? [],
        dateLabel: (s) => s.grid?.dateLabel ?? '',
        /** 구간이 하나라도 있는 학생 수. 구간 수가 아니다. */
        recorded: (s) => new Set((s.grid?.spans ?? []).map((x) => x.studentId)).size,
    },

    actions: {
        setDate(iso) {
            this.date = iso
        },

        /** 하루씩 옮긴다. 어제·내일 버튼이 부른다. */
        async move(delta) {
            const base = new Date(`${this.date}T00:00:00`)
            base.setDate(base.getDate() + delta)
            const pad = (n) => String(n).padStart(2, '0')
            this.date = `${base.getFullYear()}-${pad(base.getMonth() + 1)}-${pad(base.getDate())}`
            await this.fetchGrid()
        },

        async fetchGrid() {
            const app = useAppStore()
            // 날짜를 고르는 화면은 오늘의 출결뿐이다. 출결 기록 · 서류 미제출자에서
            // 수정하면 여기 날짜가 비어 있는데, 그대로 부르면 커맨드가 늘 실패해
            // `error`가 오염되고 진짜 저장 실패와 구별되지 않는다.
            if (!app.ready || !this.date) return
            this.error = ''
            try {
                this.grid = await invoke('get_day_grid', {classId: app.classId, date: this.date})
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 학생 하나에게 지금 조합을 찍는다.
         * 같은 조합이 이미 있으면 Rust가 그 건을 지운다(무르기).
         */
        async stamp(studentId) {
            this.error = ''
            this.busy = true
            try {
                const result = await invoke('stamp_span', {
                    input: {
                        classId: useAppStore().classId,
                        studentId,
                        date: this.date,
                        reasonId: this.draft.reasonId,
                        typeId: this.draft.typeId,
                        slots: [...this.draft.slots],
                    },
                })
                await this.fetchGrid()
                return result
            } catch (e) {
                this.error = String(e)
                throw e
            } finally {
                this.busy = false
            }
        },

        /** 구분 · 종류 · 기간을 고친다. 수정 모달이 저장을 누를 때만 부른다. */
        async editSpan(spanId, {reasonId, typeId, slots}) {
            this.error = ''
            try {
                await invoke('edit_span', {edit: {spanId, reasonId, typeId, slots}})
                await this.fetchGrid()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async deleteSpan(spanId) {
            this.error = ''
            try {
                await invoke('delete_span', {spanId})
                await this.fetchGrid()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async setMemo(spanId, memo) {
            this.error = ''
            try {
                await invoke('set_span_memo', {spanId, memo})
                await this.fetchGrid()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async setTag(spanId, tagId) {
            this.error = ''
            try {
                await invoke('set_span_tag', {spanId, tagId})
                await this.fetchGrid()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /** 여러 날에 같은 조합을 찍기 전에 대상 날짜를 미리 본다. */
        async previewBulk(from, to) {
            const app = useAppStore()
            this.error = ''
            try {
                // 휴업일은 학교가 들고 있지만 화면은 학급만 안다. 학급에서 학교를
                // 얻는 것은 Rust가 한 곳에 모아 둔다.
                return await invoke('preview_bulk', {classId: app.classId, from, to})
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async applyBulk(studentId, from, to) {
            this.error = ''
            try {
                const result = await invoke('apply_bulk', {
                    input: {
                        classId: useAppStore().classId,
                        studentId,
                        date: from,
                        reasonId: this.draft.reasonId,
                        typeId: this.draft.typeId,
                        slots: [...this.draft.slots],
                    },
                    from,
                    to,
                })
                await this.fetchGrid()
                return result
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})
