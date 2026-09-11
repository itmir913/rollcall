<script setup>
/**
 * 불리언 하나. **체크박스를 쓰지 않는다.**
 *
 * 체크박스는 클릭 표적이 작아 서른 줄짜리 목록에서 옆 칸을 누르기 쉽고, 켜짐과 꺼짐이
 * 색과 글자로 같이 보여야 훑어볼 때 한눈에 들어온다.
 */
const props = defineProps({
    modelValue: {type: Boolean, default: false},
    onLabel: {type: String, required: true},
    offLabel: {type: String, required: true},
    disabled: {type: Boolean, default: false},
})

const emit = defineEmits(['update:modelValue'])
</script>

<template>
    <button :aria-pressed="String(modelValue)" :class="['mark', modelValue ? 'is-on' : '']"
            :disabled="disabled" type="button"
            @click="emit('update:modelValue', !props.modelValue)">
        {{ modelValue ? onLabel : offLabel }}
    </button>
</template>
