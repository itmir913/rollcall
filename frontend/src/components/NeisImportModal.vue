<script setup>
/**
 * 나이스 파일 열기. **교체가 아니라 차분이다.**
 *
 * 같은 것은 그대로 두고, 없는 것은 추가하고, 다른 것은 어느 쪽을 남길지 교사가 선택한다.
 * **앱에만 있는 기록은 지우지 않는다** — 나이스에 아직 안 넣은 것이 이 앱을 쓰는 이유라,
 * 파일에 없다는 것이 삭제 근거가 될 수 없다. 세어서 알리기만 한다.
 *
 * 왼쪽이 내 기록, 오른쪽이 나이스다. NEIS 검증 화면과 **같은 배치**여야 한다 —
 * 두 화면이 같은 것을 반대로 놓으면 매번 어느 쪽이 나이스인지 다시 읽어야 한다.
 *
 * 갈래마다 목록을 두되 비어도 자리를 지운다 — 목록이 생겼다 사라지면 다음에 누를
 * 단추의 자리가 매번 달라진다.
 */
import {computed} from 'vue'
import {useNeisImportStore} from '../stores/neisImport'
import {UiButton, UiLedger, UiModal, UiNotice, UiToggle} from './ui'

defineProps({
    open: {type: Boolean, default: false},
})

const emit = defineEmits(['apply', 'close'])
const store = useNeisImportStore()

const preview = computed(() => store.preview)
const items = (verdict) => preview.value?.items.filter((i) => i.verdict === verdict) ?? []

const GROUPS = [
    {
        kind: 'add',
        verdict: 'add',
        title: '앱에 없는 기록',
        hint: '나이스에는 있는데 이 앱에 없다. 넣을 것을 선택한다',
        tone: 'is-only',
        on: '넣는다',
        off: '넘긴다',
        emptyText: '나이스에만 있는 기록이 없습니다.',
    },
    {
        kind: 'replace',
        verdict: 'differ',
        title: '서로 다름',
        hint: '어느 쪽을 남길지 선택한다. 선택하지 않으면 내 기록이 그대로 남는다',
        tone: 'is-diff',
        on: '나이스 것으로',
        off: '내 것 유지',
        emptyText: '어긋난 것이 없습니다.',
    },
]

/** 이 갈래를 전부 선택하거나 전부 푼다. 서른 줄을 하나씩 누르게 하지 않는다. */
function toggleAll(group) {
    const keys = items(group.verdict).map((i) => i.key)
    const allPicked = keys.length > 0 && keys.every((k) => store.picked[group.kind].has(k))
    store.pickAll(group.kind, allPicked ? [] : keys)
}
</script>

