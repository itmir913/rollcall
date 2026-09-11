<script setup>
/**
 * 앱 껍데기 — 왼쪽 사이드바와 본문.
 *
 * 사이드바는 **빈도로 나눈다.** 구분선 없이 붙여 두면 `NEIS 미등재`와 `NEIS 검증`이
 * 같은 기능으로 읽힌다 — 앞은 내가 아직 안 넣은 것이고, 뒤는 넣은 것이 맞는지
 * 대조하는 것이다. 설정과 업데이트는 업무가 아니므로 맨 아래 고정이다.
 */
import {onMounted} from 'vue'
import {useRoute, useRouter} from 'vue-router'
import {NAV_FOOT, NAV_GROUPS} from './router'
import {useAppStore} from './stores/app'
import {useAxisStore} from './stores/axis'
import {useHomeStore} from './stores/home'
import {UiNotice} from './components/ui'

const route = useRoute()
const router = useRouter()
const app = useAppStore()
const axis = useAxisStore()
const home = useHomeStore()

/** 배지는 안 받은 것을 전부 센다. 0이면 그리지 않는다. */
function badgeOf(key) {
    if (!key) return 0
    return key === 'docPending' ? home.docPending : home.neisPending
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

            <template v-for="(group, i) in NAV_GROUPS" :key="i">
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
                <p v-if="app.ready" class="rail__where">
                    {{ app.currentYear?.year }}학년도<br/>
                    {{ app.currentClass?.name }}
                </p>
            </div>
        </nav>

        <div>
            <UiNotice :text="app.error" kind="error"/>
            <RouterView v-if="app.booted"/>
            <p v-else class="main">데이터 파일을 여는 중…</p>
        </div>
    </div>
</template>
