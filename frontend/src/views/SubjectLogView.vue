<script setup>
/**
 * 수업 기록 — 지난 차시를 **하루가 카드 하나**로 본다.
 *
 * 한 카드에 한 달을 전부 넣으면 날짜 머리글이 목록 중간에 섞여 어디까지가 그날인지
 * 흐려진다. 월 필터는 3월부터 시작한다 — 학년도가 3월에 열리므로 1월 · 2월이 뒤에 온다.
 *
 * 여기에 담임의 숫자는 한 줄도 오지 않는다. 서류 · NEIS · 구분 · 종류는 담임이 쓰는 말이라,
 * 교과만 담당하는 교사에게는 매일 남의 일을 보는 화면이 된다.
 *
 * **차시 번호를 저장하지 않는다.** 날짜 · 교시 순으로 세면 나오는 값이라, 세는 범위가
 * 달라지면 값도 달라진다. 그래서 `N차시`가 아니라 `이 달 N번째`로 적는다.
 */
import {onMounted} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useSubjectStore} from '../stores/subject'
import {MONTHS} from '../services/academicYear'
import {UiButton, UiLedger, UiNotice, UiPage} from '../components/ui'

const app = useAppStore()
const subject = useSubjectStore()
const router = useRouter()

async function pickMonth(month) {
    subject.setMonth(month, app.currentYear?.year ?? Number(String(app.today).slice(0, 4)))
    await subject.fetchMonth().catch(() => {
    })
}

/** 그 차시를 열어 고친다. 고치는 자리는 오늘 수업 화면 한 곳이다. */
async function open(session) {
    subject.setDate(session.date)
    await subject.select(session.id).catch(() => {
    })
    await router.push('/subject/today')
}

/**
 * [오늘 수업 열기]. **오늘로 옮긴 뒤에 넘어간다.**
 *
 * 지난 차시를 한 번이라도 열어 봤으면 스토어의 날짜가 그 날에 머물러 있다.
 * 그대로 넘어가면 버튼 이름은 `오늘`인데 열리는 화면은 9월 3일이고, 교사가
 * 거기서 기록한 결석은 오늘이 아니라 그날의 기록이 된다.
 */
async function openToday() {
    subject.setDate(app.today)
    await router.push('/subject/today')
}

onMounted(async () => {
    if (!app.ready) return
    const month = subject.month ?? Number(String(app.today).slice(5, 7))
    subject.setMonth(month, app.currentYear?.year ?? Number(String(app.today).slice(0, 4)))
    await subject.fetchMonth().catch(() => {
    })
})
</script>

<template>
    <!-- 강좌를 하나도 추가하지 않았다. 무엇을 해야 하는지와 가는 길을 함께 둔다. -->
    <UiPage v-if="app.subjectClasses.length === 0" title="수업 기록">
        <UiNotice kind="warn"
                  text="아직 교과 강좌를 추가하지 않았습니다. 강좌를 추가하면 차시가 날짜별로 쌓입니다."/>
        <div class="lead">
            <UiButton size="wide" variant="primary" @click="router.push('/settings')">
                강좌 추가
            </UiButton>
        </div>
    </UiPage>

    <!-- 담당 강좌는 있는데 아직 선택하지 않았다. 추가하라고 적으면 이미 한 일을 다시 시킨다. -->
    <UiPage v-else-if="!app.ready" title="수업 기록">
        <UiNotice kind="warn" text="보고 있는 강좌가 없습니다. 담당 강좌 중 하나를 선택하세요."/>
        <div class="lead">
            <UiButton size="wide" variant="primary" @click="router.push('/move')">
                강좌 선택
            </UiButton>
        </div>
    </UiPage>

    <UiPage v-else :subtitle="app.currentClass?.name ?? ''" title="수업 기록">
        <template #actions>
            <UiButton variant="primary" @click="openToday">오늘 수업 열기</UiButton>
        </template>

        <div class="filters">
            <span class="filters__label">월</span>
            <button v-for="month in MONTHS" :key="month"
                    :class="['pick', 'pick--slot', subject.month === month ? 'is-on' : '']"
                    type="button" @click="pickMonth(month)">
                {{ month }}
            </button>
        </div>

        <div class="strip">
            <div class="strip__cell">
                <b class="num">{{ subject.sessionCount }}</b><span>차시</span>
            </div>
            <div class="strip__cell">
                <b class="num">{{ subject.dayCount }}</b><span>수업한 날</span>
            </div>
            <div class="strip__cell is-ok">
                <b class="num">{{ subject.absentTotal }}</b><span>결석</span>
            </div>
        </div>

        <p v-if="subject.days.length === 0" class="ledger__empty">
            그 달에 만든 차시가 없습니다.
        </p>

        <div class="daygroup">
            <UiLedger v-for="group in subject.days" :key="group.date"
                      :hint="`차시 ${group.sessions.length}개`" :title="group.dateLabel"
                      class="list--sesslog">
                <div v-for="session in group.sessions" :key="session.id" class="row is-calm">
                    <span class="row__no num">이 달 {{ session.nth }}번째</span>
                    <span class="row__what"><b class="num">{{ session.slot }}교시</b></span>
                    <span class="row__when num">
                        결석 {{ session.absentCount }} / {{ session.total }}명
                    </span>
                    <span class="row__memo">{{ session.memo || '—' }}</span>
                    <span class="row__acts">
                        <UiButton size="tight" @click="open(session)">열기</UiButton>
                    </span>
                </div>
            </UiLedger>
        </div>

        <UiNotice :text="subject.error" kind="error"/>
    </UiPage>
</template>

<style scoped>
/* 차례 · 교시 · 결석 · 메모 · 열기 */
.list--sesslog .row {
    grid-template-columns: 132px 92px 190px 1fr 76px;
}

/* 할 일이 하나뿐인 화면의 단추 자리. 왼쪽에 붙여 다음 걸음이 어디인지 바로 보이게 한다. */
.lead {
    display: flex;
    gap: var(--s-md);
}
</style>
