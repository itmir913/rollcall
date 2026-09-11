<script setup>
/**
 * 축 카드 — 구분 · 종류 · 기간 · 현재 선택된 출결.
 *
 * **이 컴포넌트는 한 벌뿐이다.** 오늘의 출결 화면 위쪽, 수정 모달, 집중 등재가
 * 모두 이것을 쓴다. 두 벌을 만들면 한쪽에만 규칙이 붙어 조용히 갈라진다.
 *
 * **카드는 움직이지 않는다.** 종류를 바꿔도 버튼은 그 자리에 그대로 있고 누를 수
 * 있는지 여부만 바뀐다. 안내 문구가 나타났다 사라지는 일도 없다 — 줄이 생겼다
 * 사라지면 다음에 누를 버튼의 위치가 매번 달라져 마우스 입력이 느려진다.
 *
 * 어떤 기간을 묻는지는 **종류가 정한다.** 그 값(`slotPrompt`)은 DB에서 온다.
 * 화면이 '결석' 같은 라벨로 분기하면 교사가 종류를 추가하는 순간 틀린다.
 */
import {computed} from 'vue'
import {UNDECIDED, UNKNOWN, allowedSlots, isMulti, keepUsable, slotList} from '../services/slots'
import {stampPhrase} from '../services/phrase'

const props = defineProps({
    /** {reasonId, typeId, slots} */
    modelValue: {type: Object, required: true},
    reasons: {type: Array, default: () => []},
    types: {type: Array, default: () => []},
    maxSlot: {type: Number, default: 7},
})

const emit = defineEmits(['update:modelValue'])

const slotPrompt = computed(
    () => props.types.find((t) => t.id === props.modelValue.typeId)?.slotPrompt ?? null,
)

const allowed = computed(() => allowedSlots(slotPrompt.value, props.maxSlot))

/** 화면에 놓이는 순서: 조회 · 1~N교시 · ? · 종례. `?`는 열린 쪽을 뜻한다. */
const slotButtons = computed(() => {
    const all = slotList(props.maxSlot)
    return [...all.slice(0, -1), UNKNOWN, all[all.length - 1]]
})

/** 기간을 아직 안 정한 상태. 결석은 고를 것이 없으므로 이것도 꺼진다. */
const undecidedOn = computed(() => props.modelValue.slots.length === 0)
const undecidedOff = computed(() => slotPrompt.value === 'none')

const phrase = computed(() =>
    stampPhrase({
        reasonLabel: props.reasons.find((r) => r.id === props.modelValue.reasonId)?.label,
        typeLabel: props.types.find((t) => t.id === props.modelValue.typeId)?.label,
        slotPrompt: slotPrompt.value,
        slots: props.modelValue.slots,
    }),
)

function update(patch) {
    emit('update:modelValue', {...props.modelValue, ...patch})
}

function pickType(typeId) {
    const prompt = props.types.find((t) => t.id === typeId)?.slotPrompt ?? null
    // 바뀐 구분에서 못 고르는 기간이면 미정으로 되돌린다. 버튼 자리는 그대로다.
    update({typeId, slots: keepUsable(props.modelValue.slots, prompt, props.maxSlot)})
}

function pickSlot(value) {
    if (value === UNDECIDED) {
        update({slots: []})
        return
    }
    const current = props.modelValue.slots
    if (isMulti(slotPrompt.value)) {
        const next = current.includes(value)
            ? current.filter((v) => v !== value)
            : [...current, value]
        update({
            slots: next.sort(
                (a, b) => slotButtons.value.indexOf(a) - slotButtons.value.indexOf(b),
            ),
        })
        return
    }
    update({slots: [value]})
}
</script>

<template>
    <div class="axis">
        <div class="axis__line">
            <span class="axis__step">종류</span>
            <button :class="['pick', modelValue.typeId === null ? 'is-on' : '']"
                    type="button" @click="pickType(null)">미정
            </button>
            <button v-for="type in types" :key="type.id"
                    :class="['pick', modelValue.typeId === type.id ? 'is-on' : '']"
                    type="button" @click="pickType(type.id)">
                {{ type.label }}
            </button>
        </div>

        <div class="axis__line">
            <span class="axis__step">구분</span>
            <button :class="['pick', modelValue.reasonId === null ? 'is-on' : '']"
                    type="button" @click="update({reasonId: null})">미정
            </button>
            <button v-for="reason in reasons" :key="reason.id"
                    :class="['pick', modelValue.reasonId === reason.id ? 'is-on' : '']"
                    type="button" @click="update({reasonId: reason.id})">
                {{ reason.label }}
            </button>
        </div>

        <div class="axis__line">
            <span class="axis__step">기간</span>
            <button :class="['pick', undecidedOn && !undecidedOff ? 'is-on' : '']"
                    :disabled="undecidedOff" type="button" @click="pickSlot(UNDECIDED)">
                미정
            </button>
            <span class="axis__sep"></span>
            <button v-for="slot in slotButtons" :key="slot"
                    :class="['pick', 'pick--slot', modelValue.slots.includes(slot) ? 'is-on' : '']"
                    :disabled="!allowed.includes(slot)"
                    :title="slot === UNKNOWN ? '언제인지 모를 때' : null"
                    type="button" @click="pickSlot(slot)">
                {{ slot }}
            </button>
        </div>

        <p class="stamp">현재 선택된 출결 — <b>{{ phrase }}</b></p>
    </div>
</template>
