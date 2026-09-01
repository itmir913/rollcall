<script setup>
/**
 * 출결 코드 고르기. 사유(질병·출석인정·미인정·기타)로 묶어 버튼으로 펼친다.
 *
 * 드롭다운을 쓰지 않는다. 사유와 유형을 한눈에 비교해야 고를 수 있는데,
 * 드롭다운은 열기 전까지 후보가 보이지 않아 클릭이 두 번 든다.
 *
 * 코드 목록은 설정에서 늘어날 수 있으므로, 사유 이름을 하드코딩하지 않고
 * 내려온 목록에서 뽑는다.
 */
import {computed} from 'vue'

const props = defineProps({
    modelValue: {type: Number, default: null},
    codes: {type: Array, required: true},
})

const emit = defineEmits(['update:modelValue'])

/** 사유별로 묶는다. 순서는 sort_order를 따라 Rust가 정해 준 순서 그대로다. */
const groups = computed(() => {
    const map = new Map()
    for (const code of props.codes) {
        if (!map.has(code.reason)) map.set(code.reason, [])
        map.get(code.reason).push(code)
    }
    return [...map.entries()].map(([reason, items]) => ({reason, items}))
})
</script>

<template>
    <div class="code-picker">
        <div v-for="group in groups" :key="group.reason" class="code-picker__group">
            <span class="code-picker__reason">{{ group.reason }}</span>
            <div class="code-picker__row">
                <button v-for="code in group.items" :key="code.id"
                        :class="['code-picker__btn', modelValue === code.id ? 'is-on' : '']"
                        type="button"
                        @click="emit('update:modelValue', code.id)">
                    {{ code.type }}
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.code-picker {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.code-picker__group {
    display: flex;
    align-items: center;
    gap: 10px;
}

.code-picker__reason {
    color: var(--c-ink-3);
    min-width: 68px;
}

.code-picker__row {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
}

.code-picker__btn {
    min-width: 58px;
    min-height: 40px;
    padding: 0 12px;
    border-radius: 8px;
    border: 1px solid var(--c-line);
    background: var(--c-raised);
    color: var(--c-ink-2);
    cursor: pointer;
}

.code-picker__btn:hover {
    border-color: var(--c-accent);
    color: var(--c-ink);
}

.code-picker__btn.is-on {
    background: var(--c-accent);
    border-color: var(--c-accent);
    color: #fff;
    font-weight: 600;
}
</style>
