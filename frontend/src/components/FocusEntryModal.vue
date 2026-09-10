<script setup>
/**
 * 집중 등재 — 나이스 화면을 옆에 띄워 두고 한 명씩 옮겨 적는 모드.
 *
 * **ESC와 바깥 클릭으로 닫히지 않는다.** 저장인지 취소인지 반드시 고르게 한다.
 *
 * 여기서 고친 것은 **저장을 눌러야** 반영된다. 나이스 쪽 저장이 실패하는 날이 있는데,
 * 앱만 먼저 등재로 바뀌면 두 곳이 어긋난 채 남는다. 그래서 복사본을 고치고,
 * 저장할 때 한꺼번에 넘긴다.
 */
import {computed, ref, watch} from 'vue'
import AxisCard from './AxisCard.vue'
import {UiButton, UiModal} from './ui'

const props = defineProps({
    open: {type: Boolean, default: false},
    day: {type: Object, default: null},
    reasons: {type: Array, default: () => []},
    types: {type: Array, default: () => []},
    tags: {type: Array, default: () => []},
    memos: {type: Array, default: () => []},
    maxSlot: {type: Number, default: 7},
})

const emit = defineEmits(['save', 'cancel'])

const items = ref([])
const index = ref(0)

watch(
    () => [props.open, props.day],
    () => {
        if (!props.open || !props.day) return
        // 원본이 아니라 복사본을 고친다. 취소하면 이 배열을 버리는 것으로 끝난다.
        items.value = props.day.spans
            .filter((s) => !s.neisDone)
            .map((s) => ({
                id: s.id,
                number: s.number,
                name: s.name,
                reasonId: s.reasonId ?? null,
                typeId: s.typeId ?? null,
                slots: slotsOf(s),
                memo: s.memo ?? '',
                tagId: s.tagId ?? null,
                touched: false,
            }))
        index.value = 0
    },
    {immediate: true},
)

const current = computed(() => items.value[index.value] ?? null)

const draft = computed({
    get: () => ({
        reasonId: current.value?.reasonId ?? null,
        typeId: current.value?.typeId ?? null,
        slots: current.value?.slots ?? [],
    }),
    set: (value) => {
        if (!current.value) return
        Object.assign(current.value, value, {touched: true})
    },
})

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

function move(step) {
    if (current.value) current.value.touched = true
    const next = index.value + step
    if (next < 0 || next >= items.value.length) return
    index.value = next
}
</script>

<template>
    <UiModal :dismissible="false" :open="open"
             :subtitle="day ? `${day.dateLabel} · ${index + 1} / ${items.length}명` : ''"
             size="focus" title="한 명씩 등재">
        <div v-if="current" class="focus__who">
            <UiButton :disabled="index === 0" @click="move(-1)">◀ 이전</UiButton>
            <span class="focus__name"><span class="num">{{ current.number }}</span>번 {{ current.name }}</span>
            <UiButton :disabled="index >= items.length - 1" @click="move(1)">다음 ▶</UiButton>
        </div>

        <AxisCard v-if="current" v-model="draft" :max-slot="maxSlot" :reasons="reasons"
                  :types="types"/>

        <div v-if="current" class="edit">
            <textarea v-model="current.memo" class="edit__area" placeholder="사유를 입력하세요"
                      rows="2"></textarea>
            <div class="edit__row">
                <span class="edit__label">후보</span>
                <button v-for="word in memos" :key="word"
                        :class="['chip', current.memo === word ? 'is-on' : '']"
                        type="button" @click="current.memo = current.memo === word ? '' : word">
                    {{ word }}
                </button>
            </div>
            <div class="edit__row">
                <span class="edit__label">태그</span>
                <button v-for="tag in tags" :key="tag.id"
                        :class="['chip', 'chip--tag', current.tagId === tag.id ? 'is-on' : '']"
                        type="button"
                        @click="current.tagId = current.tagId === tag.id ? null : tag.id">
                    {{ tag.name }}
                </button>
            </div>
        </div>

        <div class="focus__dots">
            <span v-for="(item, i) in items" :key="item.id"
                  :class="['focus__dot', i === index ? 'is-now' : item.touched ? 'is-done' : '']"></span>
        </div>

        <template #foot>
            <p class="modal__note focus__note">
                저장을 눌러야 반영된다. 취소하면 이 창에서 고친 것이 전부 사라진다.
            </p>
            <UiButton size="wide" @click="emit('cancel')">취소</UiButton>
            <UiButton size="wide" variant="primary" @click="emit('save', items)">저장하고 닫기</UiButton>
        </template>
    </UiModal>
</template>
