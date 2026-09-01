<script setup>
/**
 * 기간 일괄 입력 — 체험학습 등.
 *
 * 주말은 요일 계산으로 뺀다(캘린더 테이블을 두지 않는다). 재량휴업일·공휴일은
 * 프로그램이 알 수 없으므로 미리보기에서 교사가 지운다.
 */
import {computed, onMounted, ref} from 'vue'
import {useAppStore} from '../stores/app'
import {useCodeStore} from '../stores/code'
import {useDayStore} from '../stores/day'
import {useRosterStore} from '../stores/roster'
import {needsEnd, needsStart} from '../services/slots'
import CodePicker from '../components/CodePicker.vue'
import SlotPicker from '../components/SlotPicker.vue'
import {UiButton, UiCard, UiNotice, UiPage, UiToggle} from '../components/ui'

const app = useAppStore()
const day = useDayStore()
const codeStore = useCodeStore()
const roster = useRosterStore()

const selected = ref([])
const from = ref(new Date().toISOString().slice(0, 10))
const to = ref(new Date().toISOString().slice(0, 10))
const codeId = ref(null)
const startSlot = ref(null)
const endSlot = ref(null)
const symptom = ref('')
const days = ref([])
const skipped = ref(new Set())
const message = ref('')
const error = ref('')

const code = computed(() => codeStore.codes.find((c) => c.id === codeId.value) ?? null)
const askStart = computed(() => needsStart(code.value?.slotPrompt))
const askEnd = computed(() => needsEnd(code.value?.slotPrompt))
const chosenDates = computed(() =>
    days.value.filter((d) => !skipped.value.has(d.date)).map((d) => d.date),
)

function isPicked(studentId) {
    return selected.value.includes(studentId)
}

function togglePick(studentId, next) {
    selected.value = next
        ? [...selected.value, studentId]
        : selected.value.filter((id) => id !== studentId)
}

async function load() {
    if (!app.ready) return
    await Promise.all([
        roster.fetchStudents(app.yearId, app.grade, app.classNo),
        codeStore.fetchCodes(),
    ])
    codeId.value = codeStore.codes.find((c) => c.label === '출석인정결석')?.id
        ?? codeStore.codes[0]?.id
        ?? null
}

async function makePreview() {
    error.value = ''
    message.value = ''
    try {
        days.value = await day.bulkPreview(selected.value, from.value, to.value)
        skipped.value = new Set()
    } catch (e) {
        error.value = String(e)
    }
}

function toggleDay(date) {
    const next = new Set(skipped.value)
    next.has(date) ? next.delete(date) : next.add(date)
    skipped.value = next
}

async function apply() {
    error.value = ''
    try {
        const result = await day.bulkApply(
            selected.value,
            chosenDates.value,
            codeId.value,
            askStart.value ? startSlot.value : null,
            askEnd.value ? endSlot.value : null,
            symptom.value.trim() || null,
        )
        message.value = `${selected.value.length}명 × ${result.days}일을 저장했습니다. `
            + '서류 마감일은 마지막 날 기준으로 전 건에 같게 들어갑니다.'
        days.value = []
    } catch (e) {
        error.value = String(e)
    }
}

onMounted(load)
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn" text="먼저 설정에서 학년도와 학급을 정해주세요."/>

    <UiPage v-else subtitle="체험학습처럼 여러 날에 걸친 기록을 한 번에 넣습니다."
            title="기간 일괄 입력">
        <UiCard :title="`학생 ${selected.length}명 선택`">
            <div class="bulk__picks">
                <UiToggle v-for="s in roster.students" :key="s.id"
                          :model-value="isPicked(s.id)"
                          :off-label="`${s.number} ${s.name}`"
                          :on-label="`${s.number} ${s.name}`"
                          @update:model-value="togglePick(s.id, $event)"/>
            </div>
        </UiCard>

        <UiCard title="기간과 사유">
            <div class="bulk__row">
                <input v-model="from" class="field" type="date"/>
                <span>~</span>
                <input v-model="to" class="field" type="date"/>
                <input v-model="symptom" class="field bulk__reason"
                       placeholder="사유 (예: 교외체험학습)"/>
            </div>

            <CodePicker v-model="codeId" :codes="codeStore.codes"/>
            <SlotPicker v-if="askStart" v-model="startSlot" label="시작 교시"/>
            <SlotPicker v-if="askEnd" v-model="endSlot" label="끝 교시"/>

            <UiButton :disabled="!selected.length" class="bulk__save" variant="primary"
                      @click="makePreview">
                미리보기
            </UiButton>
        </UiCard>

        <UiCard v-if="days.length" :title="`미리보기 — ${chosenDates.length}일`"
                description="주말은 이미 빠져 있습니다. 재량휴업일이나 공휴일은 눌러서 빼주세요.">
            <div class="bulk__days">
                <UiToggle v-for="d in days" :key="d.date"
                          :hint="d.hasExisting ? '이미 그날 기록이 있습니다.' : ''"
                          :model-value="!skipped.has(d.date)"
                          :off-label="`${d.label} 제외`"
                          :on-label="d.hasExisting ? `${d.label} · 기록 있음` : d.label"
                          @update:model-value="toggleDay(d.date)"/>
            </div>
            <UiButton :disabled="!chosenDates.length" class="bulk__save" variant="primary"
                      @click="apply">
                저장
            </UiButton>
        </UiCard>

        <UiNotice :text="message" kind="ok"/>
        <UiNotice :text="error" kind="error"/>
    </UiPage>
</template>

<style scoped>
.bulk__picks,
.bulk__days {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
}

.bulk__row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
}

.bulk__reason {
    flex: 1;
    min-width: 220px;
}

.bulk__save {
    align-self: flex-start;
}
</style>
