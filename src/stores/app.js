import {defineStore} from 'pinia'
import {computed, ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

const K_YEAR = 'current_year_id'
const K_GRADE = 'current_grade'
const K_CLASS = 'current_class_no'

/**
 * 앱 전역 상태 — DB 열기, 학년도/학급 선택.
 *
 * 컴포넌트는 invoke를 직접 부르지 않는다. 전부 이 계층을 거친다.
 */
export const useAppStore = defineStore('app', () => {
    const status = ref(null)
    const years = ref([])
    const yearId = ref(null)
    const grade = ref(null)
    const classNo = ref(null)
    const loading = ref(false)
    const error = ref('')

    const ready = computed(
        () => yearId.value !== null && grade.value !== null && classNo.value !== null,
    )
    const currentYear = computed(() => years.value.find((y) => y.id === yearId.value) || null)

    async function init() {
        loading.value = true
        error.value = ''
        try {
            status.value = await invoke('init_db')
            if (status.value.needsMigration) {
                await invoke('migrate_schema')
            }
            await fetchYears()
            await restoreContext()
        } catch (e) {
            error.value = String(e)
            // 에러를 삼키면 "데이터 없음"과 구분되지 않는 빈 화면이 그대로 보인다.
            throw e
        } finally {
            loading.value = false
        }
    }

    async function fetchYears() {
        years.value = await invoke('get_years')
    }

    async function restoreContext() {
        const [y, g, c] = await Promise.all([
            invoke('get_config', {key: K_YEAR}),
            invoke('get_config', {key: K_GRADE}),
            invoke('get_config', {key: K_CLASS}),
        ])
        const savedYear = y === null ? null : Number(y)
        yearId.value = years.value.some((it) => it.id === savedYear) ? savedYear : null
        grade.value = g === null ? null : Number(g)
        classNo.value = c === null ? null : Number(c)
    }

    async function setContext(nextYearId, nextGrade, nextClassNo) {
        yearId.value = nextYearId
        grade.value = nextGrade
        classNo.value = nextClassNo
        await Promise.all([
            invoke('set_config', {key: K_YEAR, value: String(nextYearId)}),
            invoke('set_config', {key: K_GRADE, value: String(nextGrade)}),
            invoke('set_config', {key: K_CLASS, value: String(nextClassNo)}),
        ])
    }

    async function createYear(year, startsOn, endsOn) {
        const id = await invoke('create_year', {year, startsOn, endsOn})
        await fetchYears()
        return id
    }

    async function backupTo(dest) {
        return await invoke('export_backup', {dest})
    }

    return {
        status, years, yearId, grade, classNo, loading, error,
        ready, currentYear,
        init, fetchYears, setContext, createYear, backupTo,
    }
})
