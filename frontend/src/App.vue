<script setup>
/**
 * 앱 껍데기 — 왼쪽 사이드바와 본문.
 *
 * 사이드바는 **빈도로 나눈다.** 구분선 없이 붙여 두면 `NEIS 미등재`와 `NEIS 검증`이
 * 같은 기능으로 읽힌다 — 앞은 내가 아직 안 넣은 것이고, 뒤는 넣은 것이 맞는지
 * 대조하는 것이다. 설정과 업데이트는 업무가 아니므로 맨 아래 고정이다.
 *
 * 맨 위는 **지금 어디에 있는가**다. 담임 · 교과 스위치와 지금 보고 있는 학급이
 * 차례로 오고, 그 아래가 화면 목록이다. 목록은 모드마다 통째로 갈린다.
 */
import {computed, onMounted} from 'vue'
import {useRoute, useRouter} from 'vue-router'
import {NAV_FOOT, modeRedirect, navGroups} from './router'
import {MODES, useAppStore} from './stores/app'
import {useAxisStore} from './stores/axis'
import {useHomeStore} from './stores/home'
import {UiNotice} from './components/ui'

const route = useRoute()
const router = useRouter()
const app = useAppStore()
const axis = useAxisStore()
const home = useHomeStore()

/** 스위치에 적는 이름. 화면은 'homeroom' 같은 내부 값을 보여주지 않는다. */
const MODE_LABEL = {homeroom: '담임', subject: '교과'}

const groups = computed(() => navGroups(app.mode))

/** 배지는 안 받은 것을 전부 센다. 0이면 그리지 않는다. */
function badgeOf(key) {
    if (!key) return 0
    return key === 'docPending' ? home.docPending : home.neisPending
}

/**
 * 담임과 교과를 오간다. 학급은 스토어가 그 모드에서 마지막에 본 것으로 되돌린다.
 *
 * 지금 화면이 넘어간 모드의 것이 아니면 개요로 보낸다. 그대로 두면 교과 모드에서
 * 담임 화면이 남아 교과 강좌의 범위로 담임 질의를 던진다.
 */
async function pickMode(mode) {
    if (mode === app.mode) return
    try {
        await app.setMode(mode)
    } catch {
        return // 오류는 app.error에 담겨 본문 위 UiNotice가 그대로 보여준다.
    }
    if (route.meta.mode && route.meta.mode !== app.mode) await router.push('/')
}

onMounted(async () => {
    try {
        await app.boot()
        // **여기서 보내야 첫 실행에 닿는다.** 라우터 가드는 부팅보다 먼저 도는
        // 최초 이동을 잡지 못한다 — 그때는 아직 무엇도 알지 못하기 때문이다.
        // 부팅 전에는 아래 RouterView가 아무것도 그리지 않아 개요가 번쩍이지 않는다.
        if (app.needsWelcome) {
            await router.replace({name: 'welcome'})
            return
        }
        // **부팅 뒤에 한 번 더 판단한다.** 가드는 부팅보다 먼저 도는 최초 이동을
        // 통과시킬 수밖에 없어서, 주소창에 담임 화면을 직접 친 교과 교사가
        // 그대로 그 화면에 남는다. 그때 그 화면의 커맨드는 전부 거절된다.
        const back = modeRedirect(app, route)
        if (back !== true) await router.replace(back)
        if (!app.ready) return
        await Promise.all([axis.fetchAll(), home.fetchSummary()])
    } catch {
        // 오류는 각 스토어의 error에 담겨 화면에 그대로 나온다. 여기서 삼키지 않는다.
    }
})
</script>

<template>
    <div v-if="route.meta.bare" class="bare">
        <RouterView v-if="app.booted"/>
    </div>

    <div v-else class="shell">
        <nav class="rail">
            <div class="rail__brand">출결<em>관리</em></div>

            <!-- 한쪽만 맡은 교사에게는 그리지 않는다. 누를 것이 없는 단추가 매일 보이면
                 이 앱의 절반이 내 것이 아니라고 말하는 것과 같다. -->
            <div v-if="app.needsModeSwitch" class="rail__switch">
                <button v-for="mode in MODES" :key="mode"
                        :class="['pick', 'rail__mode', app.mode === mode ? 'is-on' : '']"
                        type="button" @click="pickMode(mode)">
                    {{ MODE_LABEL[mode] }}
                </button>
            </div>

            <RouterLink class="rail__here" to="/move">
                <span class="rail__here-year num">{{ app.currentYear?.year ?? '—' }}학년도</span>
                <b class="rail__here-name">{{ app.currentClass?.name ?? '맡은 것 없음' }}</b>
            </RouterLink>

            <template v-for="(group, i) in groups" :key="i">
                <div v-if="i > 0" class="rail__rule"></div>
                <div class="rail__group">
                    <RouterLink v-for="link in group" :key="link.to" :to="link.to"
                                active-class="is-active" class="rail__link">
                        {{ link.label }}
                        <span v-if="badgeOf(link.badge)" class="rail__badge num">
                            {{ badgeOf(link.badge) }}
                        </span>
                    </RouterLink>
                </div>
            </template>

            <div class="rail__spacer"></div>

            <div class="rail__foot">
                <RouterLink v-for="link in NAV_FOOT" :key="link.to" :to="link.to"
                            active-class="is-active" class="rail__link">
                    {{ link.label }}
                </RouterLink>
            </div>
        </nav>

        <div>
            <UiNotice :text="app.error" kind="error"/>
            <!-- 학급이 바뀌면 화면을 다시 만든다. 같은 주소에 머문 채 옮기면 명단도
                 기록도 옛 학급의 것이 남고, 그 숫자는 틀렸다는 표시 없이 그대로 보인다. -->
            <RouterView v-if="app.booted" :key="app.classId"/>
            <p v-else class="main">데이터 파일을 여는 중…</p>
        </div>
    </div>
</template>

<style scoped>
/* 두 단추를 나란히 둔다. 사이드바 폭이 정해져 있어 반씩 나눠 갖는다. */
.rail__switch {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--s-2xs);
    padding: 0 var(--s-md) var(--s-md);
}

.rail__mode {
    padding: var(--s-xs) 0;
    text-align: center;
}

/* 지금 보고 있는 곳. 눌러서 이동 화면으로 간다. */
.rail__here {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0 var(--s-md) var(--s-lg);
    padding: var(--s-sm) var(--s-md);
    border: 1px solid var(--c-line);
    border-radius: var(--r-md);
    color: var(--c-ink-3);
    text-decoration: none;
}

.rail__here:hover {
    border-color: var(--c-accent);
}

.rail__here-name {
    color: var(--c-ink);
    font-size: var(--t-md);
}
</style>
