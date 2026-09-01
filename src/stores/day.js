import {defineStore} from 'pinia'
import {computed, ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

function todayIso() {
    const d = new Date()
    const p = (n) => String(n).padStart(2, '0')
    return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`
}

const WEEKDAY = ['일', '월', '화', '수', '목', '금', '토']

export function formatKorean(iso) {
    const [y, m, d] = iso.split('-').map(Number)
    const day = new Date(y, m - 1, d)
    return `${y}.${String(m).padStart(2, '0')}.${String(d).padStart(2, '0')}.(${WEEKDAY[day.getDay()]})`
}

export function shiftDate(iso, days) {
    const [y, m, d] = iso.split('-').map(Number)
    const next = new Date(y, m - 1, d + days)
    const p = (n) => String(n).padStart(2, '0')
    return `${next.getFullYear()}-${p(next.getMonth() + 1)}-${p(next.getDate())}`
}

/** 하루치 격자. 저장은 전부 자동이다 — 확인 대화상자도, 저장 버튼도 없다. */
export const useDayStore = defineStore('day', () => {
    const date = ref(todayIso())
    const grid = ref(null)
    const loading = ref(false)
    const error = ref('')

    const rows = computed(() => grid.value?.rows ?? [])
    const items = computed(() => grid.value?.items ?? [])
    const dateLabel = computed(() => formatKorean(date.value))
    const recordedCount = computed(() => rows.value.filter((r) => r.spans.length > 0).length)

    async function fetchGrid(yearId, grade, classNo) {
        loading.value = true
        error.value = ''
        try {
            grid.value = await invoke('get_day_grid', {
                yearId, grade, classNo, date: date.value,
            })
        } catch (e) {
            error.value = String(e)
            throw e
        } finally {
            loading.value = false
        }
    }

    function setDate(iso) {
        date.value = iso
    }

    function moveDate(days) {
        date.value = shiftDate(date.value, days)
    }

    function rowByNumber(number) {
        return rows.value.find((r) => r.number === number) || null
    }

    async function addSpan({studentId, codeId, startSlot, endSlot, symptom}) {
        return await invoke('add_span', {
            studentId, date: date.value, codeId, startSlot, endSlot, symptom,
        })
    }

    async function updateSpan({id, codeId, startSlot, endSlot, symptom}) {
        await invoke('update_span', {id, codeId, startSlot, endSlot, symptom})
    }

    async function deleteSpan(id) {
        await invoke('delete_span', {id})
    }

    async function setReason(studentId, reason, codeId = null) {
        await invoke('set_daily_reason', {studentId, date: date.value, codeId, reason})
    }

    /** 연속 결석 학생은 이것 하나로 끝난다. */
    async function copyPrevious(studentId) {
        return await invoke('copy_previous', {studentId, date: date.value})
    }

    async function renderPhrase(codeId, symptom, startSlot, endSlot) {
        return await invoke('render_phrase', {codeId, symptom, startSlot, endSlot})
    }

    async function bulkPreview(studentIds, from, to) {
        return await invoke('bulk_preview', {studentIds, from, to})
    }

    async function bulkApply(studentIds, dates, codeId, startSlot, endSlot, symptom) {
        return await invoke('bulk_apply', {studentIds, dates, codeId, startSlot, endSlot, symptom})
    }

    return {
        date, grid, loading, error,
        rows, items, dateLabel, recordedCount,
        fetchGrid, setDate, moveDate, rowByNumber,
        addSpan, updateSpan, deleteSpan, setReason, copyPrevious, renderPhrase,
        bulkPreview, bulkApply,
    }
})
