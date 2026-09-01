import {defineStore} from 'pinia'
import {ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

/**
 * 학생 명단. 붙여넣기·CSV·직접 입력이 하나의 미리보기로 수렴한다.
 *
 * 재가져오기는 교체가 아니라 차분이다. 미리보기의 action을 교사가 바꿀 수 있고,
 * 저장은 그 확정본만 반영한다.
 */
export const useRosterStore = defineStore('roster', () => {
    const students = ref([])
    const diff = ref([])
    const loading = ref(false)
    const error = ref('')

    async function fetchStudents(yearId, grade, classNo) {
        loading.value = true
        error.value = ''
        try {
            students.value = await invoke('get_students', {yearId, grade, classNo})
        } catch (e) {
            error.value = String(e)
            throw e
        } finally {
            loading.value = false
        }
    }

    async function parseText(text) {
        return await invoke('parse_roster', {text})
    }

    async function preview(yearId, grade, classNo, entries) {
        error.value = ''
        try {
            diff.value = await invoke('preview_roster', {yearId, grade, classNo, entries})
            return diff.value
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function apply(yearId, grade, classNo, effectiveDate, rows) {
        const result = await invoke('apply_roster', {
            yearId, grade, classNo, effectiveDate, rows,
        })
        diff.value = []
        await fetchStudents(yearId, grade, classNo)
        return result
    }

    async function updateStudent(id, number, name, guardianPhone) {
        await invoke('update_student', {id, number, name, guardianPhone})
    }

    async function withdraw(id, date) {
        await invoke('withdraw_student', {id, date})
    }

    return {
        students, diff, loading, error,
        fetchStudents, parseText, preview, apply, updateStudent, withdraw,
    }
})
