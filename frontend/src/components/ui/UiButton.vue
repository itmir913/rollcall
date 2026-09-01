<script setup>
/**
 * 버튼 하나. 앱의 모든 버튼이 이걸 쓴다.
 *
 * 색과 여백을 컴포넌트마다 적으면 나중에 디자인을 바꿀 때 전 파일을 뒤져야 한다.
 * 시각 디자인은 아직 정하지 않았으므로, 바꿀 자리를 한 곳으로 모아 두는 것이 목적이다.
 */
defineProps({
    variant: {type: String, default: 'default'}, // default | primary | ghost | danger
    disabled: {type: Boolean, default: false},
    type: {type: String, default: 'button'},
})
</script>

<template>
    <button :class="['ui-btn', `is-${variant}`]" :disabled="disabled" :type="type">
        <slot/>
    </button>
</template>

<style scoped>
.ui-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 40px;
    padding: 0 16px;
    border-radius: 8px;
    border: 1px solid var(--c-line);
    background: var(--c-raised);
    color: var(--c-ink-2);
    cursor: pointer;
    transition: border-color 120ms ease, color 120ms ease, background 120ms ease;
}

.ui-btn:hover:not(:disabled) {
    border-color: var(--c-accent);
    color: var(--c-ink);
}

.ui-btn:focus-visible {
    outline: 2px solid var(--c-accent);
    outline-offset: 2px;
}

.ui-btn:disabled {
    opacity: 0.45;
    cursor: default;
}

.ui-btn.is-primary {
    background: var(--c-accent);
    border-color: var(--c-accent);
    color: #fff;
}

.ui-btn.is-primary:hover:not(:disabled) {
    color: #fff;
    filter: brightness(1.08);
}

.ui-btn.is-ghost {
    background: transparent;
    border-color: transparent;
    color: var(--c-ink-3);
}

.ui-btn.is-danger {
    border-color: var(--c-danger);
    color: var(--c-danger);
    background: transparent;
}
</style>
