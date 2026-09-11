import {defineStore} from 'pinia'
import {ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'

/**
 * 학생 명단. 파일에서 읽은 것이 하나의 미리보기로 수렴한다.
 *
 * 재가져오기는 교체가 아니라 **차분**이다. 사라진 번호는 삭제하지 않고 전출로 남긴다.
 * 미리보기의 action을 교사가 바꿀 수 있고, 저장은 그 확정본만 반영한다.
 *
 * 학생은 **학교에 매달린다.** 순회 교사가 학교를 둘 이상 등록해도 같은 학년·반이
 * 섞이지 않으려면 모든 질의가 school_id를 함께 넘겨야 한다.
 */
export const useRosterStore = defineStore('roster', () => {
    const students = ref([])
    const diff = ref([])
    const loading = ref(false)
    const error = ref('')

    /** 지금 보고 있는 학교. 화면이 매번 넘기지 않게 여기서 읽는다. */
    function schoolId() {
        return useAppStore().schoolId
    }

    async function fetchStudents(yearId, grade, classNo) {
        loading.value = true
        error.value = ''
        try {
            students.value = await invoke('get_students', {
                schoolId: schoolId(), yearId, grade, classNo,
            })
        } catch (e) {
            error.value = String(e)
            throw e
        } finally {
            loading.value = false
        }
    }

    /**
     * 명렬표가 말하는 학급. 파일 형식은 프런트가 읽고(services/rosterFile.js),
     * "어느 학급인가"라는 판단만 Rust에 맡긴다.
     */
    async function detectClass(entries) {
        error.value = ''
        try {
            return await invoke('detect_roster_class', {entries})
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function fetchClasses(yearId) {
        error.value = ''
        try {
            return await invoke('get_classes', {schoolId: schoolId(), yearId})
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function preview(yearId, grade, classNo, entries) {
        error.value = ''
        try {
            diff.value = await invoke('preview_roster', {
                schoolId: schoolId(), yearId, grade, classNo, entries,
            })
            return diff.value
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function apply(yearId, grade, classNo, effectiveDate, rows) {
        error.value = ''
        try {
            const result = await invoke('apply_roster', {
                schoolId: schoolId(), yearId, grade, classNo, effectiveDate, rows,
            })
            diff.value = []
            await fetchStudents(yearId, grade, classNo)
            return result
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function updateStudent(id, number, name) {
        error.value = ''
        try {
            await invoke('update_student', {schoolId: schoolId(), id, number, name})
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function fetchContacts(studentId) {
        error.value = ''
        try {
            return await invoke('get_contacts', {studentId})
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    /** 한 학생의 연락처를 통째로 바꾼다. 화면이 목록 전체를 편집하기 때문이다. */
    async function saveContacts(studentId, contacts) {
        error.value = ''
        try {
            await invoke('set_contacts', {studentId, contacts})
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function withdraw(id, date) {
        error.value = ''
        try {
            await invoke('withdraw_student', {schoolId: schoolId(), id, date})
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    return {
        students, diff, loading, error,
        fetchStudents, detectClass, fetchClasses, preview, apply, updateStudent, withdraw,
        fetchContacts, saveContacts,
    }
})
