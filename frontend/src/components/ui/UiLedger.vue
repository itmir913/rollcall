<script setup>
/**
 * 장부 한 덩어리 — 머리글 한 줄과 그 아래 행들.
 *
 * 테두리 · 그림자 · 모서리는 "여기부터 다른 것"이라는 신호다. 전부에 주면 위계가
 * 사라지므로, 목록과 축 고르기만 이것을 갖고 나머지는 선으로 나눈다.
 */
defineProps({
    title: {type: String, default: ''},
    hint: {type: String, default: ''},
    note: {type: String, default: ''},
    empty: {type: Boolean, default: false},
    emptyText: {type: String, default: '항목이 없습니다.'},
})
</script>

<template>
    <div class="ledger">
        <div v-if="title || hint || $slots.actions" class="ledger__head">
            <h4 v-if="title">{{ title }}</h4>
            <span v-if="hint" class="hint">{{ hint }}</span>
            <span class="spacer"></span>
            <span v-if="note" class="hint">{{ note }}</span>
            <slot name="actions"/>
        </div>
        <p v-if="empty" class="ledger__empty">{{ emptyText }}</p>
        <slot v-else/>
        <div v-if="$slots.foot" class="ledger__foot">
            <slot name="foot"/>
        </div>
    </div>
</template>
