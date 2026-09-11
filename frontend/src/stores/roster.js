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
 * **명단은 학급에 매달린다.** 학년 · 반 · 번호는 그 학생의 학적이지 내 명단의 소속이
 * 아니다 — 교과 강좌는 여러 반에서 모이므로 반으로 걸러낼 수 없고, 담임 명렬표에도
 * 반이 다른 학생이 들어오는 날이 있다. 그래서 질의가 받는 것은 `classId` 하나다.
 *
 * 학생 자체는 여전히 **학교에 매달린다.** 이름을 고치거나 전출로 마감하는 것은
 * 명단이 아니라 학적을 건드리는 일이라 학교를 함께 넘긴다.
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

    async function fetchStudents(classId) {
        loading.value = true
        error.value = ''
        try {
            students.value = await invoke('get_students', {classId})
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
     *
     * 여기서 돌려주는 학년 · 반은 **파일이 적어 둔 학적**이다. 어느 명단에 넣을지는
     * 교사가 고른 학급이 정한다 — 파일이 화면의 범위를 바꾸지 않는다.
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

    async function preview(classId, entries) {
        error.value = ''
        try {
            diff.value = await invoke('preview_roster', {classId, entries})
            return diff.value
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function apply(classId, effectiveDate, rows) {
        error.value = ''
        try {
            const result = await invoke('apply_roster', {classId, effectiveDate, rows})
            diff.value = []
            await fetchStudents(classId)
            return result
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function updateStudent(id, number, name) {
        error.value = ''
        try {
            await invoke('update_student', {classId: useAppStore().classId, id, number, name})
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
            await invoke('withdraw_student', {classId: useAppStore().classId, id, date})
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    return {
        students, diff, loading, error,
        fetchStudents, detectClass, preview, apply, updateStudent, withdraw,
        fetchContacts, saveContacts,
    }
})
