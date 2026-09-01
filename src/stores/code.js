import {defineStore} from 'pinia'
import {ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

/** 출결 코드. 수정은 마감 후 추가이므로 update가 아니라 revise다. */
export const useCodeStore = defineStore('code', () => {
    const codes = ref([])
    const error = ref('')

    async function fetchCodes(onDate = null) {
        error.value = ''
        try {
            codes.value = await invoke('get_codes', {onDate})
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function createCode(payload) {
        const id = await invoke('create_code', payload)
        await fetchCodes()
        return id
    }

    /** 구 행을 마감하고 새 행을 만든다. 과거 기록의 뜻이 바뀌지 않게 하려는 것이다. */
    async function reviseCode(payload) {
        const id = await invoke('revise_code', payload)
        await fetchCodes()
        return id
    }

    async function retireCode(id, validTo) {
        await invoke('retire_code', {id, validTo})
        await fetchCodes()
    }

    async function suggestSymptoms(prefix, limit = 8) {
        return await invoke('get_symptom_suggestions', {prefix, limit})
    }

    return {codes, error, fetchCodes, createCode, reviseCode, retireCode, suggestSymptoms}
})
