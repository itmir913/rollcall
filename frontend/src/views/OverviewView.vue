<script setup>
/**
 * 개요 — 밀린 일을 보는 곳.
 *
 * 오늘 찍으러 들어가는 큰 버튼 하나, 그리고 밀린 것 두 종류. 서류와 나이스는
 * 성격이 달라서 목록을 나눴다 — 서류는 학부모에게 받아야 하고, 나이스는 내가 넣어야 한다.
 * 다만 **열 문법은 같다.** 나이스에는 마감이 없으므로 그 칸에 "마감 없음"이 그대로
 * 들어간다. 칸을 없애면 두 목록의 눈높이가 어긋난다.
 *
 * 숫자는 전부 Rust가 세어 준다. 화면이 커맨드 대여섯 개를 조합해 숫자를 만들면
 * 그 조합 규칙이 프런트엔드의 비즈니스 로직이 된다.
 */
import {computed, onMounted} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useHomeStore} from '../stores/home'
import {UiButton, UiLedger, UiNotice, UiPage} from '../components/ui'
import {overdueLabel, overdueTone} from '../services/overdue'

const app = useAppStore()
const home = useHomeStore()
const router = useRouter()

const summary = computed(() => home.summary)

onMounted(() => {
    home.fetchSummary().catch(() => {
    })
})
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn"
              text="아직 학생 명단이 없습니다. 설정에서 학급과 명렬표를 넣으면 시작할 수 있습니다."/>

    <UiPage v-else :subtitle="summary?.dateLabel ?? ''" title="개요">
        <template #actions>
            <UiButton variant="primary" @click="router.push('/today')">오늘의 출결 입력하기</UiButton>
        </template>

        <div class="strip">
            <div class="strip__cell">
                <b class="num">{{ summary?.enrolled ?? '—' }}</b><span>재학</span>
            </div>
            <div class="strip__cell is-ok">
                <b class="num">{{ summary?.recorded ?? '—' }}</b><span>오늘 기록</span>
            </div>
            <RouterLink class="strip__cell is-warn" to="/today">
                <b class="num">{{ summary?.incomplete ?? '—' }}</b><span>구분 · 종류 미정</span>
            </RouterLink>
            <RouterLink class="strip__cell is-warn" to="/docs">
                <b class="num">{{ summary?.docPending ?? '—' }}</b><span>미제출</span>
            </RouterLink>
            <RouterLink class="strip__cell is-bad" to="/docs">
                <b class="num">{{ summary?.docOverdue ?? '—' }}</b><span>그중 마감 지남</span>
            </RouterLink>
        </div>

        <UiLedger :empty="!(summary?.docRows?.length)" empty-text="못 받은 서류가 없습니다."
                  hint="결석 건마다 증빙을 받았는지만 본다 · 마감이 지난 것부터"
                  title="서류 미제출">
            <div v-for="span in summary?.docRows ?? []" :key="span.id"
                 :class="['row', overdueTone(span)]">
                <span class="row__no num">{{ span.number }}</span>
                <span class="row__name"><b>{{ span.name }}</b></span>
                <span class="row__what">{{ span.reasonLabel || '미정' }} {{ span.typeLabel || '미정' }}</span>
                <span class="row__when num">{{ span.spanText }}</span>
                <span class="row__date num">{{ span.dateLabel }}</span>
                <span class="row__due num">{{ span.docDue ? `마감 ${span.docDue}` : '마감 없음' }}</span>
                <span class="row__flag">{{ overdueLabel(span) }}</span>
            </div>
            <template #foot>
                <span>서류 미제출 <b class="num">{{ summary?.docPending ?? 0 }}</b>건</span>
                <UiButton v-if="(summary?.docPending ?? 0) > (summary?.docRows?.length ?? 0)"
                          size="tight" @click="router.push('/docs')">
                    외 {{ summary.docPending - summary.docRows.length }}건 더 보기 ›
                </UiButton>
            </template>
        </UiLedger>

        <UiLedger :empty="!(summary?.neisRows?.length)" empty-text="나이스에 넣을 것이 없습니다."
                  hint="기록은 있는데 나이스에 아직 안 넣은 것 · 내가 할 일이다"
                  title="NEIS 미등재">
            <div v-for="span in summary?.neisRows ?? []" :key="span.id" class="row is-calm">
                <span class="row__no num">{{ span.number }}</span>
                <span class="row__name"><b>{{ span.name }}</b></span>
                <span class="row__what">{{ span.reasonLabel || '미정' }} {{ span.typeLabel || '미정' }}</span>
                <span class="row__when num">{{ span.spanText }}</span>
                <span class="row__date num">{{ span.dateLabel }}</span>
                <span class="row__due num">마감 없음</span>
                <span class="row__flag">{{ span.date === app.today ? '오늘' : '' }}</span>
            </div>
            <template #foot>
                <span>NEIS 미등재 <b class="num">{{ summary?.neisPending ?? 0 }}</b>건</span>
                <UiButton v-if="(summary?.neisPending ?? 0) > (summary?.neisRows?.length ?? 0)"
                          size="tight" @click="router.push('/neis')">
                    외 {{ summary.neisPending - summary.neisRows.length }}건 더 보기 ›
                </UiButton>
            </template>
        </UiLedger>

        <UiNotice :text="home.error" kind="error"/>
    </UiPage>
</template>

<style scoped>
.row {
    grid-template-columns: 48px 92px 132px 150px 130px 128px 108px;
}
</style>
