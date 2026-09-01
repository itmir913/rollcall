<script setup>
/**
 * 구간 하나를 넣거나 고치는 편집기. 격자의 행 아래에 펼쳐진다.
 *
 * 클릭으로 끝나도록 만든다 — 코드도 교시도 버튼이고, 타이핑이 필요한 곳은
 * 증상 한 칸뿐이다. 그마저도 과거에 쓴 단어가 후보 버튼으로 뜬다.
 *
 * 문구 미리보기는 Rust에게 물어본다(`render_phrase`). 프론트엔드가 같은 규칙을
 * 다시 구현하면 미리보기와 실제 저장값이 갈라진다.
 */
import {computed, onMounted, ref, watch} from 'vue'
import CodePicker from './CodePicker.vue'
import SlotPicker from './SlotPicker.vue'
import {UiButton, UiNotice} from './ui'
import {formatSpan, isReversed, needsEnd, needsStart} from '../services/slots'

const props = defineProps({
    codes: {type: Array, required: true},
    studentName: {type: String, default: ''},
    /** 고칠 구간. 없으면 새로 넣는 것이다. */
    span: {type: Object, default: null},
    /** 증상 후보를 가져오는 함수. 스토어가 넘겨준다. */
    suggest: {type: Function, required: true},
    /** 문구 미리보기를 만드는 함수. 스토어가 넘겨준다. */
    renderPhrase: {type: Function, required: true},
})

const emit = defineEmits(['save', 'cancel', 'remove'])

const codeId = ref(props.span?.codeId ?? null)
const startSlot = ref(props.span?.startSlot ?? null)
const endSlot = ref(props.span?.endSlot ?? null)
const symptom = ref(props.span?.symptom ?? '')
const phrase = ref('')
const suggestions = ref([])

const code = computed(() => props.codes.find((c) => c.id === codeId.value) ?? null)
const askStart = computed(() => needsStart(code.value?.slotPrompt))
const askEnd = computed(() => needsEnd(code.value?.slotPrompt))

const problem = computed(() => {
    if (!code.value) return '출결 구분을 골라주세요.'
    if (askStart.value && !startSlot.value) return '시작 교시를 골라주세요.'
    if (askEnd.value && !endSlot.value) return '끝 교시를 골라주세요.'
    if (isReversed(startSlot.value, endSlot.value)) return '시작 교시가 끝 교시보다 뒤입니다.'
    return ''
})

const spanText = computed(() => formatSpan(startSlot.value, endSlot.value))

/** 코드를 바꾸면 그 유형이 묻지 않는 쪽은 열린 구간으로 되돌린다. */
watch(code, (next, prev) => {
    if (!next || next === prev) return
    if (!needsStart(next.slotPrompt)) startSlot.value = next.defaultStart ?? null
    if (!needsEnd(next.slotPrompt)) endSlot.value = next.defaultEnd ?? null
})

async function refreshPhrase() {
    if (!code.value) return (phrase.value = '')
    try {
        phrase.value = await props.renderPhrase(
            code.value.id, symptom.value.trim() || null, startSlot.value, endSlot.value,
        )
    } catch {
        phrase.value = ''
    }
}

async function refreshSuggestions() {
    try {
        suggestions.value = await props.suggest(symptom.value.trim(), 8)
    } catch {
        suggestions.value = []
    }
}

watch([codeId, startSlot, endSlot, symptom], refreshPhrase)
watch(symptom, refreshSuggestions)

function pickSuggestion(word) {
    symptom.value = word
}

function save() {
    if (problem.value) return
    emit('save', {
        codeId: codeId.value,
        startSlot: startSlot.value,
        endSlot: endSlot.value,
        symptom: symptom.value.trim() || null,
    })
}

onMounted(() => {
    refreshPhrase()
    refreshSuggestions()
})
</script>

<template>
    <div class="editor">
        <div class="editor__head">
            <b>{{ studentName }}</b>
            <span class="editor__span">{{ spanText }}</span>
            <span v-if="code" class="editor__code">{{ code.label }}</span>
        </div>

        <CodePicker v-model="codeId" :codes="codes"/>

        <SlotPicker v-if="askStart" v-model="startSlot" label="시작 교시"/>
        <SlotPicker v-if="askEnd" v-model="endSlot" label="끝 교시"/>

        <div class="editor__symptom">
            <span class="editor__label">증상 · 사유</span>
            <input v-model="symptom" class="field editor__input"
                   placeholder="예) 몸살, 교외체험학습"/>
        </div>

        <div v-if="suggestions.length" class="editor__chips">
            <span class="editor__label">자주 쓴 것</span>
            <button v-for="word in suggestions" :key="word" class="editor__chip" type="button"
                    @click="pickSuggestion(word)">
                {{ word }}
            </button>
        </div>

        <div v-if="phrase" class="editor__preview">
            나이스 사유 초안 — <b>{{ phrase }}</b>
        </div>

        <UiNotice :text="problem" kind="warn"/>

        <div class="editor__actions">
            <UiButton :disabled="!!problem" variant="primary" @click="save">
                {{ span ? '수정' : '저장' }}
            </UiButton>
            <UiButton @click="emit('cancel')">취소</UiButton>
            <UiButton v-if="span" class="editor__remove" variant="danger"
                      @click="emit('remove')">
                이 구간 지우기
            </UiButton>
        </div>
    </div>
</template>

<style scoped>
.editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    border-radius: 10px;
    background: var(--c-raised);
    border: 1px solid var(--c-line);
}

.editor__head {
    display: flex;
    align-items: center;
    gap: 10px;
}

.editor__span {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: var(--c-ink-3);
}

.editor__code {
    color: var(--c-accent);
    font-weight: 600;
}

.editor__label {
    color: var(--c-ink-3);
    min-width: 68px;
}

.editor__symptom,
.editor__chips {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
}

.editor__input {
    flex: 1;
    min-width: 240px;
    min-height: 40px;
}

.editor__chip {
    min-height: 36px;
    padding: 0 12px;
    border-radius: 999px;
    border: 1px solid var(--c-line);
    background: var(--c-surface);
    color: var(--c-ink-2);
    cursor: pointer;
}

.editor__chip:hover {
    border-color: var(--c-accent);
    color: var(--c-ink);
}

.editor__preview {
    color: var(--c-ink-2);
}

.editor__actions {
    display: flex;
    gap: 8px;
}

.editor__remove {
    margin-left: auto;
}
</style>
