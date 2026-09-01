<script setup>
/**
 * 학생 명단.
 *
 * **학급은 명렬표 파일에서 나온다.** 머리글이 (학년, 반, 번호, 이름)이므로
 * "우리 반이 몇 학년 몇 반입니까"를 따로 묻지 않는다. 학년·반 열이 없거나
 * 여러 반이 섞여 있을 때만 교사에게 되묻는다.
 *
 * 재가져오기는 교체가 아니라 차분이고, 판정할 수 없는 행(번호 같고 이름 다름)은
 * 교사가 미리보기에서 고른다.
 */
import {computed, onMounted, ref} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useRosterStore} from '../stores/roster'
import RosterImport from '../components/RosterImport.vue'
import StudentPanel from '../components/StudentPanel.vue'
import {UiButton, UiCard, UiNotice, UiPage, UiTable} from '../components/ui'

const app = useAppStore()
const roster = useRosterStore()
const router = useRouter()

const effectiveDate = ref(new Date().toISOString().slice(0, 10))
const rows = ref([])
const entries = ref([])
const detected = ref(null)
const grade = ref(app.grade ?? 1)
const classNo = ref(app.classNo ?? 1)
const selectedId = ref(null)
const contacts = ref([])
const message = ref('')
const error = ref('')

const ACTIONS = [
    {value: 'added', label: '추가'},
    {value: 'unchanged', label: '변경 없음'},
    {value: 'renamed', label: '이름 변경'},
    {value: 'withdrawn', label: '전출'},
]

const DIFF_COLUMNS = [
    {key: 'number', label: '번호', width: '80px'},
    {key: 'current', label: '현재'},
    {key: 'incoming', label: '명단'},
    {key: 'action', label: '처리', width: '170px'},
]

const LIST_COLUMNS = [
    {key: 'number', label: '번호', width: '80px'},
    {key: 'name', label: '성명', width: '160px'},
    {key: 'actions', label: '', width: '200px', align: 'right'},
]

const changed = computed(() => rows.value.filter((r) => r.action !== 'unchanged'))
const needsClass = computed(() => !detected.value?.grade || !detected.value?.classNo)
const selectedRow = computed(() => {
    const s = roster.students.find((x) => x.id === selectedId.value)
    if (!s) return null
    // StudentPanel은 격자 행 모양을 기대한다. 명단 화면에는 그날 기록이 없으므로 비운다.
    return {studentId: s.id, number: s.number, name: s.name, spans: [], reason: null, checks: []}
})

async function load() {
    if (!app.ready) return
    await roster.fetchStudents(app.yearId, app.grade, app.classNo)
    grade.value = app.grade
    classNo.value = app.classNo
}

/** 파일에서 읽은 목록을 받아 학급을 확인하고 차분을 만든다. */
async function onLoaded(result) {
    error.value = ''
    message.value = ''
    entries.value = result.entries
    try {
        detected.value = await roster.detectClass(result.entries)
        if (detected.value.grade) grade.value = detected.value.grade
        if (detected.value.classNo) classNo.value = detected.value.classNo
        await makePreview()
    } catch (e) {
        error.value = String(e)
    }
}

/** 교사가 학급을 고쳤을 때 같은 목록으로 다시 비교한다. */
async function makePreview() {
    if (!entries.value.length) return
    error.value = ''
    try {
        rows.value = await roster.preview(
            app.yearId, Number(grade.value), Number(classNo.value), entries.value,
        )
    } catch (e) {
        error.value = String(e)
    }
}

async function applyPreview() {
    error.value = ''
    try {
        const g = Number(grade.value)
        const c = Number(classNo.value)
        const result = await roster.apply(app.yearId, g, c, effectiveDate.value, rows.value)
        await app.setContext(g, c)
        rows.value = []
        entries.value = []
        detected.value = null
        await load()
        message.value =
            `추가 ${result.added}명 · 이름 변경 ${result.renamed}명 · 전출 ${result.withdrawn}명`
    } catch (e) {
        error.value = String(e)
    }
}

async function switchClass([g, c]) {
    await app.setContext(g, c)
    selectedId.value = null
    await load()
}

async function selectStudent(student) {
    selectedId.value = student.id
    contacts.value = await roster.fetchContacts(student.id)
}

async function saveContacts(list) {
    try {
        await roster.saveContacts(selectedId.value, list)
        contacts.value = await roster.fetchContacts(selectedId.value)
        message.value = '연락처를 저장했습니다.'
    } catch (e) {
        error.value = String(e)
    }
}

async function withdrawOne(student) {
    error.value = ''
    try {
        await roster.withdraw(student.id, effectiveDate.value)
        if (selectedId.value === student.id) selectedId.value = null
        await load()
    } catch (e) {
        error.value = String(e)
    }
}

