<script setup>
/**
 * 번호 격자. 재학 중이면 구간이 없어도 칸이 나온다.
 *
 * 숫자 타일 대신 번호 격자를 놓는 이유는, "오늘 기록 4명"보다 **누구인지**가
 * 담임에게 쓸모 있기 때문이다. 칸을 누르면 지금 고른 조합이 그대로 찍힌다.
 */
defineProps({
    rows: {type: Array, default: () => []},
    busy: {type: Boolean, default: false},
})

const emit = defineEmits(['stamp'])

/** 칸 색. 미정이 가장 먼저다 — 채워야 할 것이 먼저 보여야 한다. */
function seatClass(row) {
    const spans = row.spans ?? []
    if (spans.length === 0) return ''
    if (spans.some((s) => !s.complete)) return 'is-undecided'
    if (spans.some((s) => s.slotPrompt === 'none')) return 'is-marked'
    return 'is-late'
}
</script>

<template>
    <div class="seats">
        <button v-for="row in rows" :key="row.studentId"
                :class="['seat', seatClass(row)]" :disabled="busy" type="button"
                @click="emit('stamp', row.studentId)">
            <span class="seat__no num">{{ row.number }}</span>
            <span class="seat__name">{{ row.name }}</span>
        </button>
    </div>
</template>
