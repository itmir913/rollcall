<script setup>
import {computed, onMounted, ref} from 'vue'
import {RouterLink, RouterView, useRoute, useRouter} from 'vue-router'
import {useAppStore} from './stores/app'
import {useTheme} from './composables/useTheme'
import {UiButton, UiNotice} from './components/ui'

const app = useAppStore()
const route = useRoute()
const router = useRouter()
const theme = useTheme()
const booted = ref(false)

const THEME_LABELS = {light: '라이트', dark: '다크', system: '시스템'}

const NAV = [
    {to: '/', label: 'HOME'},
    {to: '/bulk', label: '기간 일괄'},
    {to: '/pending', label: '미제출'},
    {to: '/roster', label: '학생 명단'},
    {to: '/settings', label: '설정'},
]

/** Welcome과 초기 설정은 머리글 없이 전체 화면으로 띄운다. */
const bare = computed(() => route.meta.bare === true)

function cycleTheme() {
    const next = theme.modes[(theme.modes.indexOf(theme.mode.value) + 1) % theme.modes.length]
    theme.setMode(next)
}

onMounted(async () => {
    try {
        await app.init()
        // 학년도가 하나도 없으면 첫 실행이다. 설정만 비었으면 Welcome을 건너뛴다.
        if (!app.years.length) await router.replace('/welcome')
        else if (!app.ready) await router.replace('/setup')
    } catch {
        // app.error에 담겨 화면에 표시된다. 조용히 넘어가지 않는다.
    } finally {
        booted.value = true
    }
})
</script>

<template>
    <div class="shell">
        <header v-if="!bare" class="shell__bar">
            <span class="shell__brand">출결관리</span>
            <nav class="shell__nav">
                <RouterLink v-for="link in NAV" :key="link.to" :to="link.to"
                            active-class="is-active" class="shell__link">
                    {{ link.label }}
                </RouterLink>
            </nav>
            <div class="shell__right">
                <span v-if="app.ready" class="shell__context">
                    {{ app.currentYear?.year }}학년도 · {{ app.grade }}학년 {{ app.classNo }}반
                </span>
                <UiButton variant="ghost"
                          :title="`테마 바꾸기 (현재 화면: ${theme.resolved.value === 'dark' ? '다크' : '라이트'})`"
                          @click="cycleTheme">
                    {{ THEME_LABELS[theme.mode.value] }}
                </UiButton>
            </div>
        </header>

        <UiNotice v-if="app.error" :text="app.error" class="shell__error" kind="error"/>

        <main :class="['shell__main', bare ? 'is-bare' : '']">
            <RouterView v-if="booted"/>
            <p v-else class="shell__booting">데이터 파일을 여는 중…</p>
        </main>
    </div>
</template>

<style scoped>
.shell {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
}

.shell__bar {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 10px 20px;
    background: var(--c-surface);
    border-bottom: 1px solid var(--c-line);
}

.shell__brand {
    font-weight: 700;
    letter-spacing: -0.01em;
}

.shell__nav {
    display: flex;
    gap: 2px;
}

.shell__link {
    padding: 8px 14px;
    border-radius: 8px;
    color: var(--c-ink-3);
    text-decoration: none;
}

.shell__link:hover {
    color: var(--c-ink);
    background: var(--c-raised);
}

.shell__link.is-active {
    color: var(--c-ink);
    background: var(--c-raised);
    font-weight: 600;
}

.shell__right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
}

.shell__context {
    color: var(--c-ink-3);
}

.shell__error {
    margin: 16px 20px 0;
}

.shell__main {
    flex: 1;
    padding: 20px;
}

.shell__main.is-bare {
    padding: 0;
}

.shell__booting {
    color: var(--c-ink-3);
}
</style>
