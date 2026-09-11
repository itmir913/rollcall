<script setup>
/**
 * 오늘의 출결 — 찍는 곳.
 *
 * 구분 · 종류 · 기간을 고정해 두고 번호를 누르면 **그 자리에서 저장**된다.
 * 확인 대화상자는 없다. 같은 조합을 다시 누르면 그 건이 취소된다 —
 * 실수로 두 번 누른 것과 "방금 찍은 것을 무르고 싶다"가 같은 동작이다.
 *
 * 아래 목록에 방금 찍힌 줄이 바로 쌓이고, 사유와 태그는 그 줄에서 열어 고친다.
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
import BulkStampModal from '../components/BulkStampModal.vue'
import {stampPhrase} from '../services/phrase'
import {UiButton, UiLedger, UiNotice, UiPage, UiTrashIcon} from '../components/ui'

const app = useAppStore()
const axis = useAxisStore()
const day = useDayStore()
const school = useSchoolStore()

const expanded = ref(null)
const fixing = ref(null)
const dropping = ref(null)
const bulkFor = ref(null)

/**
 * 여러 날 모드. 켜면 격자를 누를 때 찍는 대신 기간 창이 열린다.
 *
 * 번호를 두 벌 늘어놓지 않으려는 것이다 — 격자가 곧 우리 반이고, 학생을 고르는
 * 방법이 화면마다 달라지면 그때마다 다시 배워야 한다.
 */
const bulkMode = ref(false)

/** 여러 날 창의 머리에 적을 한 줄. 지금 고른 조합을 그대로 보여준다. */
const draftPhrase = computed(() =>
    stampPhrase({
        reasonLabel: axis.reasons.find((r) => r.id === day.draft.reasonId)?.label,
        typeLabel: axis.types.find((t) => t.id === day.draft.typeId)?.label,
        slotPrompt: axis.slotPromptOf(day.draft.typeId),
        slots: day.draft.slots,
    }),
)

const draft = computed({
    get: () => day.draft,
    set: (value) => {
        day.draft = value
    },
})

const spans = computed(() => day.spans)

async function stamp(studentId) {
    if (bulkMode.value) {
        bulkFor.value = day.rows.find((r) => r.studentId === studentId) ?? null
        return
    }
    await day.stamp(studentId).catch(() => {
    })
}

async function saveFix(patch) {
    const target = fixing.value
    fixing.value = null
    if (target) await day.editSpan(target.id, patch).catch(() => {
    })
}

/** 여러 날 찍기. 교사가 미리보기에서 뺀 날은 넘기지 않는다. */
async function applyBulk({from, to}) {
    const student = bulkFor.value
    bulkFor.value = null
    bulkMode.value = false
    if (!student) return
    await day.applyBulk(student.studentId, from, to).catch(() => {
    })
}

async function confirmDrop() {
    const target = dropping.value
    dropping.value = null
    if (target) await day.deleteSpan(target.id).catch(() => {
    })
}

// 한쪽이 실패해도 나머지는 그린다. 실패는 각 스토어의 error에 담겨 화면 아래
// UiNotice가 그대로 보여준다 — 사유 후보와 태그가 빈 것이 "없다"로 읽히면 안 된다.
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
            <UiButton :variant="bulkMode ? 'primary' : 'default'" @click="bulkMode = !bulkMode">
                여러 날
            </UiButton>
        </template>

        <AxisCard v-model="draft" :max-slot="app.maxSlot" :reasons="axis.reasons"
                  :types="axis.types"/>

        <p v-if="bulkMode" class="notice notice--warn">
            여러 날 모드입니다. 학생을 누르면 기간을 고르는 창이 열립니다 —
            기간은 화면이 아니라 입력의 한 축이라 탭으로 만들지 않았습니다.
        </p>

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
                         @fix="fixing = span"
                         @toggle-expand="expanded = expanded === span.id ? null : span.id"
                         @update-memo="day.setMemo(span.id, $event)"
                         @update-tag="day.setTag(span.id, $event)">
                    <template #tail>
                        <span class="row__acts">
                            <UiButton aria-label="지우기" icon title="지우기" variant="danger"
                                      @click="dropping = span">
                                <UiTrashIcon/>
                            </UiButton>
                        </span>
                    </template>
                </SpanRow>
            </template>
        </UiLedger>

        <UiNotice :text="day.error" kind="error"/>
        <!-- 무르기가 거부된 이유. 오류가 아니므로 경고 위계로 적는다 — 교사가 잘못한
             것이 없고, 화면은 아무 일도 일어나지 않은 것처럼 보이기 때문이다. -->
        <UiNotice :text="day.notice" kind="warn"/>
        <UiNotice :text="axis.error" kind="error"/>
        <UiNotice :text="school.error" kind="error"/>

        <SpanEditModal :max-slot="app.maxSlot" :open="Boolean(fixing)" :reasons="axis.reasons"
                       :span="fixing" :types="axis.types"
                       @close="fixing = null" @save="saveFix"/>
        <SpanDeleteModal :open="Boolean(dropping)" :span="dropping"
                         @close="dropping = null" @confirm="confirmDrop"/>
        <BulkStampModal :open="Boolean(bulkFor)" :phrase="draftPhrase"
                        :preview="(from, to) => day.previewBulk(from, to)"
                        :student="bulkFor"
                        @apply="applyBulk" @close="bulkFor = null"/>
    </UiPage>
</template>
