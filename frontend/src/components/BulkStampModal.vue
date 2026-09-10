<script setup>
/**
 * 여러 날에 같은 조합을 찍는다.
 *
 * **화면(탭)으로 만들지 않는다.** 기간은 화면이 아니라 입력의 한 축이라, 오늘의 출결에서
 * 열리는 창이어야 한다. 탭으로 만들면 "어제 것도 여기서 넣나"를 매번 되묻게 된다.
 *
 * 주말과 등록된 휴업일은 Rust가 미리 빼 준다. 그래도 남은 날 중에 학교가 쉰 날이
 * 있을 수 있으므로 **교사가 미리보기에서 지운다** — 학사일정을 앱이 알 수 없다는
 * 사실의 인정이다.
 */
import {computed, ref, watch} from 'vue'
import {UiButton, UiModal, UiNotice} from './ui'

const props = defineProps({
    open: {type: Boolean, default: false},
    student: {type: Object, default: null},
    phrase: {type: String, default: ''},
    preview: {type: Function, required: true},
})

const emit = defineEmits(['apply', 'close'])

const from = ref('')
const to = ref('')
const days = ref([])
const dropped = ref(new Set())
const error = ref('')

const chosen = computed(() => days.value.filter((d) => !dropped.value.has(d.date)))

watch(
    () => props.open,
    (isOpen) => {
        if (!isOpen) return
        days.value = []
        dropped.value = new Set()
        error.value = ''
    },
)

async function look() {
    error.value = ''
    if (!from.value || !to.value) {
        error.value = '시작일과 마지막 날을 골라주세요.'
        return
    }
    if (from.value > to.value) {
        error.value = '시작일이 마지막 날보다 뒤입니다.'
        return
    }
    try {
        days.value = await props.preview(from.value, to.value)
        dropped.value = new Set()
        if (days.value.length === 0) error.value = '그 사이에 셀 수 있는 날이 없습니다.'
    } catch (e) {
        error.value = String(e)
    }
}

function toggle(date) {
    const next = new Set(dropped.value)
    if (next.has(date)) next.delete(date)
    else next.add(date)
    dropped.value = next
}
</script>

<template>
    <UiModal :open="open"
             :subtitle="student ? `${student.number}번 ${student.name} · ${phrase}` : ''"
             size="wide" title="여러 날에 같은 출결 찍기" @close="emit('close')">
        <div class="filters">
            <span class="filters__label">기간</span>
            <input v-model="from" class="field num" type="date"/>
            <span class="filters__label">부터</span>
            <input v-model="to" class="field num" type="date"/>
            <UiButton size="tight" @click="look">날짜 보기</UiButton>
        </div>

        <UiNotice :text="error" kind="warn"/>

        <div v-if="days.length" class="bulk">
            <p class="set__hint">
                주말과 휴업일은 이미 빠졌습니다. 학교가 쉰 날이 남아 있으면 눌러서 빼주세요.
            </p>
            <div class="bulk__days">
                <button v-for="day in days" :key="day.date"
                        :class="['pick', dropped.has(day.date) ? '' : 'is-on']"
                        type="button" @click="toggle(day.date)">
                    <span class="num">{{ day.label }}</span>
                    <span v-if="day.hasExisting" class="bulk__mark">이미 있음</span>
                </button>
            </div>
        </div>

        <template #foot>
            <p v-if="chosen.length" class="modal__note focus__note">
                <b class="num">{{ chosen.length }}</b>일에 찍습니다.
            </p>
            <UiButton size="wide" @click="emit('close')">취소</UiButton>
            <UiButton :disabled="chosen.length === 0" size="wide" variant="primary"
                      @click="emit('apply', {from, to, days: chosen.map((d) => d.date)})">
                찍기
            </UiButton>
        </template>
    </UiModal>
</template>

<style scoped>
.bulk {
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.bulk__days {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
}

/* 이미 그날 구간이 있는 학생임을 알린다. 막지는 않는다 — 하루 2구간은 정상이다. */
.bulk__mark {
    margin-left: 7px;
    color: var(--c-warn);
    font-weight: 700;
}
</style>
