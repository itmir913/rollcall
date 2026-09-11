<script setup>
/**
 * NEIS 미등재 — 나이스는 **하루씩** 입력한다.
 *
 * 그래서 이 화면은 날짜별 카드다. 나이스에서 그 날짜를 열어 두고 카드 하나를 위에서
 * 아래로 옮겨 적으면 끝난다. 사람이 두 화면을 오갈 때 날짜가 어긋나는 것이 가장 흔한
 * 실수라, 목록의 묶음 단위를 나이스의 입력 단위와 같게 맞춘다.
 *
 * 사유를 누르면 그대로 복사된다. 나이스의 사유 칸에 붙여 넣으면 된다.
 * 여기서도 누른 줄은 사라지지 않는다.
 */
import {onMounted, ref} from 'vue'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {MONTHS, calendarYearOf} from '../services/academicYear'
import {usePendingStore} from '../stores/pending'
import {useSchoolStore} from '../stores/school'
import FocusEntryModal from '../components/FocusEntryModal.vue'
import {UiButton, UiLedger, UiNotice, UiPage, UiToggle} from '../components/ui'
import {useDownloadStore} from '../stores/download'

const app = useAppStore()
const axis = useAxisStore()
const pending = usePendingStore()
const school = useSchoolStore()
const download = useDownloadStore()

const focusDay = ref(null)
const copied = ref(null)

async function pickMonth(month) {
    pending.month = pending.month === month ? null : month
    pending.year = calendarYearOf(app.currentYear?.year ?? 0, month)
    await pending.fetchNeis().catch(() => {
    })
}

/** 메모를 클립보드로. 붙여 넣을 곳은 나이스의 사유 칸이다. */
async function copyMemo(span) {
    try {
        await navigator.clipboard?.writeText(span.memo ?? '')
    } catch {
        // 클립보드를 못 쓰는 환경이 있다. 복사 표시는 하지 않는다.
        return
    }
    copied.value = span.id
    setTimeout(() => {
        if (copied.value === span.id) copied.value = null
    }, 1200)
}

async function saveFocus(items) {
    const target = focusDay.value
    focusDay.value = null
    if (!target) return
    await pending.saveFocus(items).catch(() => {
    })
}

// 한쪽이 실패해도 나머지는 그린다. 실패는 각 스토어의 error에 담겨 화면 아래
// UiNotice가 그대로 보여준다 — 사유 후보와 태그가 빈 것이 "없다"로 읽히면 안 된다.
onMounted(async () => {
    if (!app.ready) return
    await Promise.all([
        pending.fetchNeis().catch(() => {
        }),
        axis.fetchMemos().catch(() => {
        }),
        school.fetchAll().catch(() => {
        }),
    ])
})
/** 파일 저장 실패를 화면에 남긴다. 눌러도 아무 일이 없는 단추를 두지 않는다. */
function saveCsv(kind, args, suggested) {
    download.csv(kind, args, suggested).catch(() => {
    })
}
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn" text="먼저 설정에서 학급과 명렬표를 넣어주세요."/>

    <UiPage v-else :subtitle="`아직 못 넣은 것 ${pending.neisLeft}건 · ${pending.neisDays.length}일치`"
            title="NEIS 미등재">
        <template #actions>
            <UiButton variant="download"
                      @click="saveCsv('pending', {classId: app.classId, kind: 'neis', today: app.today}, 'NEIS_미등재.csv')">
                미등재 CSV
            </UiButton>
        </template>

        <div class="filters">
            <span class="filters__label">월</span>
            <button v-for="month in MONTHS" :key="month"
                    :class="['pick', 'pick--slot', pending.month === month ? 'is-on' : '']"
                    type="button" @click="pickMonth(month)">
                {{ month }}
            </button>
        </div>

        <p v-if="pending.neisDays.length === 0" class="ledger__empty">나이스에 넣을 것이 없습니다.</p>

        <div class="daygroup">
            <UiLedger v-for="group in pending.neisDays" :key="group.date"
                      :hint="`${group.spans.filter((s) => !s.neisDone).length}건 남음`"
                      :title="group.dateLabel" class="list--neis">
                <template #actions>
                    <UiButton size="tight" @click="pending.markDay(group.date)">이 날짜 전부 등재</UiButton>
                    <UiButton size="tight" variant="primary" @click="focusDay = group">한 명씩 등재</UiButton>
                </template>

                <div v-for="span in group.spans" :key="span.id"
                     :class="['row', span.neisDone ? 'is-ok' : span.complete ? 'is-calm' : 'is-warn']">
                    <span class="row__no num">{{ span.number }}</span>
                    <span class="row__name"><b>{{ span.name }}</b></span>
                    <span class="row__what">
                        {{ span.reasonLabel || '미정' }} {{ span.typeLabel || '미정' }}
                    </span>
                    <span class="row__when num">{{ span.spanText }}</span>
                    <span class="row__memo">
                        <button :class="['copy', copied === span.id ? 'is-copied' : '']"
                                title="누르면 복사된다" type="button" @click="copyMemo(span)">
                            {{ copied === span.id ? '복사됨' : (span.memo || '사유 없음') }}
                        </button>
                    </span>
                    <span class="row__mark">
                        <UiToggle :model-value="span.neisDone" off-label="NEIS 미등재"
                                  on-label="NEIS 등재"
                                  @update:model-value="pending.setNeis(span.id, $event)"/>
                    </span>
                    <span class="row__acts"></span>
                </div>
            </UiLedger>
        </div>

        <UiNotice :text="pending.error" kind="error"/>
        <UiNotice :text="axis.error" kind="error"/>
        <UiNotice :text="school.error" kind="error"/>

        <FocusEntryModal :day="focusDay" :max-slot="app.maxSlot" :memos="axis.memos"
                         :open="Boolean(focusDay)" :reasons="axis.reasons" :tags="school.tags"
                         :types="axis.types"
                         @cancel="focusDay = null" @save="saveFocus"/>
        <UiNotice :text="download.error" kind="error"/>
        <UiNotice :text="download.done" kind="ok"/>
    </UiPage>
</template>
