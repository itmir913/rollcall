import {createRouter, createWebHashHistory} from 'vue-router'
import {useAppStore} from '../stores/app'

/**
 * 흐름: Welcome → 개요(`/`)
 *
 * 사이드바는 **빈도로 나눈다.** 개요가 입구이고, 그 아래 넷이 매일 여는 화면이다 —
 * 찍고(오늘의 출결), 확인하고(출결 기록), 서류를 챙기고(서류 미제출자),
 * 나이스에 넣는다(NEIS 미등재). 통계와 NEIS 검증은 월말에나 연다.
 * 설정과 업데이트는 업무가 아니므로 맨 아래 고정이다.
 *
 * 구분선이 없으면 `NEIS 미등재`와 `NEIS 검증`이 붙어 같은 기능으로 읽힌다 —
 * 앞은 내가 아직 안 넣은 것이고, 뒤는 넣은 것이 맞는지 대조하는 것이다.
 *
 * `기간 입력`을 화면으로 만들지 않는다. 기간은 화면이 아니라 입력의 한 축이라,
 * 오늘의 출결에서 교시를 고르는 자리에 "여러 날"이 함께 있으면 된다.
 */
const routes = [
    {path: '/', name: 'overview', component: () => import('../views/OverviewView.vue'), meta: {nav: '개요'}},
    {path: '/today', name: 'today', component: () => import('../views/TodayView.vue'), meta: {nav: '오늘의 출결'}},
    {path: '/log', name: 'log', component: () => import('../views/LogView.vue'), meta: {nav: '출결 기록'}},
    {path: '/docs', name: 'docs', component: () => import('../views/DocsView.vue'), meta: {nav: '서류 미제출자'}},
    {path: '/neis', name: 'neis', component: () => import('../views/NeisView.vue'), meta: {nav: 'NEIS 미등재'}},
    {path: '/stats', name: 'stats', component: () => import('../views/StatsView.vue'), meta: {nav: '통계'}},
    {path: '/verify', name: 'verify', component: () => import('../views/VerifyView.vue'), meta: {nav: 'NEIS 검증'}},
    {path: '/settings', name: 'settings', component: () => import('../views/SettingsView.vue'), meta: {nav: '설정'}},
    {path: '/update', name: 'update', component: () => import('../views/UpdateView.vue'), meta: {nav: '업데이트 확인'}},
    {path: '/welcome', name: 'welcome', component: () => import('../views/WelcomeView.vue'), meta: {bare: true}},
]

/**
 * 사이드바 구성. 세 덩어리 + 아래 고정 둘.
 * 화면 이름과 라우트 이름이 여기 한 곳에서만 이어진다.
 */
export const NAV_GROUPS = [
    [{to: '/', label: '개요'}],
    [
        {to: '/today', label: '오늘의 출결'},
        {to: '/log', label: '출결 기록'},
        {to: '/docs', label: '서류 미제출자', badge: 'docPending'},
        {to: '/neis', label: 'NEIS 미등재', badge: 'neisPending'},
    ],
    [
        {to: '/stats', label: '통계'},
        {to: '/verify', label: 'NEIS 검증'},
    ],
]

export const NAV_FOOT = [
    {to: '/settings', label: '설정'},
    {to: '/update', label: '업데이트 확인'},
]

/**
 * 첫 실행이면 Welcome으로 되돌린다. 판단만 떼어 둔 것은 라우터를 띄우지 않고도
 * 확인할 수 있어야 하기 때문이다.
 *
 * 부팅을 **기다리지 않는다.** 부팅 전에는 그대로 통과시키고, 끝난 뒤의 이동에서만
 * 판단한다 — 가드에서 `boot()`를 부르면 App.vue가 부른 것과 겹쳐 `init_db`가 두 번 돌고,
 * 열 때마다 백업 파일이 둘씩 쌓인다.
 *
 * Welcome 자신은 되돌리지 않는다. 되돌리면 그 자리에서 이동이 끝나지 않는다.
 * 학급이 정해지면 `needsWelcome`이 꺼지므로 [시작하기]가 개요로 넘어간다.
 */
export function welcomeRedirect(app, to) {
    if (!app.booted || to.name === 'welcome') return true
    return app.needsWelcome ? {name: 'welcome'} : true
}

const router = createRouter({history: createWebHashHistory(), routes})

router.beforeEach((to) => welcomeRedirect(useAppStore(), to))

export default router
