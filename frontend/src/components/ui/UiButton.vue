<script setup>
/**
 * 버튼 하나. 색과 모서리는 style.css의 `.btn--*`가 정하고, 여기서는 조합만 한다.
 *
 * `download`와 `upload`는 프로젝트 전체에서 같은 모양이어야 한다 —
 * 파일이 컴퓨터에 떨어지거나 들어오는 동작이라 다른 버튼과 섞이면 안 된다.
 * 그래서 화살표를 이 컴포넌트가 직접 들고 있다.
 */
import {computed} from 'vue'

const props = defineProps({
    variant: {type: String, default: 'default'}, // default | primary | ghost | danger | download | upload
    size: {type: String, default: 'default'}, // default | tight | wide
    fill: {type: Boolean, default: false}, // danger를 채운다(지우기 확인)
    icon: {type: Boolean, default: false}, // 아이콘만 있는 버튼
    disabled: {type: Boolean, default: false},
    type: {type: String, default: 'button'},
})

const classes = computed(() => [
    'btn',
    props.variant !== 'default' ? `btn--${props.variant}` : '',
    props.size !== 'default' ? `btn--${props.size}` : '',
    props.fill ? 'btn--fill' : '',
    props.icon ? 'btn--icon' : '',
])
</script>

<template>
    <button :class="classes" :disabled="disabled" :type="type">
        <svg v-if="variant === 'download'" aria-hidden="true" class="icon" fill="none"
             stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"
             stroke-width="1.9" viewBox="0 0 24 24">
            <path d="M12 4v11"/>
            <path d="M7 11l5 5 5-5"/>
            <path d="M5 20h14"/>
        </svg>
        <svg v-else-if="variant === 'upload'" aria-hidden="true" class="icon" fill="none"
             stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"
             stroke-width="1.9" viewBox="0 0 24 24">
            <path d="M12 20V9"/>
            <path d="M7 13l5-5 5 5"/>
            <path d="M5 4h14"/>
        </svg>
        <slot/>
    </button>
</template>
