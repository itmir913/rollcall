<script setup>
/**
 * 학생 명단.
 *
 * 붙여넣기를 주 경로로 둔다 — 나이스에서 두 열을 복사해 붙여넣으면 끝난다.
 * 재가져오기는 교체가 아니라 차분이고, 판정할 수 없는 행(번호 같고 이름 다름)은
 * 교사가 미리보기에서 고른다.
 */
import {computed, onMounted, ref} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useRosterStore} from '../stores/roster'
import {UiButton, UiCard, UiNotice, UiPage, UiTable} from '../components/ui'

const app = useAppStore()
const roster = useRosterStore()
const router = useRouter()

const pasted = ref('')
const effectiveDate = ref(new Date().toISOString().slice(0, 10))
const rows = ref([])
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
    {key: 'name', label: '성명', width: '140px'},
    {key: 'phone', label: '보호자 연락처 (선택)'},
    {key: 'actions', label: '', width: '110px', align: 'right'},
]

const changed = computed(() => rows.value.filter((r) => r.action !== 'unchanged'))

async function load() {
    if (!app.ready) return
    await roster.fetchStudents(app.yearId, app.grade, app.classNo)
}

async function makePreview() {
    error.value = ''
    message.value = ''
    try {
        const entries = await roster.parseText(pasted.value)
        if (!entries.length) {
            error.value = '번호와 이름을 찾지 못했습니다. 두 열을 함께 복사했는지 확인해주세요.'
            return
        }
        rows.value = await roster.preview(app.yearId, app.grade, app.classNo, entries)
    } catch (e) {
        error.value = String(e)
    }
}

async function applyPreview() {
    error.value = ''
    try {
        const result = await roster.apply(
            app.yearId, app.grade, app.classNo, effectiveDate.value, rows.value,
        )
        rows.value = []
        pasted.value = ''
        message.value =
            `추가 ${result.added}명 · 이름 변경 ${result.renamed}명 · 전출 ${result.withdrawn}명`
    } catch (e) {
        error.value = String(e)
    }
}

async function withdrawOne(student) {
    error.value = ''
    try {
        await roster.withdraw(student.id, effectiveDate.value)
        await load()
    } catch (e) {
        error.value = String(e)
    }
}

async function savePhone(student, event) {
    try {
        await roster.updateStudent(
            student.id, student.number, student.name, event.target.value.trim() || null,
        )
    } catch (e) {
        error.value = String(e)
    }
}

onMounted(load)
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn" text="먼저 설정에서 학년도와 학급을 정해주세요."/>

    <UiPage v-else title="학생 명단">
        <template #actions>
            <UiButton variant="primary" @click="router.push('/')">HOME으로</UiButton>
        </template>

        <UiCard description="나이스에서 번호·성명 두 열을 복사해 그대로 붙여넣으세요. 탭이든 쉼표든 상관없습니다."
                title="명렬표 붙여넣기">
            <textarea v-model="pasted" class="field roster__paste"
                      placeholder="1&#9;김철수&#10;2&#9;이영희"></textarea>
            <div class="roster__row">
                <UiButton variant="primary" @click="makePreview">미리보기</UiButton>
                <label class="roster__label">적용 기준일</label>
                <input v-model="effectiveDate" class="field" type="date"/>
            </div>
        </UiCard>

        <UiCard v-if="rows.length"
                :title="`미리보기 — 바꿀 것 ${changed.length}건`"
                description="사라진 번호는 삭제하지 않고 전출로 처리합니다. 번호가 같고 이름이 다른 행은 개명인지 전입인지 프로그램이 판정할 수 없으니 직접 골라주세요.">
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
            <div class="roster__row">
                <UiButton variant="primary" @click="applyPreview">저장</UiButton>
                <UiButton @click="rows = []">취소</UiButton>
            </div>
        </UiCard>

        <UiCard :title="`재학생 ${roster.students.length}명`">
            <UiTable :columns="LIST_COLUMNS" :rows="roster.students"
                     empty-text="아직 명단이 없습니다. 위에 붙여넣어 주세요.">
                <template #row="{row}">
                    <td>{{ row.number }}</td>
                    <td>{{ row.name }}</td>
                    <td>
                        <input :value="row.guardianPhone" class="field roster__phone"
                               placeholder="비워 두어도 됩니다" @blur="savePhone(row, $event)"/>
                    </td>
                    <td class="roster__cell-right">
                        <UiButton variant="danger" @click="withdrawOne(row)">전출</UiButton>
                    </td>
                </template>
            </UiTable>
        </UiCard>

        <UiNotice :text="message" kind="ok"/>
        <UiNotice :text="error" kind="error"/>
    </UiPage>
</template>

<style scoped>
.roster__paste {
    width: 100%;
    min-height: 150px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    resize: vertical;
}

.roster__row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
}

.roster__label {
    color: var(--c-ink-3);
}

.roster__phone {
    width: 220px;
}

.roster__cell-right {
    text-align: right;
}
</style>
