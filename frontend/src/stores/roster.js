import {defineStore} from 'pinia'
import {ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'

/**
 * 학생 명단. 파일에서 읽은 것이 하나의 미리보기로 수렴한다.
 *
 * 명렬표를 다시 여는 것은 교체가 아니라 **차분**이다. 명단에서 빠진 번호는 삭제하지 않고 나간 날만
 * 적는다(`class_member.left_on`) — **내 명단에서 빠지는 것과 학교를 떠나는 것
 * (`student.enrolled_to`)은 다른 일이다.** 미리보기의 action을 교사가 바꿀 수 있고,
 * 저장은 그 확정본만 반영한다.
 *
 * **명단은 학급에 종속된다.** 학년 · 반 · 번호는 그 학생의 학적이지 내 명단의 소속이
 * 아니다 — 교과 강좌는 여러 반에서 모이므로 반으로 걸러낼 수 없고, 담임 명렬표에도
 * 반이 다른 학생이 들어오는 날이 있다. 그래서 질의가 받는 것은 `classId` 하나다.
 *
 * **줄을 학생에 맞추는 열쇠는 학급의 역할이 결정한다.** 담임은 번호 하나이고, 교과
 * 강좌는 (학년, 반, 번호) 학적 자리 전체다 — 선택과목은 1반부터 n반까지 모여 번호
 * 하나로는 학생을 구별할 수 없다. 그 판단은 전부 Rust가 하고, 여기서는 파일에서 읽은
 * 줄을 그대로 넘긴다.
 *
 * 학생 자체는 여전히 **학교에 소속된다.** 이름을 고치거나 학적을 마감하는 것은
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

    /**
     * 그 학급의 재학 명단. **읽은 것을 돌려주기도 한다** — 명렬표 화면이 학급마다
     * 하나씩 뜰 수 있어(첫 실행이 학급 여럿에 차례로 넣는다), 공유하는 `students`
     * 하나만 보면 나중에 읽은 학급의 명단이 앞의 화면까지 덮는다.
     */
    async function fetchStudents(classId) {
        loading.value = true
        error.value = ''
        try {
            students.value = await invoke('get_students', {classId})
            return students.value
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
     * 교사가 고른 학급이 결정한다 — 파일이 화면의 범위를 바꾸지 않는다.
     *
     * **교과 강좌에는 묻지 않는다.** 강좌는 여러 반에서 모이므로 `mixed`가 정상이고,
     * 그 값을 읽는 자리가 생기면 교과에 늘 붙어 있는 경고가 된다. 부르는 쪽이 결정한다.
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

    /**
     * 차분 미리보기. 줄은 `{grade, classNo, number, name, line}`이고, `line`은 파일의
     * 몇 번째 줄인가다 — 교과에서 `4번 김하늘`이 두 줄 나란히 표시되면 그 값이 없이는
     * 어느 줄을 고칠지 말하지 못한다. 돌려받는 줄의 `key`가 화면 목록의 열쇠다.
     */
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

    /**
     * 확정본을 저장한다. 돌려받는 것은
     * `{added, created, renamed, withdrawn, blocked, seatClosed}`이고,
     * `seatClosed`만 되돌릴 수 없는 쓰기다 — 화면이 그것을 경고 위계로 말한다.
     */
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
