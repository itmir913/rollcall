<script setup>
/** 한 줄짜리 알림. 성공·경고·오류가 같은 자리에 같은 모양으로 뜬다. */
defineProps({
    kind: {type: String, default: 'info'}, // info | ok | warn | error
    text: {type: String, default: ''},
})
</script>

<template>
    <p v-if="text" :class="['ui-notice', `is-${kind}`]" role="status">{{ text }}</p>
</template>

<style scoped>
.ui-notice {
    margin: 0;
    padding: 10px 14px;
    border-radius: 8px;
    border: 1px solid transparent;
}

.is-info {
    color: var(--c-ink-3);
}

.is-ok {
    color: var(--c-ok);
    border-color: color-mix(in srgb, var(--c-ok) 35%, transparent);
    background: color-mix(in srgb, var(--c-ok) 10%, transparent);
}

.is-warn {
    color: var(--c-warn);
    border-color: color-mix(in srgb, var(--c-warn) 35%, transparent);
    background: color-mix(in srgb, var(--c-warn) 10%, transparent);
}

.is-error {
    color: var(--c-danger-ink);
    background: var(--c-danger-bg);
    border-color: color-mix(in srgb, var(--c-danger) 35%, transparent);
}
</style>
