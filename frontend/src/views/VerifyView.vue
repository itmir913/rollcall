<script setup>
/**
 * NEIS 검증 — 나이스 파일과 이 앱의 기록을 대조한다.
 *
 * 처음에는 **파일 열기 마법사**가 뜬다. 단계를 보여 주는 이유는, 파일을 잘못 선택했을 때
 * 어디로 돌아가야 하는지가 눈에 보여야 하기 때문이다.
 *
 * 결과는 **왼쪽이 내 기록, 오른쪽이 나이스**인 두 열이다. 한쪽에만 있으면 반대쪽은
 * 빈 상자로 둔다 — 칸을 없애면 어느 쪽이 없는 것인지 매번 읽어야 한다.
 *
 * **이 화면은 고치지 않는다.** 어디가 어긋났는지만 말한다.
 */
import {computed, ref} from 'vue'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {useLogStore} from '../stores/log'
import {UiButton, UiLedger, UiNotice, UiPage} from '../components/ui'
import {compareRecords, readNeisFile, sortPairs} from '../services/neisFile'

const app = useAppStore()
const axis = useAxisStore()
const log = useLogStore()

const step = ref(1)
const meta = ref(null)
const result = ref(null)
const error = ref('')
const dragging = ref(false)
const order = ref('number')

const STEPS = ['파일 놓기', '읽은 내용 확인', '대조 결과']

const counts = computed(() => ({
    same: result.value?.same.length ?? 0,
    diff: result.value?.diff.length ?? 0,
    onlyNeis: result.value?.onlyNeis.length ?? 0,
    onlyApp: result.value?.onlyApp.length ?? 0,
}))

const fileInput = ref(null)

/**
 * 파일 선택. fs 플러그인을 통째로 열지 않고 `<input type="file">`을 쓴다 —
 * 앱이 실제로 필요한 권한은 "교사가 선택한 파일 하나 읽기"뿐이다.
 */
function pickFile() {
    fileInput.value?.click()
}

async function onPick(event) {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (!file) return
    await load(new Uint8Array(await file.arrayBuffer()))
}

async function onDrop(event) {
    dragging.value = false
    const file = event.dataTransfer?.files?.[0]
    if (!file) return
    await load(new Uint8Array(await file.arrayBuffer()))
}

async function load(bytes) {
    error.value = ''
    try {
        // 구분 · 종류는 DB에서 온다. 파일의 `질병조퇴`를 두 축으로 판별하는 후보다.
        if (axis.types.length === 0) await axis.fetchAll()
        const read = await readNeisFile(bytes, {reasons: axis.reasons, types: axis.types})
        meta.value = read.meta
        step.value = 2
        // **파일의 기간으로 대조한다.** 출결 기록 화면이 마지막에 보던 달로 맞추면
        // 6월 파일을 9월 기록과 대조하고는 전부 어긋났다고 말한다. 그 화면을 한 번도
        // 열지 않았으면 아예 빈 목록과 대조한다.
        const mine = read.meta.from
            ? await log.fetchBetween(read.meta.from, read.meta.to)
            : []
        result.value = compareRecords(mine, read.rows)
        step.value = 3
    } catch (e) {
        // 못 읽었다는 사실을 조용히 넘기지 않는다.
        error.value = String(e.message ?? e)
        step.value = 1
    }
}

/** 다른 값만 강조한다. 같은 값까지 붉으면 어디가 어긋났는지 다시 읽어야 한다. */
function mark(mine, theirs, field) {
    return mine && theirs && mine[field] !== theirs[field]
}
</script>

