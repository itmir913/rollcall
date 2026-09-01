<script setup>
/**
 * HOME — 일일 입력 격자.
 *
 * 기준선: **학생 1명 = 키보드만으로 5초 이내.** 그래서
 *  · 빠른 입력 한 줄로 끝난다 — "2 q 몸살" (번호·단축키·증상)
 *  · Enter로 저장하고 입력창은 비워져 다음 학생을 기다린다
 *  · 확인 대화상자·저장 버튼·드롭다운이 없다
 *  · 전체 행을 항상 띄운다. 빈 행은 출석이고 저장되지 않는다
 */
import {computed, nextTick, onMounted, ref, watch} from 'vue'
import {useAppStore} from '../stores/app'
import {useCodeStore} from '../stores/code'
import {useDayStore} from '../stores/day'
import {useCheckStore} from '../stores/check'
import {formatSpan, parseCommand} from '../services/commandParser'
import {UiButton, UiCard, UiNotice, UiPage, UiTable, UiToggle} from '../components/ui'

const app = useAppStore()
const day = useDayStore()
const codeStore = useCodeStore()
const check = useCheckStore()

const line = ref('')
const message = ref('')
const messageKind = ref('ok')
const commandInput = ref(null)
const suggestions = ref([])

const parsed = computed(() => parseCommand(line.value, codeStore.codes))

const columns = computed(() => [
    {key: 'number', label: '번호', width: '64px'},
    {key: 'name', label: '성명', width: '110px'},
    {key: 'span', label: '구간', width: '96px'},
    {key: 'code', label: '코드', width: '150px'},
    {key: 'reason', label: '사유'},
    ...day.items.map((item) => ({key: `item-${item.id}`, label: item.name, width: '128px'})),
    {key: 'actions', label: '', width: '132px', align: 'right'},
])

