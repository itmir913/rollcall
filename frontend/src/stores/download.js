import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {exportCsv} from '../services/download'

/**
 * 파일로 저장한다. **화면이 `invoke`를 직접 부르지 않게 하려고 있는 store다.**
 *
 * 파일 저장은 화면 넷에서 같은 모양으로 일어나고(출결 기록 · 서류 미제출자 ·
 * NEIS 미등재 · 통계), 그 전부가 실패를 알릴 자리를 가져야 한다. 이전에는 템플릿에서
 * 서비스를 바로 불러 실패가 아무 데도 남지 않았다 — 교사가 CSV를 눌러도 아무 일이
 * 일어나지 않고, 왜인지 알 방법이 없었다.
 *
 * 성공도 알린다. 저장 대화상자에서 고른 경로가 어디였는지 곧 잊어버리기 때문이다.
 */
export const useDownloadStore = defineStore('download', {
    state: () => ({
        error: '',
        done: '',
        busy: false,
    }),

    actions: {
        clear() {
            this.error = ''
            this.done = ''
        },

        /**
         * @param {'spans'|'pending'|'quota'} kind
         * @returns {Promise<string|null>} 저장한 경로. 교사가 취소하면 null.
         */
        async csv(kind, args, suggested) {
            this.clear()
            this.busy = true
            try {
                const path = await exportCsv(kind, args, suggested)
                // 취소는 실패가 아니다. 아무 말도 하지 않는다.
                this.done = path ? `저장했습니다 — ${path}` : ''
                return path
            } catch (e) {
                this.error = `저장하지 못했습니다: ${e}`
                throw e
            } finally {
                this.busy = false
            }
        },

        /** 이미 만들어 둔 바이트를 파일로. 명렬표 양식이 이것을 쓴다. */
        async saveBytes(path, base64) {
            this.clear()
            try {
                await invoke('write_bytes_file', {path, data: base64})
            } catch (e) {
                this.error = `저장하지 못했습니다: ${e}`
                throw e
            }
        },
    },
})
