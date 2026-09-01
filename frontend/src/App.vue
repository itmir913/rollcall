<script setup>
/**
 * 앱 껍데기 — 접히는 사이드바.
 *
 * 사이드바는 세로 공간을 먹지 않는다. 상단 바는 56px쯤을 가로로 길게 가져가는데,
 * 이 앱에서 가장 넓은 화면(체크 열이 붙는 격자, 오른쪽 학생 패널)은 **가로가**
 * 아쉽다. 그래서 창이 좁아지면 아이콘만 남는 폭으로 자동으로 접힌다.
 * 접힘 상태는 이 PC에만 저장한다 — 화면 취향이지 출결 데이터가 아니다.
 */
import {computed, onBeforeUnmount, onMounted, ref} from 'vue'
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

/** 이 폭 아래에서는 사이드바가 자동으로 접힌다. */
const NARROW = 1180
const KEY_COLLAPSED = 'sidebar-collapsed'

const NAV = [
    {to: '/', label: 'HOME', mark: '⌂'},
    {to: '/attendance', label: '출결 입력', mark: '✎'},
    {to: '/pending', label: '미제출', mark: '!'},
    {to: '/bulk', label: '기간 일괄', mark: '≡'},
    {to: '/roster', label: '학생 명단', mark: '☰'},
    {to: '/settings', label: '설정', mark: '⚙'},
]

function readCollapsed() {
    try {
        const saved = localStorage.getItem(KEY_COLLAPSED)
        if (saved !== null) return saved === '1'
    } catch { /* 사이트 데이터 차단 등 */ }
    return window.innerWidth < NARROW
}

const collapsed = ref(readCollapsed())
/** 교사가 직접 접거나 편 적이 있는가. 있으면 창 크기가 그 뜻을 덮지 않는다. */
const pinned = ref(false)

function toggleSidebar() {
    collapsed.value = !collapsed.value
    pinned.value = true
    try {
        localStorage.setItem(KEY_COLLAPSED, collapsed.value ? '1' : '0')
    } catch { /* 저장 못 해도 이번 실행에는 적용된다 */ }
}

function onResize() {
    if (pinned.value) return
    collapsed.value = window.innerWidth < NARROW
}

const bare = computed(() => route.meta.bare === true)

function cycleTheme() {
    const next = theme.modes[(theme.modes.indexOf(theme.mode.value) + 1) % theme.modes.length]
    theme.setMode(next)
}

onMounted(async () => {
    window.addEventListener('resize', onResize)
    try {
        await app.init()
        // 명단이 하나도 없으면 첫 실행이다. 학년도는 앱이 알아서 만든다.
        if (!app.ready) await router.replace('/welcome')
    } catch {
        // app.error에 담겨 화면에 표시된다. 조용히 넘어가지 않는다.
    } finally {
        booted.value = true
    }
})

onBeforeUnmount(() => window.removeEventListener('resize', onResize))
</script>

<template>
    <div v-if="bare" class="bare">
        <RouterView v-if="booted"/>
    </div>

    <div v-else :class="['shell', collapsed ? 'is-collapsed' : '']">
        <nav class="rail">
            <div class="rail__top">
                <button class="rail__toggle" :title="collapsed ? '펼치기' : '접기'"
                        type="button" @click="toggleSidebar">
                    {{ collapsed ? '»' : '«' }}
                </button>
                <span v-if="!collapsed" class="rail__brand">출결관리</span>
            </div>

            <RouterLink v-for="link in NAV" :key="link.to" :to="link.to"
                        :title="link.label" active-class="is-active" class="rail__link">
                <span class="rail__mark">{{ link.mark }}</span>
                <span v-if="!collapsed" class="rail__text">{{ link.label }}</span>
            </RouterLink>

            <div class="rail__foot">
                <span v-if="!collapsed && app.ready" class="rail__context">
                    {{ app.currentYear?.year }}학년도<br/>
                    {{ app.grade }}학년 {{ app.classNo }}반
                </span>
                <UiButton :title="`테마 바꾸기 (현재 ${theme.resolved.value === 'dark' ? '다크' : '라이트'})`"
                          variant="ghost" @click="cycleTheme">
                    {{ collapsed ? '◐' : THEME_LABELS[theme.mode.value] }}
                </UiButton>
            </div>
        </nav>

        <div class="body">
            <UiNotice v-if="app.error" :text="app.error" class="body__error" kind="error"/>
            <main class="body__main">
                <RouterView v-if="booted"/>
                <p v-else class="body__booting">데이터 파일을 여는 중…</p>
            </main>
        </div>
    </div>
</template>

<style scoped>
.bare {
    min-height: 100vh;
}

.shell {
    display: flex;
    min-height: 100vh;
}

.rail {
    width: 208px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 10px;
    background: var(--c-surface);
    border-right: 1px solid var(--c-line);
    transition: width 140ms ease;
}

.shell.is-collapsed .rail {
    width: 60px;
}

.rail__top {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
    min-height: 40px;
}

.rail__toggle {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    border: 1px solid var(--c-line);
    background: var(--c-raised);
    color: var(--c-ink-3);
    cursor: pointer;
    flex-shrink: 0;
}

.rail__toggle:hover {
    border-color: var(--c-accent);
    color: var(--c-ink);
}

.rail__brand {
    font-weight: 700;
    white-space: nowrap;
}

.rail__link {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 42px;
    padding: 0 10px;
    border-radius: 8px;
    color: var(--c-ink-3);
    text-decoration: none;
    white-space: nowrap;
}

.rail__link:hover {
    background: var(--c-raised);
    color: var(--c-ink);
}

.rail__link.is-active {
    background: var(--c-raised);
    color: var(--c-ink);
    font-weight: 600;
}

.rail__mark {
    width: 20px;
    text-align: center;
    flex-shrink: 0;
}

.rail__foot {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
}

.rail__context {
    color: var(--c-ink-3);
    line-height: 1.5;
    padding: 0 10px;
}

.body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
}

.body__error {
    margin: 16px 20px 0;
}

.body__main {
    flex: 1;
    padding: 20px;
    min-width: 0;
}

.body__booting {
    color: var(--c-ink-3);
}
</style>
