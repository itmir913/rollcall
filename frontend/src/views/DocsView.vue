<script setup>
/**
 * 서류 미제출자 — 월 필터를 켜고 아직 못 받은 서류를 훑는다.
 *
 * **누른 줄은 사라지지 않는다.** 목록은 "그때 모은 명단"이고, 체크하면 초록으로 바뀌어
 * 그 자리에 남는다. 방금 누른 것이 눈앞에서 사라지면 잘못 눌렀는지 확인할 방법이 없다.
 * 다시 모으려면 위의 버튼을 누른다.
 *
 * 연락처 열은 두지 않는다 — 인적 사항은 이름에 마우스를 올렸을 때 보여줄 것이라
 * 목록의 자리를 상시로 차지할 이유가 없다.
 */
import {onMounted, ref} from 'vue'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {useDayStore} from '../stores/day'
import {MONTHS, calendarYearOf} from '../services/academicYear'
import {usePendingStore} from '../stores/pending'
import {useSchoolStore} from '../stores/school'
import SpanRow from '../components/SpanRow.vue'
import SpanEditModal from '../components/SpanEditModal.vue'
import {UiButton, UiLedger, UiNotice, UiPage, UiToggle} from '../components/ui'
import {overdueLabel} from '../services/overdue'
import {exportCsv} from '../services/download'

const app = useAppStore()
const axis = useAxisStore()
const day = useDayStore()
const pending = usePendingStore()
const school = useSchoolStore()

const expanded = ref(null)
const fixing = ref(null)

async function pickMonth(month) {
    pending.month = pending.month === month ? null : month
    pending.year = calendarYearOf(app.currentYear?.year ?? 0, month)
    await pending.fetchDocs().catch(() => {
    })
}

async function saveFix(patch) {
    const target = fixing.value
    fixing.value = null
    if (!target) return
    await day.editSpan(target.id, patch).catch(() => {
    })
    await pending.fetchDocs()
}

onMounted(async () => {
    if (!app.ready) return
    await Promise.all([
        pending.fetchDocs().catch(() => {
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

    <UiPage v-else
            :subtitle="`못 받은 것 ${pending.docLeft}건 · 모은 명단 ${pending.docRows.length}건`"
            title="서류 미제출자">
        <template #actions>
            <UiButton variant="download"
                      @click="exportCsv('pending', {...app.scope, kind: 'doc', today: app.today}, '서류_미제출자.csv')">
                미제출자 CSV
            </UiButton>
        </template>

        <div class="filters">
            <span class="filters__label">월</span>
            <button v-for="month in MONTHS" :key="month"
                    :class="['pick', 'pick--slot', pending.month === month ? 'is-on' : '']"
                    type="button" @click="pickMonth(month)">
                {{ month }}
            </button>
            <span class="filters__gap"></span>
            <UiButton size="tight" @click="pending.fetchDocs()">못 받은 것만 다시 모으기</UiButton>
        </div>

        <UiLedger :empty="pending.docRows.length === 0" class="list--docs"
                  empty-text="못 받은 서류가 없습니다."
                  hint="마감이 지난 것부터" title="서류 미제출">
            <SpanRow v-for="span in pending.docRows" :key="span.id"
                     :expanded="expanded === span.id" :memos="axis.memos" :span="span"
                     :tags="school.tags"
                     @fix="fixing = span"
                     @toggle-expand="expanded = expanded === span.id ? null : span.id"
                     @update-memo="day.setMemo(span.id, $event); span.memo = $event"
                     @update-tag="day.setTag(span.id, $event); pending.fetchDocs()">
                <template #tail>
                    <span class="row__date num">{{ span.dateLabel }}</span>
                    <span class="row__due num">
                        {{ span.docDue ? `마감 ${span.docDue}` : '마감 없음' }}
                        <b class="row__flag">{{ overdueLabel(span) }}</b>
                    </span>
                    <span class="row__mark">
                        <UiToggle :model-value="span.docDone" off-label="서류 미제출"
                                  on-label="서류 제출"
                                  @update:model-value="pending.setDoc(span.id, $event)"/>
                    </span>
                </template>
            </SpanRow>
        </UiLedger>

        <UiNotice :text="pending.error" kind="error"/>

        <SpanEditModal :max-slot="app.maxSlot" :open="Boolean(fixing)" :reasons="axis.reasons"
                       :span="fixing" :types="axis.types"
                       @close="fixing = null" @save="saveFix"/>
    </UiPage>
</template>
