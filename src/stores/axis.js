import {defineStore} from 'pinia'
import {computed, ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

/**
 * 출결의 두 축(구분·종류)과 그 쌍인 코드.
 *
 * 화면은 이 둘을 각각 고른다. 한쪽만 고른 상태도 저장 가능하므로, 스토어도
 * "선택된 코드" 하나가 아니라 두 개의 선택을 들고 있다.
 */
export const useAxisStore = defineStore('axis', () => {
    const reasons = ref([])
    const types = ref([])
    const codes = ref([])
    const error = ref('')

    async function fetchAll(onDate = null) {
        error.value = ''
        try {
            const [r, t, c] = await Promise.all([
                invoke('get_reasons', {onDate}),
                invoke('get_types', {onDate}),
                invoke('get_codes', {onDate}),
            ])
            reasons.value = r
            types.value = t
            codes.value = c
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    /** 두 축으로 쌍을 찾는다. 한쪽이라도 비면 null — 그것이 정상이다. */
    function findCode(reasonId, typeId) {
        if (reasonId === null || typeId === null) return null
        return codes.value.find((c) => c.reasonId === reasonId && c.typeId === typeId) ?? null
    }

    function reasonById(id) {
        return reasons.value.find((r) => r.id === id) ?? null
    }

    function typeById(id) {
        return types.value.find((t) => t.id === id) ?? null
    }

    /** 두 축이 고른 조합의 이름. 미정이면 그렇게 말한다. */
    function describe(reasonId, typeId) {
        const code = findCode(reasonId, typeId)
        if (code) return code.label
        const r = reasonById(reasonId)?.label
        const t = typeById(typeId)?.label
        if (r && !t) return `${r} · 종류 미정`
        if (!r && t) return `구분 미정 · ${t}`
        return '미정'
    }

    const ready = computed(() => reasons.value.length > 0 && types.value.length > 0)

    async function suggestSymptoms(prefix, limit = 8) {
        return await invoke('get_symptom_suggestions', {prefix, limit})
    }

    async function reviseCode(payload) {
        const id = await invoke('revise_code', payload)
        await fetchAll()
        return id
    }

    async function retireCode(id, validTo) {
        await invoke('retire_code', {id, validTo})
        await fetchAll()
    }

    return {
        reasons, types, codes, error, ready,
        fetchAll, findCode, reasonById, typeById, describe,
        suggestSymptoms, reviseCode, retireCode,
    }
})