<template>
    <UiPage subtitle="나이스 출결 파일과 이 앱의 기록을 대조한다" title="NEIS 검증">
        <template #actions>
            <UiButton v-if="step === 3" variant="upload" @click="step = 1">다른 파일</UiButton>
        </template>

        <div class="steps">
            <template v-for="(name, i) in STEPS" :key="name">
                <span v-if="i > 0" class="step__arrow">→</span>
                <span :class="['step', step === i + 1 ? 'is-now' : step > i + 1 ? 'is-done' : '']">
                    <span class="step__no num">{{ i + 1 }}</span>{{ name }}
                </span>
            </template>
        </div>

        <UiNotice :text="error" kind="error"/>

        <div v-if="step !== 3" :class="['drop', dragging ? 'is-over' : '']"
             @dragleave.prevent="dragging = false"
             @dragover.prevent="dragging = true"
             @drop.prevent="onDrop">
            <span class="drop__title">나이스 출결 파일을 여기에 끌어다 놓으세요</span>
            <span class="drop__hint">
                일일출석부(XLS data) 또는 월별 출결 현황 <span class="num">.xlsx</span>
            </span>
            <UiButton size="wide" variant="upload" @click="pickFile">파일 선택</UiButton>
            <input ref="fileInput" accept=".xlsx" hidden type="file" @change="onPick"/>
        </div>

        <template v-if="step === 3 && result">
            <div class="import">
                <span class="import__file">{{ meta.parser }}로 읽음</span>
                <span class="import__meta">
                    <span class="num">{{ meta.from }} ~ {{ meta.to }}</span>
                    · 버린 줄 {{ meta.skipped.length }}
                </span>
            </div>

            <UiNotice :text="meta.skipped.length
                          ? `읽지 못한 줄이 있습니다 — ${meta.skipped.map((s) => `${s.line}번째 줄`).join(', ')}`
                          : ''"
                      kind="warn"/>
            <UiNotice :text="meta.unknownCodes.length
                          ? `모르는 출결 표기입니다 — ${meta.unknownCodes.join(', ')}`
                          : ''"
                      kind="warn"/>
            <UiNotice :text="meta.merged
                          ? `나이스가 하루 두 구간을 한 줄로 합쳐 내보낸 것이 ${meta.merged}건 있습니다. 나누어 두었으니 구분이 맞는지 확인해주세요.`
                          : ''"
                      kind="warn"/>

            <div class="strip">
                <div class="strip__cell is-ok"><b class="num">{{ counts.same }}</b><span>일치</span></div>
                <div class="strip__cell is-bad"><b class="num">{{ counts.diff }}</b><span>다름</span></div>
                <div class="strip__cell is-warn"><b class="num">{{ counts.onlyNeis }}</b><span>나이스에만</span></div>
                <div class="strip__cell is-warn"><b class="num">{{ counts.onlyApp }}</b><span>앱에만</span></div>
            </div>

            <div class="filters">
                <span class="filters__label">정렬</span>
                <button :class="['pick', order === 'number' ? 'is-on' : '']" type="button"
                        @click="order = 'number'">번호순
                </button>
                <button :class="['pick', order === 'date' ? 'is-on' : '']" type="button"
                        @click="order = 'date'">날짜순
                </button>
            </div>

            <UiLedger v-for="group in [
                          {key: 'diff', title: '서로 다름', hint: '양쪽을 다 보여주고 다른 값만 강조한다', pairs: result.diff, tone: 'is-diff', verdict: '다름'},
                          {key: 'onlyNeis', title: '나이스에만 있음', hint: '앱에 없는 기록이다', pairs: result.onlyNeis, tone: 'is-only', verdict: '나이스에만'},
                          {key: 'onlyApp', title: '앱에만 있음', hint: '아직 나이스에 안 넣은 것이다 · NEIS 미등재 목록과 같다', pairs: result.onlyApp, tone: 'is-only', verdict: '앱에만'},
                      ]" :key="group.key"
                      :empty="group.pairs.length === 0" :hint="group.hint"
                      :note="`${group.pairs.length}건`" :title="group.title"
                      empty-text="어긋난 것이 없습니다.">
                <div class="cmp__head">
                    <span>내 기록</span><span>나이스</span><span>어느 쪽</span>
                </div>
                <div v-for="(pair, i) in sortPairs(group.pairs, order)" :key="i"
                     :class="['cmp', group.tone]">
                    <span v-if="pair.mine" class="side">
                        <span class="side__who">
                            <b><span class="num">{{ pair.mine.number }}</span>번 {{ pair.mine.name }}</b>
                            <span class="num">{{ pair.mine.date }}</span>
                        </span>
                        <span class="side__what">
                            <b :class="mark(pair.mine, pair.theirs, 'reasonLabel') ? 'side__hl' : ''">
                                {{ pair.mine.reasonLabel || '미정' }}
                            </b>
                            <b :class="mark(pair.mine, pair.theirs, 'typeLabel') ? 'side__hl' : ''">
                                {{ pair.mine.typeLabel || '미정' }}
                            </b>
                            · <span :class="['num', mark(pair.mine, pair.theirs, 'spanText') ? 'side__hl' : '']">
                                {{ pair.mine.spanText }}
                            </span>
                        </span>
                    </span>
                    <span v-else class="side side--empty">기록 없음</span>

                    <span v-if="pair.theirs" class="side">
                        <span class="side__who">
                            <b><span class="num">{{ pair.theirs.number }}</span>번 {{ pair.theirs.name }}</b>
                            <span class="num">{{ pair.theirs.date }}</span>
                        </span>
                        <span class="side__what">
                            <b :class="mark(pair.mine, pair.theirs, 'reasonLabel') ? 'side__hl' : ''">
                                {{ pair.theirs.reasonLabel || '미정' }}
                            </b>
                            <b :class="mark(pair.mine, pair.theirs, 'typeLabel') ? 'side__hl' : ''">
                                {{ pair.theirs.typeLabel || '미정' }}
                            </b>
                            · <span :class="['num', mark(pair.mine, pair.theirs, 'spanText') ? 'side__hl' : '']">
                                {{ pair.theirs.spanText }}
                            </span>
                            <span v-if="pair.theirs.detail"> · {{ pair.theirs.detail }}</span>
                        </span>
                    </span>
                    <span v-else class="side side--empty">기록 없음</span>

                    <span class="cmp__verdict">{{ group.verdict }}</span>
                </div>
            </UiLedger>

            <p class="set__hint">
                이 화면은 고치지 않는다. 앱으로 들여오려면 출결 기록의 <b>나이스 파일 열기</b>를,
                나이스에 넣으려면 <b>NEIS 미등재</b>를 쓴다.
            </p>
        </template>
    </UiPage>
</template>
