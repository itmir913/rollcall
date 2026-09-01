<script setup>
/**
 * 큰 토글 버튼. 체크박스는 쓰지 않는다.
 *
 * 이유는 둘이다.
 *  · 체크박스는 클릭 표적이 작아, 30행짜리 격자에서 옆 칸을 누르기 쉽다.
 *  · 켜짐/꺼짐이 색과 글자로 같이 보여야 훑어볼 때 한눈에 들어온다.
 *
 * 이 컴포넌트가 앱의 모든 불리언 입력을 담당한다. 격자의 체크 열, 설정의
 * "주말 포함" 같은 것이 전부 여기로 들어온다. 모양을 바꾸려면 이 파일만 고친다.
 */
defineProps({
    modelValue: {type: Boolean, required: true},
    /** 켜졌을 때 문구. 비우면 아이콘만 나온다. */
    onLabel: {type: String, default: '완료'},
    offLabel: {type: String, default: '미완료'},
    /** 마우스를 올렸을 때 보이는 보조 설명(마감일 등). */
    hint: {type: String, default: ''},
    disabled: {type: Boolean, default: false},
    /** full이면 칸 너비를 채운다. 격자에서 쓴다. */
    block: {type: Boolean, default: false},
})

const emit = defineEmits(['update:modelValue'])
</script>

<template>
    <button
        :aria-pressed="modelValue"
        :class="['ui-toggle', modelValue ? 'is-on' : 'is-off', block ? 'is-block' : '']"
        :disabled="disabled"
        :title="hint"
        type="button"
        @click="emit('update:modelValue', !modelValue)">
        <span aria-hidden="true" class="ui-toggle__mark">{{ modelValue ? '✓' : '' }}</span>
        <span class="ui-toggle__text">{{ modelValue ? onLabel : offLabel }}</span>
    </button>
</template>

<style scoped>
.ui-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 40px;
    padding: 0 14px;
    border-radius: 8px;
    border: 1px solid var(--c-line);
    background: var(--c-raised);
    color: var(--c-ink-3);
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease, color 120ms ease;
}

.ui-toggle.is-block {
    width: 100%;
}

.ui-toggle.is-on {
    background: color-mix(in srgb, var(--c-ok) 14%, transparent);
    border-color: var(--c-ok);
    color: var(--c-ok);
    font-weight: 600;
}

.ui-toggle:hover:not(:disabled) {
    border-color: var(--c-accent);
}

.ui-toggle:focus-visible {
    outline: 2px solid var(--c-accent);
    outline-offset: 2px;
}

.ui-toggle:disabled {
    opacity: 0.4;
    cursor: default;
}

.ui-toggle__mark {
    width: 1em;
}
</style>