<template>
    <UiModal :open="open" :subtitle="preview
                 ? `${store.meta?.parser}로 읽음 · ${preview.from} ~ ${preview.to}`
                 : ''"
             size="wide" title="나이스 파일 열기" @close="emit('close')">
        <template v-if="preview">
            <div class="strip">
                <div class="strip__cell is-ok">
                    <b class="num">{{ preview.same }}</b><span>같음</span>
                </div>
                <div class="strip__cell is-warn">
                    <b class="num">{{ preview.add }}</b><span>앱에 없음</span>
                </div>
                <div class="strip__cell is-bad">
                    <b class="num">{{ preview.differ }}</b><span>다름</span>
                </div>
                <div class="strip__cell is-bad">
                    <b class="num">{{ preview.unreadable }}</b><span>못 읽음</span>
                </div>
                <div class="strip__cell">
                    <b class="num">{{ preview.onlyMine }}</b>
                    <span>앱에만</span>
                </div>
            </div>

            <UiNotice :text="store.meta?.merged
                          ? `나이스가 하루 두 구간을 한 줄로 합쳐 내보낸 것이 ${store.meta.merged}건 있습니다. 나누어 두었으니 구분이 맞는지 확인해주세요.`
                          : ''"
                      kind="warn"/>
            <UiNotice :text="store.meta?.unknownCodes?.length
                          ? `모르는 출결 표기입니다 — ${store.meta.unknownCodes.join(', ')}`
                          : ''"
                      kind="warn"/>
            <UiNotice :text="store.meta?.skipped?.length
                          ? `읽지 못한 줄이 있습니다 — ${store.meta.skipped.map((s) => `${s.line}번째 줄`).join(', ')}`
                          : ''"
                      kind="warn"/>

            <UiLedger :empty="preview.same === 0" :note="`${preview.same}건`"
                      empty-text="나이스와 똑같은 기록이 없습니다."
                      hint="내용은 그대로 둔다. 나이스에 있다는 사실만 표시한다"
                      title="이미 같음">
                <div class="set__row">
                    <span class="set__label">나이스 등재로 표시</span>
                    <UiToggle v-model="store.markNeis" off-label="표시하지 않는다"
                              on-label="표시한다"/>
                </div>
            </UiLedger>

            <UiLedger v-for="group in GROUPS" :key="group.kind"
                      :empty="items(group.verdict).length === 0" :empty-text="group.emptyText"
                      :hint="group.hint" :note="`${items(group.verdict).length}건`"
                      :title="group.title">
                <template #actions>
                    <UiButton size="tight" @click="toggleAll(group)">전부 선택</UiButton>
                </template>
                <div class="cmp__head">
                    <span>내 기록</span><span>나이스</span><span>할 일</span>
                </div>
                <div v-for="item in items(group.verdict)" :key="item.key"
                     :class="['cmp', group.tone]">
                    <span v-if="item.myAxis" class="side">
                        <span class="side__who">
                            <b><span class="num">{{ item.number }}</span>번 {{ item.name }}</b>
                            <span class="num">{{ item.dateLabel }}</span>
                        </span>
                        <span class="side__what">
                            <b>{{ item.myAxis }}</b> · <span class="num">{{ item.mySpan }}</span>
                        </span>
                    </span>
                    <span v-else class="side side--empty">기록 없음</span>

                    <span class="side">
                        <span class="side__who">
                            <b><span class="num">{{ item.number }}</span>번 {{ item.name }}</b>
                            <span class="num">{{ item.dateLabel }}</span>
                        </span>
                        <span class="side__what">
                            <b>{{ item.theirAxis }}</b> ·
                            <span class="num">{{ item.theirSpan }}</span>
                            <span v-if="item.detail"> · {{ item.detail }}</span>
                        </span>
                    </span>

                    <UiToggle :model-value="store.picked[group.kind].has(item.key)"
                              :off-label="group.off" :on-label="group.on"
                              @update:model-value="store.toggle(group.kind, item.key)"/>
                </div>
            </UiLedger>

            <UiLedger :empty="items('unreadable').length === 0"
                      :note="`${items('unreadable').length}건`"
                      empty-text="전부 읽었습니다."
                      hint="지어내지 않는다. 무엇이 막았는지만 알린다"
                      title="읽지 못한 줄">
                <div v-for="item in items('unreadable')" :key="item.key" class="cmp cmp--why">
                    <span class="side side--empty">
                        <span class="num">{{ item.number }}번 · {{ item.date }}</span>
                    </span>
                    <span class="side">
                        <span class="side__what">{{ item.why }}</span>
                    </span>
                    <span class="cmp__verdict">못 읽음</span>
                </div>
            </UiLedger>

            <p class="set__hint">
                앱에만 있는 기록은 지우지 않는다. 나이스에 아직 넣지 않은 것이므로
                <b>NEIS 미등재</b> 목록에 그대로 남는다.
            </p>
        </template>

        <p v-else class="ledger__empty">{{ store.error || '파일을 읽는 중입니다.' }}</p>

        <template #foot>
            <p v-if="preview" class="modal__note">
                넣기 <b class="num">{{ store.picked.add.size }}</b> ·
                고치기 <b class="num">{{ store.picked.replace.size }}</b>
            </p>
            <UiButton size="wide" @click="emit('close')">취소</UiButton>
            <UiButton :disabled="!store.hasWork || store.busy" size="wide" variant="primary"
                      @click="emit('apply')">
                저장
            </UiButton>
        </template>
    </UiModal>
</template>

<style scoped>
/* 못 읽은 줄은 짝이 없다. 두 칸을 그대로 두되 왼쪽을 비운 상자로 남긴다 —
   칸을 없애면 위 목록과 열이 어긋나 눈이 다시 자리를 찾아야 한다. */
.cmp--why {
    border-left-color: var(--c-danger);
}
</style>
