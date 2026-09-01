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

    /**
     * 명렬표가 말하는 학급. 파일 파싱은 프론트가 하고(services/rosterFile.js),
     * "어느 학급인가"라는 판단만 Rust에 맡긴다.
     */
    async function detectClass(entries) {
        return await invoke('detect_roster_class', {entries})
    }

    async function fetchClasses(yearId) {
        return await invoke('get_classes', {yearId})
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

    async function updateStudent(id, number, name) {
        await invoke('update_student', {id, number, name})
    }

    async function fetchContacts(studentId) {
        return await invoke('get_contacts', {studentId})
    }

    /** 한 학생의 연락처를 통째로 바꾼다. 화면이 목록 전체를 편집하기 때문이다. */
    async function saveContacts(studentId, contacts) {
        await invoke('set_contacts', {studentId, contacts})
    }

    async function withdraw(id, date) {
        await invoke('withdraw_student', {id, date})
    }

    return {
        students, diff, loading, error,
        fetchStudents, detectClass, fetchClasses, preview, apply, updateStudent, withdraw,
        fetchContacts, saveContacts,
    }
})