const preview = computed(() => {
    const p = parsed.value
    if (p.error) return {kind: 'error', text: p.error}
    if (!p.ok) {
        const row = p.number ? day.rowByNumber(p.number) : null
        if (p.needs === 'number') return {kind: 'info', text: '번호 → 단축키 → (교시) → 증상'}
        if (p.needs === 'code') {
            return row
                ? {kind: 'info', text: `${row.name} — 단축키를 누르세요`}
                : {kind: 'error', text: `${p.number}번 학생이 없습니다.`}
        }
        if (p.needs === 'slot') return {kind: 'info', text: `${p.code.label} — 교시를 입력하세요`}
        return {kind: 'info', text: ''}
    }
    const row = day.rowByNumber(p.number)
    if (!row) return {kind: 'error', text: `${p.number}번 학생이 없습니다.`}
    return {
        kind: 'ok',
        text: `${row.name} · ${p.code.label} · ${formatSpan(p.startSlot, p.endSlot)}`
            + (p.symptom ? ` · ${p.symptom}` : ''),
    }
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

async function submit() {
    const p = parsed.value
    if (!p.ok) {
        if (p.error) say(p.error, 'error')
        return
    }
    const row = day.rowByNumber(p.number)
    if (!row) return say(`${p.number}번 학생이 없습니다.`, 'error')

    try {
        await day.addSpan({
            studentId: row.studentId,
            codeId: p.code.id,
            startSlot: p.startSlot,
            endSlot: p.endSlot,
            symptom: p.symptom,
        })
        line.value = ''
        await refresh()
        say(`${row.number}번 ${row.name} — ${p.code.label} 저장`)
    } catch (e) {
        say(String(e), 'error')
    }
}

async function removeSpan(spanId) {
    try {
        await day.deleteSpan(spanId)
        await refresh()
        say('구간을 지웠습니다.')
    } catch (e) {
        say(String(e), 'error')
    } finally {
        focusCommand()
    }
}

async function repeatYesterday(row) {
    try {
        const n = await day.copyPrevious(row.studentId)
        await refresh()
        say(`${row.name} — 직전 기록 ${n}건을 그대로 넣었습니다.`)
    } catch (e) {
        say(String(e), 'error')
    } finally {
        focusCommand()
    }
}

async function toggleCheck(row, item, next) {
    try {
        await check.setCheck(row.studentId, day.date, item.id, next)
        await refresh()
    } catch (e) {
        say(String(e), 'error')
    } finally {
        focusCommand()
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

function focusCommand() {
    nextTick(() => commandInput.value?.focus())
}

async function move(days) {
    day.moveDate(days)
    await refresh()
    focusCommand()
}

async function updateSuggestions() {
    const typed = parsed.value.ok ? parsed.value.symptom : null
    suggestions.value = typed ? await codeStore.suggestSymptoms(typed, 6) : []
}

watch(line, updateSuggestions)

onMounted(async () => {
    await refresh()
    focusCommand()
})
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn"
              text="먼저 설정에서 학년도와 학급을 정해주세요."/>

    <UiPage v-else :subtitle="`기록 ${day.recordedCount}명 / 재학 ${day.rows.length}명`"
            :title="day.dateLabel">
        <template #actions>
            <UiButton @click="move(-1)">◀ 어제</UiButton>
            <input :value="day.date" class="field" type="date"
                   @change="day.setDate($event.target.value); refresh()"/>
            <UiButton @click="move(1)">내일 ▶</UiButton>
        </template>

        <!-- 빠른 입력 -->
        <UiCard>
            <div class="quick">
                <label class="quick__label" for="cmd">빠른 입력</label>
                <input id="cmd" ref="commandInput" v-model="line" autocomplete="off"
                       class="field quick__input"
                       placeholder="예)  2 q 몸살      ·      5 w 6 복통"
                       @keydown.enter.prevent="submit"/>
                <UiButton :disabled="!parsed.ok" variant="primary" @click="submit">저장</UiButton>
            </div>

            <UiNotice :kind="preview.kind" :text="preview.text"/>

            <p v-if="suggestions.length" class="quick__hint">
                자주 쓴 증상: {{ suggestions.join(' · ') }}
            </p>

            <div class="quick__keys">
                <span v-for="c in codeStore.codes.filter((c) => c.shortcut)" :key="c.id"
                      class="quick__key">
                    <b>{{ c.shortcut }}</b> {{ c.label }}
                </span>
            </div>
        </UiCard>

        <!-- 격자 -->
        <UiCard>
            <UiTable :columns="columns" :rows="day.rows"
                     empty-text="이 날짜에 재학 중인 학생이 없습니다." row-key="studentId">
                <template #row="{row}">
                    <td>{{ row.number }}</td>
                    <td>{{ row.name }}</td>
                    <td>
                        <div v-for="span in row.spans" :key="span.id" class="mono">
                            {{ span.spanText }}
                        </div>
                    </td>
                    <td>
                        <div v-for="span in row.spans" :key="span.id" class="code-cell">
                            <span>{{ span.codeLabel }}</span>
                            <UiButton title="이 구간 지우기" variant="ghost"
                                      @click="removeSpan(span.id)">✕</UiButton>
                        </div>
                    </td>
                    <td>
                        <input v-if="row.reason" :value="row.reason.reason" class="field w-full"
                               @blur="saveReason(row, $event)"
                               @keydown.enter="$event.target.blur()"/>
                    </td>
                    <td v-for="item in day.items" :key="item.id">
                        <UiToggle v-if="checkState(row, item.id)"
                                  :hint="dueHint(checkState(row, item.id))"
                                  :model-value="checkState(row, item.id).done"
                                  block off-label="미완료" on-label="완료"
                                  @update:model-value="toggleCheck(row, item, $event)"/>
                    </td>
                    <td class="text-right">
                        <UiButton v-if="!row.spans.length" @click="repeatYesterday(row)">
                            어제 것 그대로
                        </UiButton>
                    </td>
                </template>
            </UiTable>
        </UiCard>

        <div class="summary">
            <span v-for="s in check.summary" :key="s.itemId" class="summary__item">
                {{ s.itemName }} 미완료 <b>{{ s.count }}</b>건
            </span>
            <RouterLink class="summary__link" to="/pending">미제출자 목록 →</RouterLink>
        </div>

        <UiNotice :kind="messageKind" :text="message"/>
    </UiPage>
</template>

<style scoped>
.quick {
    display: flex;
    align-items: center;
    gap: 12px;
}

.quick__label {
    color: var(--c-ink-3);
    flex-shrink: 0;
}

.quick__input {
    flex: 1;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    min-height: 40px;
}

.quick__hint {
    margin: 0;
    color: var(--c-ink-3);
}

.quick__keys {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
}

.quick__key {
    padding: 4px 10px;
    border-radius: 6px;
    background: var(--c-raised);
    color: var(--c-ink-2);
}

.mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.code-cell {
    display: flex;
    align-items: center;
    gap: 4px;
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

.w-full {
    width: 100%;
}

.text-right {
    text-align: right;
}
</style>
