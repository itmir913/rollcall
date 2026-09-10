<script setup>
/**
 * 출결 한 줄. **네 화면이 같은 줄을 쓴다** — 오늘의 출결 · 출결 기록 ·
 * 서류 미제출자 · NEIS 미등재. 다른 것은 맨 뒤 칸뿐이라 `tail` 슬롯으로 받는다.
 *
 * 칸 순서는 어느 화면에서나 같다:
 *   번호 → 이름 → 구분·종류 → 기간 → 사유 → (덧붙임) → 상태
 * 같은 것을 찾을 때 눈이 매번 다른 자리로 가면 화면을 옮길 때마다 다시 읽어야 한다.
 *
 * 사유는 접혀 있다가 누르면 그 줄이 커지면서 열린다. 칩을 스무 개 늘어놓으면
 * 번호 · 이름 · 출결이 오른쪽으로 밀리는데, 훑을 때 보는 것은 그 셋이다.
 */
import {ref, watch} from 'vue'

const props = defineProps({
    span: {type: Object, required: true},
    tags: {type: Array, default: () => []},
    memos: {type: Array, default: () => []},
    expanded: {type: Boolean, default: false},
    editable: {type: Boolean, default: true},
})

const emit = defineEmits(['fix', 'drop', 'toggleExpand', 'updateMemo', 'updateTag'])

const draft = ref(props.span.memo ?? '')

// 펼칠 때만 원본을 다시 읽는다. 타이핑 중에 값을 덮어쓰면 커서가 튄다.
watch(
    () => props.expanded,
    (open) => {
        if (open) draft.value = props.span.memo ?? ''
    },
)

/** 줄의 색조. 미정이거나 겹치면 주황, 결석은 초록, 나머지는 붉은 띠다. */
function tone(span) {
    if (!span.complete || span.overlapping) return 'is-warn'
    return span.slotPrompt === 'none' ? 'is-ok' : 'is-bad'
}

function commit() {
    if (draft.value !== (props.span.memo ?? '')) emit('updateMemo', draft.value)
    emit('toggleExpand')
}

function pickMemo(word) {
    draft.value = draft.value === word ? '' : word
    emit('updateMemo', draft.value)
}

function pickTag(tag) {
    emit('updateTag', props.span.tagId === tag.id ? null : tag.id)
}
</script>

<template>
    <div :class="['row', tone(span)]">
        <span class="row__no num">{{ span.number }}</span>
        <span class="row__name"><b>{{ span.name }}</b></span>

        <span class="row__what">
            <button v-if="editable" class="cellbtn cellbtn--plain" type="button"
                    @click="emit('fix')">
                {{ span.reasonLabel || '미정' }} {{ span.typeLabel || '미정' }}
            </button>
            <template v-else>{{ span.reasonLabel || '미정' }} {{ span.typeLabel || '미정' }}</template>
        </span>

        <span class="row__when num">
            <button v-if="editable" class="cellbtn cellbtn--plain" type="button"
                    @click="emit('fix')">
                {{ span.spanText }}
            </button>
            <template v-else>{{ span.spanText }}</template>
            <b v-if="span.overlapping" class="row__clash">겹침</b>
        </span>

        <span class="row__reason">
            <button :class="['cellbtn', span.memo || span.tagName ? 'has-value' : '']"
                    type="button" @click="emit('toggleExpand')">
                <b v-if="span.tagName" class="tagmark">{{ span.tagName }}</b>
                {{ span.memo || (span.tagName ? '' : '사유를 입력하세요') }}
            </button>
        </span>

        <slot name="tail"/>
    </div>

    <div v-if="expanded" class="edit">
        <textarea v-model="draft" class="edit__area" placeholder="사유를 입력하세요" rows="2"
                  @blur="emit('updateMemo', draft)"></textarea>
        <div class="edit__row">
            <span class="edit__label">후보</span>
            <button v-for="word in memos" :key="word"
                    :class="['chip', span.memo === word ? 'is-on' : '']"
                    type="button" @click="pickMemo(word)">
                {{ word }}
            </button>
        </div>
        <div class="edit__row">
            <span class="edit__label">태그</span>
            <button v-for="tag in tags" :key="tag.id"
                    :class="['chip', 'chip--tag', span.tagId === tag.id ? 'is-on' : '']"
                    type="button" @click="pickTag(tag)">
                {{ tag.name }}
            </button>
            <button class="btn btn--tight edit__done" type="button" @click="commit">완료</button>
        </div>
    </div>
</template>
