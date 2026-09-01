<script setup>
/**
 * 오늘의 출결 입력.
 *
 * 흐름: **구분을 누르고 → 종류를 누르고 → 학생을 누른다.** 학생을 누를 때마다
 * 그 조합이 한 건씩 찍힌다. 여러 명이 같은 사유면 계속 눌러 나가면 된다.
 *
 * 두 축을 "미정"으로 둔 채 학생을 눌러도 저장된다. 안 왔는데 연락이 닿지 않는
 * 학생이 그 경우이고, 나중에 채운다. 그것이 예외가 아니라 정상 경로다.
 *
 * 학생을 오래 눌러 고르는 것이 아니라 **바로 저장**한다. 실수는 오른쪽 패널에서
 * 지우면 되고, 확인 대화상자를 넣으면 스무 명 입력이 스무 번의 확인이 된다.
 */
import {computed, onMounted, ref} from 'vue'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {useDayStore} from '../stores/day'
import {useCheckStore} from '../stores/check'
import {useRosterStore} from '../stores/roster'
import AxisBar from '../components/AxisBar.vue'
import StudentPanel from '../components/StudentPanel.vue'
import {UiButton, UiCard, UiNotice, UiPage} from '../components/ui'
import {slotProblem} from '../services/slots'

const app = useAppStore()
const axis = useAxisStore()
const day = useDayStore()
const check = useCheckStore()
const roster = useRosterStore()

const reasonId = ref(null)
const typeId = ref(null)
const startSlot = ref(null)
const endSlot = ref(null)
const symptom = ref('')
const suggestions = ref([])

const selectedId = ref(null)
const contacts = ref([])
const busy = ref(false)
const message = ref('')
const messageKind = ref('ok')
/** 고치는 중인 구간. 있으면 학생 클릭 대신 이 구간을 갱신한다. */
const editingSpan = ref(null)

const type = computed(() => axis.typeById(typeId.value))
const problem = computed(() => slotProblem(type.value?.slotPrompt, startSlot.value, endSlot.value))
const stampLabel = computed(() => axis.describe(reasonId.value, typeId.value))
const selectedRow = computed(
    () => day.rows.find((r) => r.studentId === selectedId.value) ?? null,
)

function say(text, kind = 'ok') {
    message.value = text
    messageKind.value = kind
}

async function refresh() {
    if (!app.ready) return
    await Promise.all([
        day.fetchGrid(app.yearId, app.grade, app.classNo),
        axis.fetchAll(day.date),
        check.fetchSummary(app.yearId, app.grade, app.classNo),
    ])
}

/** 학생 칸을 누르면 곧바로 한 건이 찍힌다. */
async function stampOn(row) {
    if (problem.value) return say(problem.value, 'error')
    busy.value = true
    try {
        if (editingSpan.value) {
            await day.updateSpan({
                id: editingSpan.value.id,
                reasonId: reasonId.value,
                typeId: typeId.value,
                startSlot: startSlot.value,
                endSlot: endSlot.value,
                symptom: symptom.value.trim() || null,
            })
            editingSpan.value = null
            say(`${row.name} — 구간을 고쳤습니다.`)
        } else {
            await day.stamp({
                studentIds: [row.studentId],
                reasonId: reasonId.value,
                typeId: typeId.value,
                startSlot: startSlot.value,
                endSlot: endSlot.value,
                symptom: symptom.value.trim() || null,
            })
            say(`${row.number}번 ${row.name} — ${stampLabel.value}`)
        }
        await refresh()
    } catch (e) {
        say(String(e), 'error')
    } finally {
        busy.value = false
    }
}

async function selectStudent(row) {
    selectedId.value = row.studentId
    try {
        contacts.value = await roster.fetchContacts(row.studentId)
    } catch (e) {
        say(String(e), 'error')
    }
}

function startEditing(span) {
    editingSpan.value = span
    reasonId.value = span.reasonId
    typeId.value = span.typeId
    startSlot.value = span.startSlot
    endSlot.value = span.endSlot
    symptom.value = span.symptom ?? ''
    say('축을 고치고 학생을 다시 누르면 그 구간이 바뀝니다.', 'warn')
}

