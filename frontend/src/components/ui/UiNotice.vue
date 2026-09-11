<script setup>
/**
 * 한 줄 알림.
 *
 * 빈 문자열이면 아무것도 그리지 않는다 — 빈 상자가 자리를 차지하면 그만큼 화면이
 * 흔들린다. 실패를 조용히 삼키지 않기 위한 자리이지, 상시 표시용이 아니다.
 */
defineProps({
    kind: {type: String, default: 'error'}, // error | warn | ok
    text: {type: String, default: ''},
})
</script>

<template>
    <p v-if="text" :class="['notice', `notice--${kind}`]" role="status">{{ text }}</p>
</template>
