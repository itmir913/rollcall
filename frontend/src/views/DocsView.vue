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
import {useDownloadStore} from '../stores/download'

const app = useAppStore()
const axis = useAxisStore()
const day = useDayStore()
const pending = usePendingStore()
const school = useSchoolStore()
const download = useDownloadStore()

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

// 한쪽이 실패해도 나머지는 그린다. 실패는 각 스토어의 error에 담겨 화면 아래
// UiNotice가 그대로 보여준다 — 사유 후보와 태그가 빈 것이 "없다"로 읽히면 안 된다.
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
/** 내보내기 실패를 화면에 남긴다. 눌러도 아무 일이 없는 단추를 두지 않는다. */
function saveCsv(kind, args, suggested) {
    download.csv(kind, args, suggested).catch(() => {
    })
}
// 템플릿에서 스토어를 바로 부르면 실패가 처리되지 않은 거부로 흩어진다.
// 메모는 이 앱이 "무엇을 받기로 했는지"를 담는 유일한 자리라 조용히 사라지면 안 된다.
async function setMemo(span, memo) {
    try {
        await day.setMemo(span.id, memo)
        span.memo = memo
    } catch {
        // day.error에 담겨 화면에 나온다.
    }
}

async function setTag(span, tagId) {
    try {
        await day.setTag(span.id, tagId)
        await pending.fetchDocs()
    } catch {
        // day.error에 담겨 화면에 나온다.
    }
}
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn" text="먼저 설정에서 학급과 명렬표를 넣어주세요."/>

    <UiPage v-else
            :subtitle="`못 받은 것 ${pending.docLeft}건 · 모은 명단 ${pending.docRows.length}건`"
            title="서류 미제출자">
        <template #actions>
            <UiButton variant="download"
                      @click="saveCsv('pending', {classId: app.classId, kind: 'doc', today: app.today}, '서류_미제출자.csv')">
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
                     @update-memo="setMemo(span, $event)"
                     @update-tag="setTag(span, $event)">
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
        <UiNotice :text="day.error" kind="error"/>
        <UiNotice :text="axis.error" kind="error"/>
        <UiNotice :text="school.error" kind="error"/>

        <SpanEditModal :max-slot="app.maxSlot" :open="Boolean(fixing)" :reasons="axis.reasons"
                       :span="fixing" :types="axis.types"
                       @close="fixing = null" @save="saveFix"/>
        <UiNotice :text="download.error" kind="error"/>
        <UiNotice :text="download.done" kind="ok"/>
    </UiPage>
</template>
