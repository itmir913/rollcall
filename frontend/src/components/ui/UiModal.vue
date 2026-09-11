<script setup>
/**
 * 대화상자.
 *
 * **저장에는 쓰지 않는다.** 편집기의 [저장]이 곧 저장이고, 학생 번호를 누르면 그대로
 * 입력된다. 되돌리기 어려운 것은 삭제뿐이라 거기에만 한 번 묻는다.
 *
 * `dismissible`이 false면 ESC와 바깥 클릭으로 닫히지 않는다 — 집중 등재처럼
 * 저장인지 취소인지 반드시 골라야 하는 경우다.
 */
import {nextTick, onBeforeUnmount, onMounted, ref, watch} from 'vue'

const props = defineProps({
    open: {type: Boolean, default: false},
    title: {type: String, default: ''},
    subtitle: {type: String, default: ''},
    size: {type: String, default: 'default'}, // default | wide | focus
    dismissible: {type: Boolean, default: true},
})

const emit = defineEmits(['close'])
const box = ref(null)

function onBackdrop(event) {
    if (!props.dismissible) return
    if (event.target === event.currentTarget) emit('close')
}

function onKey(event) {
    if (event.key === 'Escape' && props.open && props.dismissible) emit('close')
}

/** 열리면 첫 버튼으로 포커스를 옮긴다. 마우스로 쓰더라도 초점이 어디 있는지는 보여야 한다. */
watch(
    () => props.open,
    async (isOpen) => {
        if (!isOpen) return
        await nextTick()
        box.value?.querySelector('button')?.focus()
    },
)

onMounted(() => document.addEventListener('keydown', onKey))
onBeforeUnmount(() => document.removeEventListener('keydown', onKey))
</script>

<template>
    <Teleport to="body">
        <div v-if="open" :aria-label="title" class="modal" role="dialog" aria-modal="true"
             @click="onBackdrop">
            <div ref="box" :class="['modal__box', size !== 'default' ? `modal__box--${size}` : '']">
                <div v-if="title" class="modal__head">
                    <h5>{{ title }}</h5>
                    <p v-if="subtitle" class="modal__who">{{ subtitle }}</p>
                </div>
                <div class="modal__body">
                    <slot/>
                </div>
                <div class="modal__foot">
                    <slot name="foot"/>
                </div>
            </div>
        </div>
    </Teleport>
</template>
