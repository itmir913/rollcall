<script setup>
/**
 * 통계 — 한도를 지켜보는 곳.
 *
 * 체험학습 연 20일, 생리통 조퇴 월 1회처럼 **세어야 하는 규정**이 있고, 지금까지
 * 손으로 적은 표로 관리해 온 것들이다. 앱이 이미 날짜를 전부 들고 있으므로 세는 일은
 * 앱이 한다.
 *
 * 칸 하나가 하루다. 숫자로 `15/20`만 적으면 남은 수를 매번 빼야 하지만, 칸은
 * **남은 자리가 눈에 바로 보인다**. 손으로 적던 표를 그대로 옮긴 것이다.
 *
 * **한도는 입력을 막지 않는다.** 넘겼다는 이유로 기록을 거부하면 정작 넘긴 날을
 * 남길 수 없다.
 */
import {computed, onMounted} from 'vue'
import {useAppStore} from '../stores/app'
import {useSchoolStore} from '../stores/school'
import {useStatsStore} from '../stores/stats'
import {UiButton, UiLedger, UiNotice, UiPage} from '../components/ui'
import {useDownloadStore} from '../stores/download'
import {countOf, monthClass, quotaCells, remainLabel} from '../services/quota'

const app = useAppStore()
const school = useSchoolStore()
const stats = useStatsStore()
const download = useDownloadStore()

const MONTH_KEYS = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 1, 2]

const totals = computed(() => {
    const rows = stats.reports.flatMap((r) => r.rows)
    return {
        users: rows.filter((r) => r.used > 0).length,
        used: rows.reduce((sum, r) => sum + r.used, 0),
        near: rows.filter((r) => r.state === 'near').length,
        over: rows.filter((r) => r.state === 'over').length,
    }
})

async function pickRule(ruleId) {
    stats.ruleId = stats.ruleId === ruleId ? null : ruleId
    await stats.fetchReports().catch(() => {
    })
}

// 한쪽이 실패해도 나머지는 그린다. 실패는 각 스토어의 error에 담겨 화면 아래
// UiNotice가 그대로 보여준다 — 규정 목록이 빈 것이 "규정이 없다"로 읽히면 안 된다.
onMounted(async () => {
    if (!app.ready) return
    await Promise.all([
        stats.fetchReports().catch(() => {
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
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn" text="먼저 설정에서 학급과 명렬표를 넣어주세요."/>

    <UiPage v-else :subtitle="`${app.currentYear?.year ?? ''}학년도 ${app.grade}학년 ${app.classNo}반`"
            title="통계">
        <template #actions>
            <UiButton v-if="stats.ruleId" variant="download"
                      @click="saveCsv('quota', {...app.scope, ruleId: stats.ruleId}, '한도.csv')">
                이 표 CSV
            </UiButton>
        </template>

        <div class="filters">
            <span class="filters__label">규정</span>
            <button v-for="rule in school.rules" :key="rule.id"
                    :class="['pick', stats.ruleId === rule.id ? 'is-on' : '']"
                    type="button" @click="pickRule(rule.id)">
                {{ rule.name }}
            </button>
            <button :class="['pick', stats.ruleId === null ? 'is-on' : '']" type="button"
                    @click="pickRule(null)">전체
            </button>
            <span class="filters__gap"></span>
            <UiButton size="tight" @click="$router.push('/settings')">규정 추가</UiButton>
        </div>

        <div class="strip">
            <div class="strip__cell"><b class="num">{{ totals.users }}</b><span>쓴 학생</span></div>
            <div class="strip__cell"><b class="num">{{ totals.used }}</b><span>합계</span></div>
            <div class="strip__cell is-warn"><b class="num">{{ totals.near }}</b><span>한도 임박</span></div>
            <div class="strip__cell is-bad"><b class="num">{{ totals.over }}</b><span>한도 소진</span></div>
        </div>

        <template v-for="report in stats.reports" :key="report.rule.id">
            <UiLedger v-if="report.rule.period !== 'month'" :empty="report.rows.length === 0"
                      :hint="`${report.rule.tagName ? `태그 「${report.rule.tagName}」` : '전체'} · 한도 ${report.rule.limitN}${report.rule.unit === 'day' ? '일' : '회'}`"
                      :note="'칸 하나가 하루. 올리면 날짜가 나온다'"
                      :title="report.rule.name" class="list--quota"
                      empty-text="아직 쓴 학생이 없습니다.">
                <div v-for="row in report.rows" :key="row.studentId"
                     :class="['row', row.state === 'over' ? 'is-bad' : row.state === 'near' ? 'is-warn' : 'is-calm']">
                    <span class="row__no num">{{ row.number }}</span>
                    <span class="row__name"><b>{{ row.name }}</b></span>
                    <span class="dots">
                        <span v-for="(date, i) in quotaCells(row)" :key="i"
                              :class="['dot', date ? 'is-used' : '']" :title="date ?? ''"></span>
                    </span>
                    <span class="row__count num">
                        {{ row.used }} / {{ row.limitN }}{{ report.rule.unit === 'day' ? '일' : '회' }}
                    </span>
                    <span class="row__date num">{{ row.dates[row.dates.length - 1] ?? '—' }}</span>
                    <span class="row__flag">{{ remainLabel(row) }}</span>
                </div>
            </UiLedger>

            <UiLedger v-else :empty="report.rows.length === 0"
                      :hint="`태그 「${report.rule.tagName ?? '전체'}」 · 달마다 ${report.rule.limitN}회`"
                      :note="'한 달에 한도를 넘기면 그 달이 붉어진다'"
                      :title="report.rule.name" class="list--month"
                      empty-text="아직 쓴 학생이 없습니다.">
                <div v-for="row in report.rows" :key="row.studentId"
                     :class="['row', row.state === 'over' ? 'is-bad' : 'is-calm']">
                    <span class="row__no num">{{ row.number }}</span>
                    <span class="row__name"><b>{{ row.name }}</b></span>
                    <span class="months">
                        <span v-for="month in MONTH_KEYS" :key="month"
                              :class="['mcell', monthClass(row, month, report.rule.limitN)]">
                            <i class="num">{{ month }}</i>
                            <b v-if="countOf(row, month)" class="num">{{ countOf(row, month) }}</b>
                        </span>
                    </span>
                    <span class="row__flag">
                        {{ row.state === 'over' ? '한도 넘김' : '한도 안' }}
                    </span>
                </div>
            </UiLedger>
        </template>

        <UiLedger v-if="stats.untagged.length" class="list--gap"
                  hint="세는 대상인지 아닌지 판단할 수 없는 건이다. 태그를 붙이면 위 표에 들어간다"
                  title="태그가 없는 출결">
            <div v-for="span in stats.untagged" :key="span.id" class="row is-warn">
                <span class="row__no num">{{ span.number }}</span>
                <span class="row__name"><b>{{ span.name }}</b></span>
                <span class="row__what">{{ span.reasonLabel || '미정' }} {{ span.typeLabel || '미정' }}</span>
                <span class="row__when num">{{ span.spanText }}</span>
                <span class="row__date num">{{ span.dateLabel }}</span>
                <span class="row__acts">
                    <UiButton v-for="tag in school.tags" :key="tag.id" size="tight"
                              @click="stats.setTag(span.id, tag.id)">
                        {{ tag.name }}
                    </UiButton>
                </span>
            </div>
        </UiLedger>

        <UiNotice :text="stats.error" kind="error"/>
        <UiNotice :text="school.error" kind="error"/>
        <UiNotice :text="download.error" kind="error"/>
        <UiNotice :text="download.done" kind="ok"/>
    </UiPage>
</template>
