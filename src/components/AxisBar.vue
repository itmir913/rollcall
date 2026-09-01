<script setup>
/**
 * 출결의 두 축을 고르는 막대. 고른 뒤 학생을 눌러 찍는다.
 *
 * 축마다 버튼을 펼친다. 드롭다운을 쓰지 않는 이유는 후보가 넷씩이고, 두 축을
 * 한눈에 비교해야 고를 수 있기 때문이다.
 *
 * **둘 다 안 고른 상태도 유효하다.** 학생이 안 왔는데 연락이 닿지 않으면 그대로
 * 찍어 두고 나중에 채운다. 그래서 각 축에 "미정" 버튼이 있고, 그게 기본값이다.
 *
 * 교시 줄은 고른 종류가 요구할 때만 나타난다(`slotPrompt`). 그 값은 종류 행이
 * 들고 있으므로 화면이 "조퇴"라는 이름을 문자열로 비교하지 않는다.
 */
import {computed, watch} from 'vue'
import SlotPicker from './SlotPicker.vue'
import {needsEnd, needsStart, slotProblem} from '../services/slots'

const props = defineProps({
    reasons: {type: Array, required: true},
    types: {type: Array, required: true},
    reasonId: {type: Number, default: null},
    typeId: {type: Number, default: null},
    startSlot: {type: String, default: null},
    endSlot: {type: String, default: null},
    /** 미정(둘 다 비움)을 허용할지. 기간 일괄 입력에서는 끈다. */
    allowUndecided: {type: Boolean, default: true},
})

const emit = defineEmits([
    'update:reasonId', 'update:typeId', 'update:startSlot', 'update:endSlot',
])

const type = computed(() => props.types.find((t) => t.id === props.typeId) ?? null)
const askStart = computed(() => needsStart(type.value?.slotPrompt))
const askEnd = computed(() => needsEnd(type.value?.slotPrompt))
const problem = computed(() =>
    slotProblem(type.value?.slotPrompt, props.startSlot, props.endSlot),
)

defineExpose({problem})

// 종류를 바꾸면 그 종류가 묻지 않는 쪽은 열린 구간으로 되돌린다.
// 안 그러면 조퇴에서 고른 5교시가 결석으로 바꾼 뒤에도 남는다.
watch(type, (next) => {
    if (!needsStart(next?.slotPrompt)) emit('update:startSlot', null)
    if (!needsEnd(next?.slotPrompt)) emit('update:endSlot', null)
})

function pickReason(id) {
    emit('update:reasonId', props.reasonId === id ? null : id)
}

function pickType(id) {
    emit('update:typeId', props.typeId === id ? null : id)
}
</script>

<template>
    <div class="axis">
        <div class="axis__row">
            <span class="axis__label">구분</span>
            <button v-for="r in reasons" :key="r.id"
                    :class="['axis__btn', reasonId === r.id ? 'is-on' : '']"
                    type="button" @click="pickReason(r.id)">
                {{ r.label }}
            </button>
            <button v-if="allowUndecided"
                    :class="['axis__btn', 'is-undecided', reasonId === null ? 'is-on' : '']"
                    title="아직 정하지 않음"
                    type="button" @click="emit('update:reasonId', null)">
                미정
            </button>
        </div>

        <div class="axis__row">
            <span class="axis__label">종류</span>
            <button v-for="t in types" :key="t.id"
                    :class="['axis__btn', typeId === t.id ? 'is-on' : '']"
                    type="button" @click="pickType(t.id)">
                {{ t.label }}
            </button>
            <button v-if="allowUndecided"
                    :class="['axis__btn', 'is-undecided', typeId === null ? 'is-on' : '']"
                    title="아직 정하지 않음"
                    type="button" @click="emit('update:typeId', null)">
                미정
            </button>
        </div>

        <SlotPicker v-if="askStart" :model-value="startSlot" label="시작 교시"
                    @update:model-value="emit('update:startSlot', $event)"/>
        <SlotPicker v-if="askEnd" :model-value="endSlot" label="끝 교시"
                    @update:model-value="emit('update:endSlot', $event)"/>

        <p v-if="problem" class="axis__problem">{{ problem }}</p>
    </div>
</template>

<style scoped>
.axis {
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.axis__row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
}

.axis__label {
    color: var(--c-ink-3);
    min-width: 68px;
}

.axis__btn {
    min-width: 64px;
    min-height: 40px;
    padding: 0 14px;
    border-radius: 8px;
    border: 1px solid var(--c-line);
    background: var(--c-raised);
    color: var(--c-ink-2);
    cursor: pointer;
}

.axis__btn:hover {
    border-color: var(--c-accent);
    color: var(--c-ink);
}

.axis__btn.is-on {
    background: var(--c-accent);
    border-color: var(--c-accent);
    color: #fff;
    font-weight: 600;
}

.axis__btn.is-undecided {
    border-style: dashed;
}

.axis__btn.is-undecided.is-on {
    background: transparent;
    border-color: var(--c-warn);
    color: var(--c-warn);
}

.axis__problem {
    margin: 0;
    color: var(--c-warn);
}
</style>
