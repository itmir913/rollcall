<script setup>
/**
 * 삭제 확인. **저장에는 대화상자를 쓰지 않지만 삭제에는 쓴다** —
 * 되돌릴 수 없는 것은 이것뿐이기 때문이다.
 *
 * 무엇을 지우는지 네 가지를 전부 보여준다. 날짜와 학생만 보여주면 같은 학생의
 * 두 구간 중 어느 것을 지우는지 알 수 없다.
 */
import {UiButton, UiModal} from './ui'

defineProps({
    open: {type: Boolean, default: false},
    span: {type: Object, default: null},
})

const emit = defineEmits(['confirm', 'close'])
</script>

<template>
    <UiModal :open="open" title="이 출결을 지웁니다" @close="emit('close')">
        <div v-if="span" class="modal__what">
            <span class="modal__key">날짜</span>
            <span class="modal__val num">{{ span.dateLabel }}</span>
            <span class="modal__key">학생</span>
            <span class="modal__val"><b><span class="num">{{ span.number }}</span>번 {{ span.name }}</b></span>
            <span class="modal__key">출결</span>
            <span class="modal__val">
                {{ span.reasonLabel || '미정' }} {{ span.typeLabel || '미정' }} ·
                <span class="num">{{ span.spanText }}</span>
            </span>
            <span class="modal__key">태그</span>
            <span class="modal__val">{{ span.tagName || '—' }}</span>
            <span class="modal__key">메모</span>
            <span class="modal__val">{{ span.memo || '—' }}</span>
        </div>
        <p class="modal__note">지운 기록은 되돌릴 수 없습니다.</p>

        <template #foot>
            <UiButton size="wide" @click="emit('close')">취소</UiButton>
            <UiButton fill size="wide" variant="danger" @click="emit('confirm')">지우기</UiButton>
        </template>
    </UiModal>
</template>
