import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {readNeisFile} from '../services/neisFile'
import {useAppStore} from './app'
import {useAxisStore} from './axis'

/**
 * 나이스 파일 열기. **교체가 아니라 차분이다.**
 *
 * 경계는 명렬표와 같다 — 파일 서식은 `services/neisFile.js`가 읽고, "같은가 · 다른가 ·
 * 없는가"라는 판단은 Rust가 한다. 그래야 화면을 고쳐도 업무 규칙이 따라 흔들리지 않는다.
 *
 * **일 · 월을 교사에게 묻지 않는다.** 기간은 파일 안에 있다. 그래서 이 store에는
 * 기간을 선택하는 상태가 없다.
 */
export const useNeisImportStore = defineStore('neisImport', {
    state: () => ({
        /** 파일에서 읽은 줄. 미리보기와 적용에 그대로 다시 넘긴다. */
        rows: [],
        meta: null,
        preview: null,
        /** 교사가 선택한 것. 처음에는 추가만 켜 둔다 — 덮어쓰기는 스스로 선택하게 한다. */
        picked: {add: new Set(), replace: new Set()},
        markNeis: true,
        error: '',
        busy: false,
    }),

    getters: {
        /** 무엇이든 할 일이 있는가. 없으면 적용 단추가 꺼진다. */
        hasWork: (s) =>
            s.picked.add.size > 0 ||
            s.picked.replace.size > 0 ||
            (s.markNeis && (s.preview?.same ?? 0) > 0),
    },

    actions: {
        reset() {
            this.rows = []
            this.meta = null
            this.preview = null
            this.picked = {add: new Set(), replace: new Set()}
            this.error = ''
        },

        /**
         * 파일을 읽고 차분까지 한 번에. 못 읽으면 **던진다** — 조용히 빈 결과를 내면
         * 교사는 "결석이 없는 달"과 "읽지 못한 파일"을 구별할 수 없다.
         */
        async load(bytes) {
            const app = useAppStore()
            const axis = useAxisStore()
            this.reset()
            this.busy = true
            try {
                if (axis.types.length === 0) await axis.fetchAll()
                const read = await readNeisFile(bytes, {
                    reasons: axis.reasons,
                    types: axis.types,
                })
                this.rows = read.rows
                this.meta = read.meta
                this.preview = await invoke('preview_neis_import', {
                    classId: app.classId,
                    rows: read.rows,
                    today: app.today,
                })
                // 추가는 기본으로 선택한다. 앱에 없는 기록을 넣는 것이 파일 열기의 목적이다.
                this.picked.add = new Set(
                    this.preview.items.filter((i) => i.verdict === 'add').map((i) => i.key),
                )
                return this.preview
            } catch (e) {
                this.error = String(e.message ?? e)
                throw e
            } finally {
                this.busy = false
            }
        },

        toggle(kind, key) {
            const set = new Set(this.picked[kind])
            if (set.has(key)) set.delete(key)
            else set.add(key)
            this.picked = {...this.picked, [kind]: set}
        },

        pickAll(kind, keys) {
            this.picked = {...this.picked, [kind]: new Set(keys)}
        },

        async apply() {
            const app = useAppStore()
            this.busy = true
            this.error = ''
            try {
                return await invoke('apply_neis_import', {
                    classId: app.classId,
                    rows: this.rows,
                    choice: {
                        add: [...this.picked.add],
                        replace: [...this.picked.replace],
                        markNeis: this.markNeis,
                    },
                    today: app.today,
                })
            } catch (e) {
                this.error = String(e)
                throw e
            } finally {
                this.busy = false
            }
        },
    },
})