onMounted(load)
</script>

<template>
    <UiPage subtitle="엑셀·CSV 명렬표 파일을 넣으면 됩니다." title="학생 명단">
        <template #actions>
            <UiButton v-if="app.ready" variant="primary" @click="router.push('/')">
                HOME으로
            </UiButton>
        </template>

        <!-- 학급 전환 -->
        <UiCard v-if="app.classes.length > 1" title="학급">
            <div class="row">
                <UiButton v-for="[g, c] in app.classes" :key="`${g}-${c}`"
                          :variant="app.grade === g && app.classNo === c ? 'primary' : 'default'"
                          @click="switchClass([g, c])">
                    {{ g }}학년 {{ c }}반
                </UiButton>
            </div>
        </UiCard>

        <!-- 파일 가져오기 -->
        <UiCard description="엑셀이나 CSV 파일을 그대로 넣습니다. (학년, 반, 번호, 이름) 네 열이면 학급도 함께 읽습니다."
                title="명렬표 가져오기">
            <RosterImport @loaded="onLoaded"/>
            <div class="row">
                <label class="muted">적용 기준일</label>
                <input v-model="effectiveDate" class="field" type="date"/>
            </div>
        </UiCard>

        <!-- 미리보기 -->
        <UiCard v-if="rows.length" :title="`미리보기 — 바꿀 것 ${changed.length}건`">
            <UiNotice v-if="detected?.mixed" kind="warn"
                      text="명렬표에 여러 학급이 섞여 있습니다. 어느 반으로 넣을지 정해주세요."/>
            <UiNotice v-else-if="needsClass" kind="info"
                      text="명렬표에 학년·반이 없습니다. 아래에서 정해주세요."/>
            <UiNotice v-else kind="ok"
                      :text="`명렬표가 말하는 학급: ${grade}학년 ${classNo}반`"/>

            <div class="row">
                <input v-model="grade" class="field num" min="1" type="number"/>
                <span>학년</span>
                <input v-model="classNo" class="field num" min="1" type="number"/>
                <span>반</span>
                <UiButton @click="makePreview">이 학급으로 다시 비교</UiButton>
            </div>

            <p class="muted">
                사라진 번호는 삭제하지 않고 전출로 처리합니다. 번호가 같고 이름이 다른 행은
                개명인지 전입인지 프로그램이 판정할 수 없으니 직접 골라주세요.
            </p>

            <UiTable :columns="DIFF_COLUMNS" :rows="rows" row-key="number">
                <template #row="{row}">
                    <td>{{ row.number }}</td>
                    <td>{{ row.currentName ?? '—' }}</td>
                    <td>{{ row.incomingName ?? '—' }}</td>
                    <td>
                        <select v-model="row.action" class="field">
                            <option v-for="a in ACTIONS" :key="a.value" :value="a.value">
                                {{ a.label }}
                            </option>
                        </select>
                    </td>
                </template>
            </UiTable>

            <div class="row">
                <UiButton variant="primary" @click="applyPreview">저장</UiButton>
                <UiButton @click="rows = []; entries = []; detected = null">취소</UiButton>
            </div>
        </UiCard>

        <!-- 현재 명단 + 학생 패널 -->
        <div v-if="app.ready" class="board">
            <UiCard class="board__list" :title="`재학생 ${roster.students.length}명`">
                <UiTable :columns="LIST_COLUMNS" :rows="roster.students"
                         empty-text="아직 명단이 없습니다. 위에 붙여넣어 주세요.">
                    <template #row="{row}">
                        <td>{{ row.number }}</td>
                        <td>{{ row.name }}</td>
                        <td class="cell-right">
                            <UiButton variant="ghost" @click="selectStudent(row)">연락처</UiButton>
                            <UiButton variant="danger" @click="withdrawOne(row)">전출</UiButton>
                        </td>
                    </template>
                </UiTable>
            </UiCard>

            <StudentPanel :contacts="contacts" :describe="() => ''" :items="[]"
                          :row="selectedRow"
                          @close="selectedId = null"
                          @save-contacts="saveContacts"/>
        </div>

        <UiNotice :text="message" kind="ok"/>
        <UiNotice :text="error" kind="error"/>
    </UiPage>
</template>

<style scoped>
.row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
}

.num {
    width: 90px;
}

.muted {
    color: var(--c-ink-3);
    margin: 0;
}

.cell-right {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
}

.board {
    display: flex;
    gap: 16px;
    align-items: flex-start;
}

.board__list {
    flex: 1;
    min-width: 0;
}
</style>
