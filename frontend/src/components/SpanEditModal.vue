<script setup>
/**
 * 출결 고치기 — 구분 · 종류 · 기간.
 *
 * 안에 들어가는 것은 화면 위쪽과 **같은 축 카드**다. 실무에서 가장 잦은 수정이
 * 이것이라, 편집기를 두 벌 만들면 한쪽에만 규칙이 붙어 갈라진다.
 *
 * 저장을 눌러야 반영된다. 복사본을 고치므로 취소하면 아무것도 바뀌지 않는다.
 */
import {ref, watch} from 'vue'
import AxisCard from './AxisCard.vue'
import {UiButton, UiModal} from './ui'

const props = defineProps({
    open: {type: Boolean, default: false},
    span: {type: Object, default: null},
    reasons: {type: Array, default: () => []},
    types: {type: Array, default: () => []},
    maxSlot: {type: Number, default: 7},
})

const emit = defineEmits(['save', 'close'])

const draft = ref({reasonId: null, typeId: null, slots: []})

watch(
    () => [props.open, props.span],
    () => {
        if (!props.open || !props.span) return
        draft.value = {
            reasonId: props.span.reasonId ?? null,
            typeId: props.span.typeId ?? null,
            slots: slotsOf(props.span),
        }
    },
    {immediate: true},
)

/**
 * 저장된 구간을 다시 축 카드의 선택으로 되돌린다.
 *
 * 지각은 끝만, 조퇴는 시작만 고르게 되어 있으므로 그쪽 값만 집어낸다.
 * 결석은 하루 종일이라 고를 것이 없다.
 */
function slotsOf(span) {
    if (span.slotPrompt === 'none') return []
    if (span.slotPrompt === 'end') return span.endSlot ? [span.endSlot] : []
    if (span.slotPrompt === 'start') return span.startSlot ? [span.startSlot] : []
    if (span.slotPrompt === 'multi') {
        const from = Number(span.startSlot)
        const to = Number(span.endSlot)
        if (!Number.isInteger(from) || !Number.isInteger(to)) return []
        const out = []
        for (let n = from; n <= to; n += 1) out.push(String(n))
        return out
    }
    return span.startSlot ? [span.startSlot] : []
}
</script>

<template>
    <UiModal :open="open" :subtitle="span ? `${span.dateLabel} · ${span.number}번 ${span.name}` : ''"
             size="wide" title="출결 고치기" @close="emit('close')">
        <AxisCard v-model="draft" :max-slot="maxSlot" :reasons="reasons" :types="types"/>

        <template #foot>
            <UiButton size="wide" @click="emit('close')">취소</UiButton>
            <UiButton size="wide" variant="primary" @click="emit('save', draft)">저장</UiButton>
        </template>
    </UiModal>
</template>