function cancelEditing() {
    editingSpan.value = null
    say('')
}

async function removeSpan(span) {
    busy.value = true
    try {
        await day.deleteSpan(span.id)
        if (editingSpan.value?.id === span.id) editingSpan.value = null
        await refresh()
        say('구간을 지웠습니다.')
    } catch (e) {
        say(String(e), 'error')
    } finally {
        busy.value = false
    }
}

async function saveReason(text) {
    if (!selectedRow.value) return
    try {
        await day.setReason(
            selectedRow.value.studentId, text,
            selectedRow.value.reason?.reasonId ?? null,
            selectedRow.value.reason?.typeId ?? null,
        )
        await refresh()
        say('사유를 저장했습니다.')
    } catch (e) {
        say(String(e), 'error')
    }
}

async function toggleCheck(item, next) {
    if (!selectedRow.value) return
    try {
        await check.setCheck(selectedRow.value.studentId, day.date, item.id, next)
        await refresh()
    } catch (e) {
        say(String(e), 'error')
    }
}

async function saveContacts(list) {
    if (!selectedRow.value) return
    try {
        await roster.saveContacts(selectedRow.value.studentId, list)
        contacts.value = await roster.fetchContacts(selectedRow.value.studentId)
        say('연락처를 저장했습니다.')
    } catch (e) {
        say(String(e), 'error')
    }
}

async function copyPrevious(row) {
    busy.value = true
    try {
        const n = await day.copyPrevious(row.studentId)
        await refresh()
        say(`${row.name} — 직전 기록 ${n}건을 넣었습니다.`)
    } catch (e) {
        say(String(e), 'error')
    } finally {
        busy.value = false
    }
}

async function updateSuggestions() {
    try {
        suggestions.value = await axis.suggestSymptoms(symptom.value.trim(), 8)
    } catch {
        suggestions.value = []
    }
}

async function move(days) {
    day.moveDate(days)
    selectedId.value = null
    editingSpan.value = null
    await refresh()
}

onMounted(async () => {
    await refresh()
    await updateSuggestions()
})
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn" text="먼저 학생 명단을 넣어주세요."/>

    <UiPage v-else :subtitle="`재학 ${day.rows.length}명 · 기록 ${day.recordedCount}명`"
            :title="day.dateLabel">
        <template #actions>
            <UiButton @click="move(-1)">◀ 어제</UiButton>
            <input :value="day.date" class="field" type="date"
                   @change="day.setDate($event.target.value); move(0)"/>
            <UiButton @click="move(1)">내일 ▶</UiButton>
        </template>

        <!-- 축 고르기 -->
        <UiCard>
            <AxisBar v-model:end-slot="endSlot" v-model:reason-id="reasonId"
                     v-model:start-slot="startSlot" v-model:type-id="typeId"
                     :reasons="axis.reasons" :types="axis.types"/>

            <div class="symptom">
                <span class="symptom__label">증상 · 사유</span>
                <input v-model="symptom" class="field symptom__input"
                       placeholder="비워 두어도 됩니다" @input="updateSuggestions"/>
                <button v-for="word in suggestions" :key="word" class="chip" type="button"
                        @click="symptom = word; updateSuggestions()">
                    {{ word }}
                </button>
            </div>

            <div class="stamp">
                <span class="stamp__hint">
                    지금 찍히는 것 —
                    <b :class="reasonId === null || typeId === null ? 'is-undecided' : ''">
                        {{ stampLabel }}
                    </b>
                </span>
                <UiButton v-if="editingSpan" variant="danger" @click="cancelEditing">
                    구간 고치기 취소
                </UiButton>
            </div>
        </UiCard>

        <div class="board">
            <!-- 학생 명단 -->
            <UiCard class="board__list">
                <p class="board__guide">
                    {{ editingSpan
                        ? '고칠 구간의 주인을 누르세요.'
                        : '학생을 누르면 위 조합이 한 건 찍힙니다. 이름을 다시 누르면 한 건 더 쌓입니다.' }}
                </p>
                <div class="students">
                    <div v-for="row in day.rows" :key="row.studentId"
                         :class="['student',
                                  row.spans.length ? 'has-record' : '',
                                  row.spans.some((s) => !s.complete) ? 'is-undecided' : '',
                                  selectedId === row.studentId ? 'is-selected' : '']">
                        <button :disabled="busy || !!problem" class="student__stamp" type="button"
                                @click="stampOn(row)">
                            <span class="student__no">{{ row.number }}</span>
                            <span class="student__name">{{ row.name }}</span>
                            <span v-if="row.spans.length" class="student__marks">
                                <span v-for="span in row.spans" :key="span.id"
                                      :class="['student__mark', span.complete ? '' : 'is-undecided']">
                                    {{ axis.describe(span.reasonId, span.typeId) }}
                                    <em>{{ span.spanText }}</em>
                                </span>
                            </span>
                        </button>
                        <button class="student__more" title="자세히" type="button"
                                @click="selectStudent(row)">⋯</button>
                    </div>
                </div>
            </UiCard>

            <StudentPanel :busy="busy" :contacts="contacts" :describe="axis.describe"
                          :items="day.items" :row="selectedRow"
                          @close="selectedId = null"
                          @copy-previous="copyPrevious"
                          @delete-span="removeSpan"
                          @edit-span="startEditing"
                          @save-contacts="saveContacts"
                          @save-reason="saveReason"
                          @toggle-check="toggleCheck"/>
        </div>

        <UiNotice :kind="messageKind" :text="message"/>
    </UiPage>
