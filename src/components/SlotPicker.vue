<script setup>
/**
 * 교시 고르기. 조회 · 1~9 · 종례를 전부 버튼으로 펼친다.
 *
 * 드롭다운을 쓰지 않는 이유는 후보가 11개로 고정이고 늘어날 일이 없기 때문이다.
 * 펼쳐 두면 한 번 클릭으로 끝나고, 지금 무엇이 골라져 있는지 눈으로 보인다.
 */
import {SLOTS, slotLabel} from '../services/slots'

defineProps({
    modelValue: {type: String, default: null},
    label: {type: String, default: ''},
    /** 비우기(=열린 구간)를 허용할지. 조퇴의 시작 교시처럼 필수인 자리에서는 끈다. */
    clearable: {type: Boolean, default: false},
})

const emit = defineEmits(['update:modelValue'])

const slots = SLOTS
</script>

<template>
    <div class="slot-picker">
        <span v-if="label" class="slot-picker__label">{{ label }}</span>
        <div class="slot-picker__row">
            <button v-for="slot in slots" :key="slot"
                    :class="['slot-picker__btn', modelValue === slot ? 'is-on' : '']"
                    type="button"
                    @click="emit('update:modelValue', slot)">
                {{ slot }}
            </button>
            <button v-if="clearable"
                    :class="['slot-picker__btn', 'is-clear', modelValue === null ? 'is-on' : '']"
                    title="열린 구간 (처음부터 / 끝까지)"
                    type="button"
                    @click="emit('update:modelValue', null)">
                *
            </button>
        </div>
        <span class="slot-picker__value">{{ slotLabel(modelValue) }}</span>
    </div>
</template>

<style scoped>
.slot-picker {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
}

.slot-picker__label {
    color: var(--c-ink-3);
    min-width: 68px;
}

.slot-picker__row {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
}

.slot-picker__btn {
    min-width: 44px;
    min-height: 40px;
    padding: 0 10px;
    border-radius: 8px;
    border: 1px solid var(--c-line);
    background: var(--c-raised);
    color: var(--c-ink-2);
    cursor: pointer;
}

.slot-picker__btn:hover {
    border-color: var(--c-accent);
    color: var(--c-ink);
}

.slot-picker__btn.is-on {
    background: var(--c-accent);
    border-color: var(--c-accent);
    color: #fff;
    font-weight: 600;
}

.slot-picker__btn.is-clear {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.slot-picker__value {
    color: var(--c-ink-3);
    min-width: 56px;
}
</style>
