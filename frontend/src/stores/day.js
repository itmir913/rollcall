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
    /** 두 축 중 하나라도 비어 있는 구간을 가진 학생 수 */
    const incompleteCount = computed(
        () => rows.value.filter((r) => r.spans.some((s) => !s.complete)).length,
    )

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

    /**
     * 여러 학생에게 같은 출결을 한 번에 찍는다.
     *
     * 화면의 입력 방식이 이렇다 — 구분과 종류를 고른 뒤 학생을 눌러 나간다.
     * 한 명을 눌러도 목록 하나짜리로 같은 경로를 탄다. 두 축이 비어 있어도
     * 저장된다("안 왔는데 연락이 안 됨").
     */
    async function stamp({studentIds, reasonId, typeId, startSlot, endSlot, symptom}) {
        return await invoke('add_spans', {
            studentIds, date: date.value, reasonId, typeId, startSlot, endSlot, symptom,
        })
    }

    async function updateSpan({id, reasonId, typeId, startSlot, endSlot, symptom}) {
        await invoke('update_span', {id, reasonId, typeId, startSlot, endSlot, symptom})
    }

    async function deleteSpan(id) {
        await invoke('delete_span', {id})
    }

    async function setReason(studentId, reason, reasonId = null, typeId = null) {
        await invoke('set_daily_reason', {
            studentId, date: date.value, reasonId, typeId, reason,
        })
    }

    /** 연속 결석 학생은 이것 하나로 끝난다. */
    async function copyPrevious(studentId) {
        return await invoke('copy_previous', {studentId, date: date.value})
    }

    async function renderPhrase(reasonId, typeId, symptom, startSlot, endSlot) {
        return await invoke('render_phrase', {
            reasonId, typeId, symptom, startSlot, endSlot, onDate: date.value,
        })
    }

    async function bulkPreview(studentIds, from, to) {
        return await invoke('bulk_preview', {studentIds, from, to})
    }

    async function bulkApply(studentIds, dates, reasonId, typeId, startSlot, endSlot, symptom) {
        return await invoke('bulk_apply', {
            studentIds, dates, reasonId, typeId, startSlot, endSlot, symptom,
        })
    }

    /** 아직 두 축이 다 채워지지 않은 구간들. 채워 넣어야 할 목록이다. */
    async function fetchIncomplete(yearId, grade, classNo) {
        return await invoke('get_incomplete', {yearId, grade, classNo})
    }

    return {
        date, grid, loading, error,
        rows, items, dateLabel, recordedCount, incompleteCount,
        fetchGrid, setDate, moveDate, rowByNumber,
        stamp, updateSpan, deleteSpan, setReason, copyPrevious, renderPhrase,
        bulkPreview, bulkApply, fetchIncomplete,
    }
})
