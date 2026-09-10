<script setup>
/**
 * 출결 기록 — **하루가 카드 하나**다.
 *
 * 한 카드에 한 달을 전부 넣으면 날짜 머리글이 목록 중간에 섞여 어디까지가 그날인지
 * 흐려진다. 월 필터는 3월부터 시작한다 — 학년도가 3월에 열리므로 1월·2월이 뒤에 온다.
 *
 * 서류와 나이스를 여기서 바로 체크한다. 출결을 확인하는 중에 "아, 이건 받았지"가
 * 떠오르는 곳이 바로 여기라 다른 화면으로 보내지 않는다.
 *
 * 나이스 가져오기도 여기다. 결과가 쌓이는 곳이 이 화면이라 사이드바 항목으로
 * 만들지 않는다. **일 · 월을 묻지 않는다** — 나이스 파일 안에 기간이 들어 있다.
 */
import {computed, onMounted, ref} from 'vue'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {useDayStore} from '../stores/day'
import {useLogStore} from '../stores/log'
import {MONTHS} from '../services/academicYear'
import {usePendingStore} from '../stores/pending'
import {useSchoolStore} from '../stores/school'
import SpanRow from '../components/SpanRow.vue'
import SpanDeleteModal from '../components/SpanDeleteModal.vue'
import SpanEditModal from '../components/SpanEditModal.vue'
import {UiButton, UiLedger, UiNotice, UiPage, UiToggle} from '../components/ui'
import {exportCsv} from '../services/download'

const app = useAppStore()
const axis = useAxisStore()
const day = useDayStore()
const log = useLogStore()
const pending = usePendingStore()
const school = useSchoolStore()

const expanded = ref(null)
const fixing = ref(null)
const dropping = ref(null)
const message = ref('')

const counts = computed(() => log.counts)

async function pickMonth(month) {
    log.setMonth(month, app.currentYear?.year ?? new Date().getFullYear())
    await log.fetchMonth().catch(() => {
    })
}

async function saveFix(patch) {
    const target = fixing.value
    fixing.value = null
    if (!target) return
    await day.editSpan(target.id, patch).catch(() => {
    })
    await log.fetchMonth()
}

async function confirmDrop() {
    const target = dropping.value
    dropping.value = null
    if (!target) return
    await day.deleteSpan(target.id).catch(() => {
    })
    await log.fetchMonth()
}

async function setMemo(span, memo) {
    await day.setMemo(span.id, memo).catch(() => {
    })
    span.memo = memo
}

async function setTag(span, tagId) {
    await day.setTag(span.id, tagId).catch(() => {
    })
    await log.fetchMonth()
}

async function toggleDoc(span, value) {
    await pending.setDoc(span.id, value).catch(() => {
    })
    span.docDone = value
}

async function toggleNeis(span, value) {
    await pending.setNeis(span.id, value).catch(() => {
    })
    span.neisDone = value
}

/** 나이스 파일 서식은 아직 분석하지 않았다. 정직하게 알린다. */
function importNeis() {
    message.value =
        '나이스 파일 서식은 아직 분석하지 않았습니다. 파일을 주시면 그 서식에 맞춰 붙이겠습니다.'
}

onMounted(async () => {
    if (!app.ready) return
    const now = new Date()
    log.setMonth(log.month ?? now.getMonth() + 1, app.currentYear?.year ?? now.getFullYear())
    await Promise.all([
        log.fetchMonth().catch(() => {
        }),
        axis.fetchMemos().catch(() => {
        }),
        school.fetchAll().catch(() => {
        }),
    ])
})
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn" text="먼저 설정에서 학급과 명렬표를 넣어주세요."/>

    <UiPage v-else :subtitle="`${app.currentYear?.year ?? ''}학년도 ${app.grade}학년 ${app.classNo}반`"
            title="출결 기록">
        <template #actions>
            <UiButton variant="upload" @click="importNeis">NEIS 가져오기</UiButton>
            <UiButton variant="download"
                      @click="exportCsv('spans', {...app.scope, from: `${log.year}-${String(log.month).padStart(2,'0')}-01`, to: `${log.year}-${String(log.month).padStart(2,'0')}-31`}, `출결_${log.year}-${log.month}.csv`)">
                {{ log.month }}월 CSV
            </UiButton>
        </template>

        <div class="filters">
            <span class="filters__label">월</span>
            <button v-for="month in MONTHS" :key="month"
                    :class="['pick', 'pick--slot', log.month === month ? 'is-on' : '']"
                    type="button" @click="pickMonth(month)">
                {{ month }}
            </button>
        </div>

        <div class="strip">
            <div class="strip__cell"><b class="num">{{ counts['결석'] ?? 0 }}</b><span>결석</span></div>
            <div class="strip__cell"><b class="num">{{ counts['지각'] ?? 0 }}</b><span>지각</span></div>
            <div class="strip__cell"><b class="num">{{ counts['조퇴'] ?? 0 }}</b><span>조퇴</span></div>
            <div class="strip__cell"><b class="num">{{ counts['결과'] ?? 0 }}</b><span>결과</span></div>
            <div class="strip__cell is-warn"><b class="num">{{ counts['미정'] ?? 0 }}</b><span>미정</span></div>
        </div>

        <UiNotice :text="message" kind="warn"/>

        <p v-if="log.days.length === 0" class="ledger__empty">그 달에 기록된 출결이 없습니다.</p>

        <div class="daygroup">
            <UiLedger v-for="group in log.days" :key="group.date"
                      :hint="`${group.spans.length}건 · 재학 ${group.enrolled}명`"
                      :note="'구분 · 기간 · 사유를 눌러 고친다'"
                      :title="group.dateLabel" class="list--log">
                <SpanRow v-for="span in group.spans" :key="span.id"
                         :expanded="expanded === span.id" :memos="axis.memos" :span="span"
                         :tags="school.tags"
                         @fix="fixing = span"
                         @toggle-expand="expanded = expanded === span.id ? null : span.id"
                         @update-memo="setMemo(span, $event)"
                         @update-tag="setTag(span, $event)">
                    <template #tail>
                        <span class="row__mark">
                            <UiToggle :model-value="span.docDone" off-label="서류 미제출"
                                      on-label="서류 제출"
                                      @update:model-value="toggleDoc(span, $event)"/>
                        </span>
                        <span class="row__mark">
                            <UiToggle :model-value="span.neisDone" off-label="NEIS 미등재"
                                      on-label="NEIS 등재"
                                      @update:model-value="toggleNeis(span, $event)"/>
                        </span>
                        <span class="row__acts">
                            <UiButton aria-label="지우기" icon title="지우기" variant="danger"
                                      @click="dropping = span">
                                <svg aria-hidden="true" class="icon" fill="none" stroke="currentColor"
                                     stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"
                                     viewBox="0 0 24 24">
                                    <path d="M4 7h16"/>
                                    <path d="M10 4h4a1 1 0 0 1 1 1v2H9V5a1 1 0 0 1 1-1z"/>
                                    <path d="M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12"/>
                                    <path d="M10 11v6M14 11v6"/>
                                </svg>
                            </UiButton>
                        </span>
                    </template>
                </SpanRow>
            </UiLedger>
        </div>

        <UiNotice :text="log.error" kind="error"/>

        <SpanEditModal :max-slot="app.maxSlot" :open="Boolean(fixing)" :reasons="axis.reasons"
                       :span="fixing" :types="axis.types"
                       @close="fixing = null" @save="saveFix"/>
        <SpanDeleteModal :open="Boolean(dropping)" :span="dropping"
                         @close="dropping = null" @confirm="confirmDrop"/>
    </UiPage>
</template>
