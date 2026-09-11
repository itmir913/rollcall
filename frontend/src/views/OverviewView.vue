<script setup>
/**
 * 개요 — 밀린 일을 보는 곳.
 *
 * **모드마다 다른 화면이다.** 담임에게는 오늘 출결을 입력하러 들어가는 큰 버튼 하나와 밀린 것
 * 두 종류가, 교과에게는 오늘의 차시와 이번 달 수업이 온다. 한쪽의 숫자가 다른 쪽에
 * 한 줄도 새지 않는다 — 비담임 교사는 담임 모드를 한 번도 쓰지 않고, 담임만 하는
 * 교사에게 교과 화면은 평생 빈 채로 남는다.
 *
 * 담임의 서류와 나이스는 성격이 달라서 목록을 나눴다 — 서류는 학부모에게 받아야 하고,
 * 나이스는 내가 넣어야 한다. 다만 **열 문법은 같다.** 나이스에는 마감이 없으므로 그 칸에
 * "마감 없음"이 그대로 들어간다. 칸을 없애면 두 목록의 눈높이가 어긋난다.
 *
 * 숫자는 전부 Rust가 세어 준다. 화면이 커맨드 대여섯 개를 조합해 숫자를 만들면
 * 그 조합 규칙이 프런트엔드의 비즈니스 로직이 된다.
 */
import {computed, onMounted} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useHomeStore} from '../stores/home'
import {useSubjectStore} from '../stores/subject'
import {UiButton, UiLedger, UiNotice, UiPage} from '../components/ui'
import {overdueLabel, overdueTone} from '../services/overdue'

const app = useAppStore()
const home = useHomeStore()
const subject = useSubjectStore()
const router = useRouter()

const summary = computed(() => home.summary)

onMounted(async () => {
    // 담임 요약은 담임 모드에서만 온다. 스토어가 스스로 막으므로 여기서 다시 묻지 않는다.
    home.fetchSummary().catch(() => {
    })
    if (!app.ready || app.isHomeroom) return
    // 교과의 숫자는 이번 달 차시에서 나온다. 오늘 몫은 그 안에서 날짜로 고른다.
    subject.setDate(app.today)
    subject.setMonth(
        Number(String(app.today).slice(5, 7)),
        app.currentYear?.year ?? Number(String(app.today).slice(0, 4)),
    )
    await subject.fetchMonth().catch(() => {
    })
})
</script>

<template>
    <!-- ── 교과 모드 ───────────────────────────────────────────
         담임 어휘가 한 줄도 오지 않는다. 서류 · NEIS · 구분 · 종류는 담임이 쓰는 말이라,
         교과만 담당하는 교사에게는 매일 남의 일을 보는 화면이 된다. -->
    <UiPage v-if="!app.isHomeroom" :subtitle="app.currentClass?.name ?? ''" title="개요">
        <template v-if="app.ready" #actions>
            <UiButton variant="primary" @click="router.push('/subject/today')">
                오늘 수업 입력하기
            </UiButton>
        </template>

        <!-- 강좌를 하나도 등록하지 않았다. 무엇을 해야 하는지와 가는 길을 함께 둔다. -->
        <template v-if="app.subjectClasses.length === 0">
            <UiNotice kind="warn"
                      text="아직 수업을 등록하지 않았습니다. 강좌를 만들면 교시를 추가하고 결석을 기록할 수 있습니다."/>
            <div class="lead">
                <UiButton size="wide" variant="primary" @click="router.push('/settings')">
                    수업 등록하기
                </UiButton>
            </div>
        </template>

        <!-- 담당 강좌는 있는데 아직 고르지 않았다. -->
        <template v-else-if="!app.ready">
            <UiNotice kind="warn" text="보고 있는 수업이 없습니다. 담당 강좌 중 하나를 고르세요."/>
            <div class="lead">
                <UiButton size="wide" variant="primary" @click="router.push('/move')">
                    수업 고르기
                </UiButton>
            </div>
        </template>

        <template v-else>
            <div class="strip">
                <div class="strip__cell">
                    <b class="num">{{ subject.daySessions.length }}</b><span>오늘 차시</span>
                </div>
                <div class="strip__cell is-ok">
                    <b class="num">{{ subject.dayAbsentTotal }}</b><span>오늘 빠진 사람</span>
                </div>
                <div class="strip__cell">
                    <b class="num">{{ subject.sessionCount }}</b><span>{{ subject.month }}월 차시</span>
                </div>
                <div class="strip__cell">
                    <b class="num">{{ subject.dayCount }}</b><span>{{ subject.month }}월 수업한 날</span>
                </div>
            </div>

            <UiLedger :empty="subject.daySessions.length === 0"
                      class="list--todaysess"
                      empty-text="오늘 만든 교시가 없습니다. [오늘 수업]에서 교시를 추가하세요."
                      hint="칸을 만든 것이 곧 그 교시를 불렀다는 뜻이다" title="오늘의 차시">
                <div v-for="session in subject.daySessions" :key="session.id" class="row is-calm">
                    <span class="row__what"><b class="num">{{ session.slot }}교시</b></span>
                    <span class="row__when num">
                        빠진 사람 {{ session.absentCount }} / {{ session.total }}명
                    </span>
                    <span class="row__memo">{{ session.memo || '—' }}</span>
                </div>
                <template #foot>
                    <span>{{ subject.month }}월 차시 <b class="num">{{ subject.sessionCount }}</b>개</span>
                    <UiButton size="tight" @click="router.push('/subject/log')">
                        수업 기록 보기 ›
                    </UiButton>
                </template>
            </UiLedger>

            <UiNotice :text="subject.error" kind="error"/>
        </template>
    </UiPage>

    <!-- ── 담임 모드 ───────────────────────────────────────────
         **교과와 같은 두 갈래다.** 담임 학급을 하나도 등록하지 않은 것과, 등록은 했는데
         아직 고르지 않은 것은 다음 걸음이 다르다 — 앞은 설정에서 만들어야 하고 뒤는
         이동에서 고르기만 하면 된다. 한 문장으로 합치면 이미 만든 교사를 설정으로
         보내 놓고 거기서 또 무엇을 해야 하는지 알려주지 않게 된다. -->
    <UiPage v-else-if="app.homeroomClasses.length === 0" title="개요">
        <UiNotice kind="warn"
                  text="아직 담임 학급을 등록하지 않았습니다. 학급과 명렬표를 넣으면 출결을 입력할 수 있습니다."/>
        <div class="lead">
            <UiButton size="wide" variant="primary" @click="router.push('/settings')">
                담임 학급 등록하기
            </UiButton>
        </div>
    </UiPage>

    <!-- 담임 학급은 있는데 아직 고르지 않았다. -->
    <UiPage v-else-if="!app.ready" title="개요">
        <UiNotice kind="warn" text="보고 있는 학급이 없습니다. 담임 학급 중 하나를 고르세요."/>
        <div class="lead">
            <UiButton size="wide" variant="primary" @click="router.push('/move')">
                학급 고르기
            </UiButton>
        </div>
    </UiPage>

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

/* 교시 · 빠진 사람 · 메모. 담임 목록과 칸 수가 다르므로 따로 적는다. */
.list--todaysess .row {
    grid-template-columns: 120px 190px 1fr;
}

/* 할 일이 하나뿐인 화면의 단추 자리. 왼쪽에 붙여 다음 걸음이 어디인지 바로 보이게 한다. */
.lead {
    display: flex;
    gap: var(--s-md);
}
</style>
