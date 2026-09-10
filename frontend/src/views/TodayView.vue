<script setup>
/**
 * 오늘의 출결 — 찍는 곳.
 *
 * 구분 · 종류 · 기간을 고정해 두고 번호를 누르면 **그 자리에서 저장**된다.
 * 확인 대화상자는 없다. 같은 조합을 다시 누르면 그 건이 취소된다 —
 * 실수로 두 번 누른 것과 "방금 찍은 것을 무르고 싶다"가 같은 동작이다.
 *
 * 아래 목록에 방금 찍힌 줄이 바로 쌓이고, 사유와 태그는 그 줄에서 펼쳐 고친다.
 * 구분 · 기간은 그 자리를 눌러 수정 모달로 고친다 — 화면 위쪽과 같은 축 카드다.
 */
import {computed, onMounted, ref} from 'vue'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {useDayStore} from '../stores/day'
import {useSchoolStore} from '../stores/school'
import AxisCard from '../components/AxisCard.vue'
import SeatGrid from '../components/SeatGrid.vue'
import SpanRow from '../components/SpanRow.vue'
import SpanDeleteModal from '../components/SpanDeleteModal.vue'
import SpanEditModal from '../components/SpanEditModal.vue'
import {UiButton, UiLedger, UiNotice, UiPage} from '../components/ui'

const app = useAppStore()
const axis = useAxisStore()
const day = useDayStore()
const school = useSchoolStore()

const expanded = ref(null)
const fixing = ref(null)
const dropping = ref(null)

const draft = computed({
    get: () => day.draft,
    set: (value) => {
        day.draft = value
    },
})

const spans = computed(() => day.spans)

async function stamp(studentId) {
    await day.stamp(studentId).catch(() => {
    })
}

async function saveFix(patch) {
    const target = fixing.value
    fixing.value = null
    if (target) await day.editSpan(target.id, patch).catch(() => {
    })
}

async function confirmDrop() {
    const target = dropping.value
    dropping.value = null
    if (target) await day.deleteSpan(target.id).catch(() => {
    })
}

onMounted(async () => {
    if (!app.ready) return
    if (!day.date) day.setDate(app.today)
    await Promise.all([
        day.fetchGrid().catch(() => {
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

    <UiPage v-else :subtitle="`재학 ${day.rows.length}명 · 오늘 기록 ${day.recorded}명`"
            :title="day.dateLabel">
        <template #actions>
            <UiButton @click="day.move(-1)">◀ 어제</UiButton>
            <input :value="day.date" class="field num" type="date"
                   @change="day.setDate($event.target.value); day.fetchGrid()"/>
            <UiButton @click="day.move(1)">내일 ▶</UiButton>
        </template>

        <AxisCard v-model="draft" :max-slot="app.maxSlot" :reasons="axis.reasons"
                  :types="axis.types"/>

        <SeatGrid :busy="day.busy" :rows="day.rows" @stamp="stamp"/>

        <div class="legend">
            <span><i class="sw-ok"></i>결석</span>
            <span><i class="sw-bad"></i>지각 · 조퇴 · 결과</span>
            <span><i class="sw-warn"></i>구분 · 종류 미정</span>
            <span><i class="sw-line"></i>정상 출석</span>
        </div>

        <UiLedger :empty="spans.length === 0" :note="`${spans.length}건`"
                  empty-text="오늘 기록된 출결이 없습니다." class="list--today" title="오늘의 출결">
            <template v-for="span in spans" :key="span.id">
                <SpanRow :expanded="expanded === span.id" :memos="axis.memos" :span="span"
                         :tags="school.tags"
                         @drop="dropping = span"
                         @fix="fixing = span"
                         @toggle-expand="expanded = expanded === span.id ? null : span.id"
                         @update-memo="day.setMemo(span.id, $event)"
                         @update-tag="day.setTag(span.id, $event)">
                    <template #tail>
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
            </template>
        </UiLedger>

        <UiNotice :text="day.error" kind="error"/>

        <SpanEditModal :max-slot="app.maxSlot" :open="Boolean(fixing)" :reasons="axis.reasons"
                       :span="fixing" :types="axis.types"
                       @close="fixing = null" @save="saveFix"/>
        <SpanDeleteModal :open="Boolean(dropping)" :span="dropping"
                         @close="dropping = null" @confirm="confirmDrop"/>
    </UiPage>
</template>
