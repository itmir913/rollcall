import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'

/**
 * 서류 미제출자와 NEIS 미등재.
 *
 * 두 화면 모두 **그때 모은 명단**을 보여준다. 체크를 눌러도 줄이 사라지지 않는다 —
 * 방금 누른 것이 눈앞에서 사라지면 잘못 눌렀는지 확인할 방법이 없다.
 * 그래서 표시를 바꾼 뒤에는 목록을 다시 부르지 않고 그 줄만 고친다.
 * 다시 모으려면 화면의 필터 버튼이 fetch를 부른다.
 */
export const usePendingStore = defineStore('pending', {
    state: () => ({
        docRows: [],
        neisDays: [],
        includeDone: false,
        year: null,
        month: null,
        error: '',
    }),

    getters: {
        docLeft: (s) => s.docRows.filter((r) => !r.docDone).length,
        neisLeft: (s) => s.neisDays.flatMap((d) => d.spans).filter((r) => !r.neisDone).length,
    },

    actions: {
        async fetchDocs() {
            const app = useAppStore()
            if (!app.ready) return
            this.error = ''
            try {
                this.docRows = await invoke('get_doc_pending', {
                    ...app.scope,
                    year: this.year,
                    month: this.month,
                    includeDone: this.includeDone,
                    today: app.today,
                })
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async fetchNeis() {
            const app = useAppStore()
            if (!app.ready) return
            this.error = ''
            try {
                this.neisDays = await invoke('get_neis_pending', {
                    ...app.scope,
                    year: this.year,
                    month: this.month,
                    today: app.today,
                })
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /** 그 줄만 고친다. 목록에서 빼지 않는다. */
        patch(spanId, patch) {
            for (const row of this.docRows) if (row.id === spanId) Object.assign(row, patch)
            for (const day of this.neisDays) {
                for (const row of day.spans) if (row.id === spanId) Object.assign(row, patch)
            }
        },

        async setDoc(spanId, done) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('set_doc_done', {spanId, done, today: app.today})
                this.patch(spanId, {docDone: done, docDoneOn: done ? app.today : null})
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async setNeis(spanId, done) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('set_neis_done', {spanId, done, today: app.today})
                this.patch(spanId, {neisDone: done, neisDoneOn: done ? app.today : null})
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /** 급할 때 그 날짜를 한 번에 등재 표시한다. */
        async markDay(date) {
            const app = useAppStore()
            this.error = ''
            try {
                const count = await invoke('mark_day_neis', {...app.scope, date, today: app.today})
                const day = this.neisDays.find((d) => d.date === date)
                if (day) {
                    day.spans.forEach((s) => Object.assign(s, {neisDone: true, neisDoneOn: app.today}))
                }
                return count
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 집중 등재의 저장. 한 명씩 고친 것을 한꺼번에 반영한다.
         *
         * 나이스 저장이 실패하는 날이 있어, 앱만 먼저 등재로 바뀌면 두 곳이 어긋난다.
         * 그래서 모달에서는 복사본을 고치고 여기서만 실제로 저장한다.
         */
        async saveFocus(items) {
            const app = useAppStore()
            this.error = ''
            try {
                for (const item of items) {
                    await invoke('edit_span', {
                        edit: {
                            spanId: item.id,
                            reasonId: item.reasonId,
                            typeId: item.typeId,
                            slots: item.slots,
                        },
                    })
                    await invoke('set_span_memo', {spanId: item.id, memo: item.memo ?? ''})
                    await invoke('set_span_tag', {spanId: item.id, tagId: item.tagId ?? null})
                    await invoke('set_neis_done', {spanId: item.id, done: true, today: app.today})
                }
                await this.fetchNeis()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})
