<script setup>
/**
 * HOME — 일일 입력 격자. **마우스로 조작한다.**
 *
 *  · 행의 [기록 추가]를 누르면 그 행 아래에 편집기가 펼쳐진다
 *  · 코드도 교시도 버튼이다. 드롭다운을 쓰지 않는다
 *  · 타이핑이 필요한 곳은 증상 한 칸뿐이고, 과거에 쓴 단어는 후보 버튼으로 뜬다
 *  · 저장은 편집기의 [저장] 한 번. 확인 대화상자는 없다
 *  · 전체 행을 항상 띄운다. 빈 행은 출석이고 저장되지 않는다
 *
 * 키보드 단축 입력은 배포 직전에 따로 정한다. 지금은 없다.
 */
import {computed, onMounted, ref} from 'vue'
import {useAppStore} from '../stores/app'
import {useCodeStore} from '../stores/code'
import {useDayStore} from '../stores/day'
import {useCheckStore} from '../stores/check'
import SpanEditor from '../components/SpanEditor.vue'
import {UiButton, UiCard, UiNotice, UiPage, UiToggle} from '../components/ui'

const app = useAppStore()
const day = useDayStore()
const codeStore = useCodeStore()
const check = useCheckStore()

const message = ref('')
const messageKind = ref('ok')

/** 지금 편집 중인 자리. { studentId, spanId | null } */
const editing = ref(null)

const editingRow = computed(() =>
    editing.value ? day.rows.find((r) => r.studentId === editing.value.studentId) ?? null : null,
)

const editingSpan = computed(() => {
    if (!editing.value?.spanId || !editingRow.value) return null
    return editingRow.value.spans.find((s) => s.id === editing.value.spanId) ?? null
})

function say(text, kind = 'ok') {
    message.value = text
    messageKind.value = kind
}

async function refresh() {
    if (!app.ready) return
    await Promise.all([
        day.fetchGrid(app.yearId, app.grade, app.classNo),
        codeStore.fetchCodes(day.date),
        check.fetchSummary(app.yearId, app.grade, app.classNo),
    ])
}

function isEditing(row, spanId = null) {
    return editing.value?.studentId === row.studentId && editing.value?.spanId === spanId
}

function openNew(row) {
    editing.value = {studentId: row.studentId, spanId: null}
}

function openSpan(row, span) {
    editing.value = {studentId: row.studentId, spanId: span.id}
}

function closeEditor() {
    editing.value = null
}

async function saveSpan(payload) {
    const row = editingRow.value
    const span = editingSpan.value
    if (!row) return
    try {
        if (span) {
            await day.updateSpan({id: span.id, ...payload})
            say(`${row.name} — 구간을 고쳤습니다.`)
        } else {
            await day.addSpan({studentId: row.studentId, ...payload})
            say(`${row.number}번 ${row.name} — 저장했습니다.`)
        }
        closeEditor()
        await refresh()
    } catch (e) {
        say(String(e), 'error')
    }
}

async function removeSpan() {
    const span = editingSpan.value
    if (!span) return
    try {
        await day.deleteSpan(span.id)
        closeEditor()
        await refresh()
        say('구간을 지웠습니다.')
    } catch (e) {
        say(String(e), 'error')
    }
}

async function repeatPrevious(row) {
    try {
        const n = await day.copyPrevious(row.studentId)
        await refresh()
        say(`${row.name} — 직전 기록 ${n}건을 그대로 넣었습니다.`)
    } catch (e) {
        say(String(e), 'error')
    }
}

async function toggleCheck(row, item, next) {
    try {
        await check.setCheck(row.studentId, day.date, item.id, next)
        await refresh()
    } catch (e) {
        say(String(e), 'error')
    }
}

async function saveReason(row, event) {
    const text = event.target.value.trim()
    if (!text || text === row.reason?.reason) return
    try {
        await day.setReason(row.studentId, text, row.reason?.codeId ?? null)
        await refresh()
    } catch (e) {
        say(String(e), 'error')
    }
}

function checkState(row, itemId) {
    return row.checks.find((c) => c.itemId === itemId) ?? null
}

function dueHint(state) {
    return state?.dueDate ? `마감 ${state.dueDate}` : '마감 없음'
}

async function move(days) {
    closeEditor()
    day.moveDate(days)
    await refresh()
}

async function pickDate(event) {
    closeEditor()
    day.setDate(event.target.value)
    await refresh()
}