</template>

<style scoped>
.symptom {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
}

.symptom__label {
    color: var(--c-ink-3);
    min-width: 68px;
}

.symptom__input {
    width: 240px;
}

.chip {
    min-height: 36px;
    padding: 0 12px;
    border-radius: 999px;
    border: 1px solid var(--c-line);
    background: var(--c-surface);
    color: var(--c-ink-2);
    cursor: pointer;
}

.chip:hover {
    border-color: var(--c-accent);
    color: var(--c-ink);
}

.stamp {
    display: flex;
    align-items: center;
    gap: 12px;
}

.stamp__hint {
    color: var(--c-ink-3);
}

.stamp__hint b {
    color: var(--c-ink);
}

.stamp__hint b.is-undecided {
    color: var(--c-warn);
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

.board__guide {
    margin: 0;
    color: var(--c-ink-3);
}

.students {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 8px;
}

.student {
    display: flex;
    align-items: stretch;
    border: 1px solid var(--c-line);
    border-radius: 10px;
    overflow: hidden;
    background: var(--c-surface);
}

.student.has-record {
    border-color: var(--c-accent);
}

.student.is-undecided {
    border-color: var(--c-warn);
}

.student.is-selected {
    box-shadow: 0 0 0 2px var(--c-accent);
}

.student__stamp {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 10px 12px;
    border: 0;
    background: transparent;
    color: var(--c-ink);
    cursor: pointer;
    text-align: left;
}

.student__stamp:hover:not(:disabled) {
    background: var(--c-raised);
}

.student__stamp:disabled {
    cursor: default;
    opacity: 0.6;
}

.student__no {
    color: var(--c-ink-3);
}

.student__name {
    font-weight: 600;
}

.student__marks {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
}

/* 표 셀 안의 요약 배지 — 본문보다 작게 두는 예외다. */
.student__mark {
    font-size: 0.85rem;
    color: var(--c-accent);
}

.student__mark.is-undecided {
    color: var(--c-warn);
}

.student__mark em {
    font-style: normal;
    color: var(--c-ink-3);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.student__more {
    width: 34px;
    border: 0;
    border-left: 1px solid var(--c-line);
    background: transparent;
    color: var(--c-ink-3);
    cursor: pointer;
}

.student__more:hover {
    background: var(--c-raised);
    color: var(--c-ink);
}
</style>
