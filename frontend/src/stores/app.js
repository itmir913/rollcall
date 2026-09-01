import {defineStore} from 'pinia'
import {computed, ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

const K_YEAR = 'current_year_id'
const K_GRADE = 'current_grade'
const K_CLASS = 'current_class_no'

/**
 * 학년도는 3월에 시작한다. 1~2월에 앱을 켜면 아직 지난해 학년도다.
 * `getFullYear()`를 그대로 쓰면 2027년 2월에 "2027학년도"를 만들어 버린다.
 */
export function academicYearOf(date = new Date()) {
    return date.getMonth() + 1 >= 3 ? date.getFullYear() : date.getFullYear() - 1
}

/**
 * 앱 전역 상태 — DB 열기, 학년도, 지금 보고 있는 학급.
 *
 * **학년도를 교사에게 묻지 않는다.** 3월 기준으로 앱이 정해서 만든다. 학년도
 * 추가는 1년에 한 번 있는 일이라 첫 실행에 낼 질문이 아니고, 학급은 명렬표가
 * 알려준다.
 *
 * 컴포넌트는 invoke를 직접 부르지 않는다. 전부 이 계층을 거친다.
 */
export const useAppStore = defineStore('app', () => {
    const status = ref(null)
    const years = ref([])
    const yearId = ref(null)
    const grade = ref(null)
    const classNo = ref(null)
    const classes = ref([])
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
            if (status.value.needsMigration) await invoke('migrate_schema')
            await ensureCurrentYear()
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

    /** 올해 학년도가 없으면 만든다. 있으면 그대로 쓴다. */
    async function ensureCurrentYear() {
        await fetchYears()
        const year = academicYearOf()
        let row = years.value.find((y) => y.year === year)
        if (!row) {
            await invoke('create_year', {
                year,
                startsOn: `${year}-03-01`,
                endsOn: `${year + 1}-02-28`,
            })
            await fetchYears()
            row = years.value.find((y) => y.year === year)
        }
        yearId.value = row?.id ?? years.value[0]?.id ?? null
    }

    /**
     * 저장된 학급을 되살린다. 없으면 명단이 있는 학급 중 첫 번째를 고른다.
     * 학급은 학년도마다 다르므로 학년도별로 저장한다.
     */
    async function restoreContext() {
        if (yearId.value === null) return
        classes.value = await invoke('get_classes', {yearId: yearId.value})

        const [savedYear, g, c] = await Promise.all([
            invoke('get_config', {key: K_YEAR}),
            invoke('get_config', {key: `${K_GRADE}.${yearId.value}`}),
            invoke('get_config', {key: `${K_CLASS}.${yearId.value}`}),
        ])
        if (savedYear !== null && years.value.some((y) => y.id === Number(savedYear))) {
            yearId.value = Number(savedYear)
        }

        const saved = g !== null && c !== null ? [Number(g), Number(c)] : null
        const exists = saved && classes.value.some(([sg, sc]) => sg === saved[0] && sc === saved[1])

        if (exists) {
            grade.value = saved[0]
            classNo.value = saved[1]
        } else if (classes.value.length) {
            ;[grade.value, classNo.value] = classes.value[0]
        } else {
            grade.value = null
            classNo.value = null
        }
    }

    async function setContext(nextGrade, nextClassNo, nextYearId = yearId.value) {
        yearId.value = nextYearId
        grade.value = nextGrade
        classNo.value = nextClassNo
        await Promise.all([
            invoke('set_config', {key: K_YEAR, value: String(nextYearId)}),
            invoke('set_config', {key: `${K_GRADE}.${nextYearId}`, value: String(nextGrade)}),
            invoke('set_config', {key: `${K_CLASS}.${nextYearId}`, value: String(nextClassNo)}),
        ])
        classes.value = await invoke('get_classes', {yearId: nextYearId})
    }

    async function backupTo(dest) {
        return await invoke('export_backup', {dest})
    }

    return {
        status, years, yearId, grade, classNo, classes, loading, error,
        ready, currentYear,
        init, fetchYears, restoreContext, setContext, backupTo,
    }
})