onMounted(refresh)
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn" text="먼저 설정에서 학년도와 학급을 정해주세요."/>

    <UiPage v-else :subtitle="`기록 ${day.recordedCount}명 / 재학 ${day.rows.length}명`"
            :title="day.dateLabel">
        <template #actions>
            <UiButton @click="move(-1)">◀ 어제</UiButton>
            <input :value="day.date" class="field" type="date" @change="pickDate"/>
            <UiButton @click="move(1)">내일 ▶</UiButton>
        </template>

        <UiCard>
            <table class="grid">
                <thead>
                <tr>
                    <th class="grid__num">번호</th>
                    <th class="grid__name">성명</th>
                    <th>출결</th>
                    <th class="grid__reason">나이스 사유</th>
                    <th v-for="item in day.items" :key="item.id" class="grid__check">
                        {{ item.name }}
                    </th>
                </tr>
                </thead>
                <tbody>
                <template v-for="row in day.rows" :key="row.studentId">
                    <tr :class="row.spans.length ? 'is-recorded' : ''">
                        <td class="grid__num">{{ row.number }}</td>
                        <td class="grid__name">{{ row.name }}</td>

                        <td>
                            <div class="marks">
                                <button v-for="span in row.spans" :key="span.id"
                                        :class="['mark', isEditing(row, span.id) ? 'is-open' : '']"
                                        title="눌러서 고치기"
                                        type="button"
                                        @click="openSpan(row, span)">
                                    <span class="mark__span">{{ span.spanText }}</span>
                                    <span class="mark__code">{{ span.codeLabel }}</span>
                                    <span v-if="span.symptom" class="mark__symptom">
                                        {{ span.symptom }}
                                    </span>
                                </button>

                                <button v-if="!row.spans.length && !isEditing(row)"
                                        class="add" type="button" @click="openNew(row)">
                                    출석 · 눌러서 기록 추가
                                </button>
                                <button v-else-if="!isEditing(row)" class="add is-small"
                                        title="구간을 하나 더 추가" type="button"
                                        @click="openNew(row)">
                                    + 구간
                                </button>

                                <UiButton v-if="!row.spans.length" variant="ghost"
                                          @click="repeatPrevious(row)">
                                    어제 것 그대로
                                </UiButton>
                            </div>
                        </td>

                        <td class="grid__reason">
                            <input v-if="row.reason" :value="row.reason.reason"
                                   class="field grid__input"
                                   @blur="saveReason(row, $event)"
                                   @keydown.enter="$event.target.blur()"/>
                        </td>

                        <td v-for="item in day.items" :key="item.id" class="grid__check">
                            <UiToggle v-if="checkState(row, item.id)"
                                      :hint="dueHint(checkState(row, item.id))"
                                      :model-value="checkState(row, item.id).done"
                                      block off-label="미완료" on-label="완료"
                                      @update:model-value="toggleCheck(row, item, $event)"/>
                        </td>
                    </tr>

                    <tr v-if="editing?.studentId === row.studentId">
                        <td :colspan="4 + day.items.length" class="grid__editor">
                            <SpanEditor :key="`${row.studentId}-${editing.spanId ?? 'new'}`"
                                        :codes="codeStore.codes"
                                        :render-phrase="day.renderPhrase"
                                        :span="editingSpan"
                                        :student-name="`${row.number}번 ${row.name}`"
                                        :suggest="codeStore.suggestSymptoms"
                                        @cancel="closeEditor"
                                        @remove="removeSpan"
                                        @save="saveSpan"/>
                        </td>
                    </tr>
                </template>

                <tr v-if="!day.rows.length">
                    <td :colspan="4 + day.items.length" class="grid__empty">
                        이 날짜에 재학 중인 학생이 없습니다.
                    </td>
                </tr>
                </tbody>
            </table>
        </UiCard>

        <div class="summary">
            <span v-for="s in check.summary" :key="s.itemId">
                {{ s.itemName }} 미완료 <b>{{ s.count }}</b>건
            </span>
            <RouterLink class="summary__link" to="/pending">미제출자 목록 →</RouterLink>
        </div>

        <UiNotice :kind="messageKind" :text="message"/>
    </UiPage>
</template>

<style scoped>
.grid {
    width: 100%;
    border-collapse: collapse;
}

.grid th {
    padding: 10px 12px;
    color: var(--c-ink-3);
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
}

.grid td {
    padding: 8px 12px;
    border-top: 1px solid var(--c-line);
    vertical-align: middle;
}

.grid__num {
    width: 64px;
}

.grid__name {
    width: 110px;
}

.grid__reason {
    width: 30%;
}

.grid__check {
    width: 128px;
}

.grid__input {
    width: 100%;
}

.grid__editor {
    padding: 4px 12px 14px;
}

.grid__empty {
    text-align: center;
    color: var(--c-ink-3);
    padding: 28px 12px;
}

tr.is-recorded .grid__num,
tr.is-recorded .grid__name {
    font-weight: 600;
}

.marks {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
}

.mark {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 40px;
    padding: 0 12px;
    border-radius: 8px;
    border: 1px solid var(--c-line);
    background: var(--c-surface);
    color: var(--c-ink);
    cursor: pointer;
}

.mark:hover,
.mark.is-open {
    border-color: var(--c-accent);
}

.mark__span {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: var(--c-ink-3);
}

.mark__code {
    font-weight: 600;
}

.mark__symptom {
    color: var(--c-ink-3);
}

.add {
    min-height: 40px;
    padding: 0 14px;
    border-radius: 8px;
    border: 1px dashed var(--c-line);
    background: transparent;
    color: var(--c-ink-3);
    cursor: pointer;
}

.add:hover {
    border-color: var(--c-accent);
    color: var(--c-ink);
}

.add.is-small {
    padding: 0 10px;
}

.summary {
    display: flex;
    align-items: center;
    gap: 20px;
    color: var(--c-ink-2);
}

.summary__link {
    margin-left: auto;
    color: var(--c-accent);
    text-decoration: none;
}
</style>
