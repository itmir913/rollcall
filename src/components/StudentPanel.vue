<script setup>
/**
 * 학생 상세. 명단 **오른편에 붙어서** 뜬다.
 *
 * 화면 가운데를 가리는 모달을 쓰지 않는다. 출결 입력은 학생을 옮겨 다니며 하는
 * 일이라, 패널을 열 때마다 명단이 가려지면 다음 학생을 찾을 수 없다. 여기서는
 * 명단이 계속 보이고 패널만 바뀐다.
 */
import {computed, ref, watch} from 'vue'
import {formatSpan} from '../services/slots'
import {UiButton, UiNotice, UiToggle} from './ui'

const props = defineProps({
    row: {type: Object, default: null},
    /** 체크 항목 정의 */
    items: {type: Array, default: () => []},
    /** 두 축을 이름으로 바꿔 주는 함수. axis 스토어가 넘겨준다. */
    describe: {type: Function, required: true},
    contacts: {type: Array, default: () => []},
    busy: {type: Boolean, default: false},
})

const emit = defineEmits([
    'close', 'edit-span', 'delete-span', 'save-reason', 'toggle-check',
    'save-contacts', 'copy-previous',
])

const reasonText = ref('')
const draftContacts = ref([])
const contactsDirty = ref(false)

watch(
    () => props.row,
    (row) => {
        reasonText.value = row?.reason?.reason ?? ''
    },
    {immediate: true},
)

watch(
    () => props.contacts,
    (list) => {
        draftContacts.value = list.map((c) => ({...c}))
        contactsDirty.value = false
    },
    {immediate: true, deep: true},
)

const incomplete = computed(() => props.row?.spans.some((s) => !s.complete) ?? false)

function checkState(itemId) {
    return props.row?.checks.find((c) => c.itemId === itemId) ?? null
}

function addContact() {
    draftContacts.value.push({id: 0, label: '', value: '', note: null, sortOrder: 0})
    contactsDirty.value = true
}

function removeContact(index) {
    draftContacts.value.splice(index, 1)
    contactsDirty.value = true
}
</script>

<template>
    <aside v-if="row" class="panel">
        <header class="panel__head">
            <div>
                <b class="panel__name">{{ row.number }}번 {{ row.name }}</b>
                <p v-if="incomplete" class="panel__flag">구분 · 종류가 아직 비어 있습니다</p>
            </div>
            <UiButton variant="ghost" @click="emit('close')">닫기 ✕</UiButton>
        </header>

        <!-- 그날의 구간 -->
        <section class="panel__section">
            <h3 class="panel__title">오늘의 출결</h3>
            <p v-if="!row.spans.length" class="panel__muted">
                기록 없음 — 출석입니다.
            </p>
            <div v-for="span in row.spans" :key="span.id" class="span">
                <div class="span__body">
                    <span class="span__range">{{ formatSpan(span.startSlot, span.endSlot) }}</span>
                    <span :class="['span__code', span.complete ? '' : 'is-undecided']">
                        {{ describe(span.reasonId, span.typeId) }}
                    </span>
                    <span v-if="span.symptom" class="panel__muted">{{ span.symptom }}</span>
                </div>
                <div class="span__actions">
                    <UiButton variant="ghost" @click="emit('edit-span', span)">고치기</UiButton>
                    <UiButton variant="ghost" @click="emit('delete-span', span)">✕</UiButton>
                </div>
            </div>
            <UiButton v-if="!row.spans.length" :disabled="busy"
                      @click="emit('copy-previous', row)">
                직전 기록 그대로 넣기
            </UiButton>
        </section>

        <!-- 나이스 사유 -->
        <section v-if="row.reason || row.spans.length" class="panel__section">
            <h3 class="panel__title">나이스 사유</h3>
            <textarea v-model="reasonText" class="field panel__reason"
                      placeholder="두 축이 정해지면 초안이 만들어집니다."></textarea>
            <UiButton :disabled="!reasonText.trim()"
                      @click="emit('save-reason', reasonText.trim())">
                사유 저장
            </UiButton>
        </section>

        <!-- 체크 -->
        <section v-if="row.checks.length" class="panel__section">
            <h3 class="panel__title">서류 · 입력</h3>
            <div v-for="item in items" :key="item.id" class="check">
                <span>{{ item.name }}</span>
                <UiToggle v-if="checkState(item.id)"
                          :hint="checkState(item.id).dueDate
                              ? `마감 ${checkState(item.id).dueDate}` : '마감 없음'"
                          :model-value="checkState(item.id).done"
                          off-label="미완료" on-label="완료"
                          @update:model-value="emit('toggle-check', item, $event)"/>
            </div>
        </section>

        <!-- 연락처 -->
        <section class="panel__section">
            <h3 class="panel__title">연락처</h3>
            <div v-for="(c, i) in draftContacts" :key="i" class="contact">
                <input v-model="c.label" class="field contact__label" placeholder="관계"
                       @input="contactsDirty = true"/>
                <input v-model="c.value" class="field contact__value" placeholder="전화번호"
                       @input="contactsDirty = true"/>
                <UiButton variant="ghost" @click="removeContact(i)">✕</UiButton>
            </div>
            <div class="panel__row">
                <UiButton @click="addContact">+ 연락처 추가</UiButton>
                <UiButton v-if="contactsDirty" variant="primary"
                          @click="emit('save-contacts', draftContacts)">
                    저장
                </UiButton>
            </div>
            <UiNotice v-if="!draftContacts.length" kind="info"
                      text="학생 본인·어머니·아버지 등 필요한 만큼 넣을 수 있습니다."/>
        </section>
    </aside>
</template>

<style scoped>
.panel {
    width: 340px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 16px;
    background: var(--c-surface);
    border: 1px solid var(--c-line);
    border-radius: 12px;
    align-self: flex-start;
    position: sticky;
    top: 16px;
    max-height: calc(100vh - 120px);
    overflow-y: auto;
}

.panel__head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
}

.panel__name {
    font-size: 1.1rem;
}

.panel__flag {
    margin: 4px 0 0;
    color: var(--c-warn);
}

.panel__section {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.panel__title {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--c-ink-3);
}

.panel__muted {
    margin: 0;
    color: var(--c-ink-3);
}

.panel__reason {
    min-height: 72px;
    resize: vertical;
    width: 100%;
}

.panel__row {
    display: flex;
    gap: 8px;
}

.span {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--c-raised);
}

.span__body {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.span__range {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: var(--c-ink-3);
}

.span__code {
    font-weight: 600;
}

.span__code.is-undecided {
    color: var(--c-warn);
}

.span__actions {
    display: flex;
    gap: 2px;
}

.check {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
}

.contact {
    display: flex;
    gap: 6px;
}

.contact__label {
    width: 84px;
}

.contact__value {
    flex: 1;
    min-width: 0;
}
</style>
