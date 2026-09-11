import {createRouter, createWebHashHistory} from 'vue-router'
import {useAppStore} from '../stores/app'

/**
 * 흐름: Welcome → 개요(`/`)
 *
 * 사이드바는 **빈도로 나눈다.** 개요가 입구이고, 그 아래 넷이 매일 여는 화면이다 —
 * 입력하고(오늘의 출결), 확인하고(출결 기록), 서류를 챙기고(서류 미제출자),
 * 나이스에 넣는다(NEIS 미등재). 통계와 NEIS 검증은 월말에나 연다.
 * 설정과 업데이트는 업무가 아니므로 맨 아래 고정이다.
 *
 * 구분선이 없으면 `NEIS 미등재`와 `NEIS 검증`이 붙어 같은 기능으로 읽힌다 —
 * 앞은 내가 아직 안 넣은 것이고, 뒤는 넣은 것이 맞는지 대조하는 것이다.
 *
 * `기간 입력`을 화면으로 만들지 않는다. 기간은 화면이 아니라 입력의 한 축이라,
 * 오늘의 출결에서 교시를 고르는 자리에 "여러 날"이 함께 있으면 된다.
 *
 * **담임과 교과는 화면부터 구분된다.** 라우트의 `meta.mode`가 그 화면이 어느 모드의
 * 것인지 말하고, 없으면 두 모드가 함께 쓰는 화면이다(개요 · 이동 · 설정 · 업데이트).
 * 표시만 두면 분리가 겉모양뿐이라, 아래 `modeRedirect`가 주소로 들어오는 길도 막는다.
 */
const routes = [
    {path: '/', name: 'overview', component: () => import('../views/OverviewView.vue'), meta: {nav: '개요'}},
    {path: '/move', name: 'move', component: () => import('../views/MoveView.vue'), meta: {nav: '이동'}},
    {
        path: '/today', name: 'today', component: () => import('../views/TodayView.vue'),
        meta: {nav: '오늘의 출결', mode: 'homeroom'},
    },
    {
        path: '/log', name: 'log', component: () => import('../views/LogView.vue'),
        meta: {nav: '출결 기록', mode: 'homeroom'},
    },
    {
        path: '/docs', name: 'docs', component: () => import('../views/DocsView.vue'),
        meta: {nav: '서류 미제출자', mode: 'homeroom'},
    },
    {
        path: '/neis', name: 'neis', component: () => import('../views/NeisView.vue'),
        meta: {nav: 'NEIS 미등재', mode: 'homeroom'},
    },
    {
        path: '/stats', name: 'stats', component: () => import('../views/StatsView.vue'),
        meta: {nav: '통계', mode: 'homeroom'},
    },
    {
        path: '/verify', name: 'verify', component: () => import('../views/VerifyView.vue'),
        meta: {nav: 'NEIS 검증', mode: 'homeroom'},
    },
    // 교과는 차시 단위다. 오늘 수업에서 [교시 추가]로 칸을 만들고 그 칸에서 빠진
    // 학생만 기록한다. 수업 기록은 그 칸들이 날짜별로 쌓인 곳이다.
    {
        path: '/subject/today', name: 'subject-today',
        component: () => import('../views/SubjectTodayView.vue'),
        meta: {nav: '오늘 수업', mode: 'subject'},
    },
    {
        path: '/subject/log', name: 'subject-log',
        component: () => import('../views/SubjectLogView.vue'),
        meta: {nav: '수업 기록', mode: 'subject'},
    },
    {path: '/settings', name: 'settings', component: () => import('../views/SettingsView.vue'), meta: {nav: '설정'}},
    {path: '/update', name: 'update', component: () => import('../views/UpdateView.vue'), meta: {nav: '업데이트 확인'}},
    {path: '/welcome', name: 'welcome', component: () => import('../views/WelcomeView.vue'), meta: {bare: true}},
]

/**
 * 두 모드가 함께 쓰는 입구. 개요가 첫 줄이고, 이동이 그 아래다.
 *
 * `always`는 **담당 학급 · 강좌가 없어도 열리는 줄**이라는 뜻이다. 담임 학급이 없는 담임
 * 모드에서 나머지 항목은 비활성화되지만, 이 둘까지 잠그면 등록하러 갈 길이 막힌다.
 */
const NAV_TOP = [
    {to: '/', label: '개요', always: true},
    {to: '/move', label: '이동', always: true},
]

/**
 * 사이드바 구성. **모드마다 다른 목록이다.**
 * 화면 이름과 라우트 이름이 여기 한 곳에서만 연결된다.
 *
 * 담임 모드는 세 덩어리다. 교과 모드에는 담임의 것이 한 줄도 오지 않는다 —
 * 비담임 교사는 담임 모드를 한 번도 쓰지 않고, 반대로 담임만 하는 교사에게
 * 교과 화면은 평생 빈 채로 남는다.
 */
export const NAV_GROUPS = {
    homeroom: [
        NAV_TOP,
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
    ],
    subject: [
        NAV_TOP,
        [
            {to: '/subject/today', label: '오늘 수업'},
            {to: '/subject/log', label: '수업 기록'},
        ],
    ],
}

/**
 * 지금 모드의 사이드바.
 *
 * **모르는 모드에는 두 모드가 함께 쓰는 줄만 돌려준다.** 담임 목록을 돌려주면
 * 어긋난 모드 하나로 담임 화면이 통째로 새고, 그 화면들은 교과 강좌의 범위로
 * 담임 질의를 던진다. 빈 사이드바보다 나은 것은 담임 목록이 아니라 개요 · 이동이다.
 */
export function navGroups(mode) {
    return NAV_GROUPS[mode] ?? [NAV_TOP]
}

export const NAV_FOOT = [
    {to: '/settings', label: '설정'},
    {to: '/update', label: '업데이트 확인'},
]

/**
 * 아직 시작하지 않았으면 Welcome으로 되돌린다. 판단만 떼어 둔 것은 라우터를 띄우지
 * 않고도 확인할 수 있어야 하기 때문이다.
 *
 * 부팅을 **기다리지 않는다.** 부팅 전에는 그대로 통과시키고, 끝난 뒤의 이동에서만
 * 판단한다 — 가드에서 `boot()`를 부르면 App.vue가 부른 것과 겹쳐 `init_db`가 두 번 돌고,
 * 열 때마다 백업 파일이 둘씩 쌓인다.
 *
 * Welcome 자신은 되돌리지 않는다. 되돌리면 그 자리에서 이동이 끝나지 않는다.
 * **담당 학급 · 강좌가 하나라도 생기면 `needsWelcome`이 꺼진다** — 담임 학급이 없는 담임
 * 모드나 아직 강좌를 넣지 않은 학교는 시작하지 않은 상태가 아니다.
 */
export function welcomeRedirect(app, to) {
    if (!app.booted || to.name === 'welcome') return true
    return app.needsWelcome ? {name: 'welcome'} : true
}

/**
 * 지금 모드의 화면이 아니면 개요로 되돌린다.
 *
 * 사이드바에서 지운 것만으로는 분리가 겉모양뿐이다 — 주소창에 `#/today`를 치거나
 * 교과 모드로 바꾸기 전에 열어 둔 주소로 뒤로 가면 담임 화면이 그대로 열리고,
 * 그 화면은 교과 강좌의 범위로 담임 질의를 던진다.
 *
 * **그 모드에 담당 학급 · 강좌가 없을 때도 되돌린다.** 비담임 교사가 담임 모드로 들어오는 것은
 * 정상이지만, 그때 담임 화면은 범위가 없어 빈 목록만 그린다. 개요가 그 자리에서
 * 무엇을 등록해야 하는지 알린다.
 *
 * 부팅 전에는 판단하지 않는다. 그때는 담당 학급 · 강좌를 아직 읽지 못해 모드가 기본값(담임)이라,
 * 교과 화면을 열려는 교사까지 개요로 되돌리게 된다.
 */
export function modeRedirect(app, to) {
    if (!app.booted) return true
    const need = to.meta?.mode
    if (!need) return true
    return need === app.mode && app.hasClassIn(need) ? true : {name: 'overview'}
}

const router = createRouter({history: createWebHashHistory(), routes})

// 두 판단을 연결한다. 앞이 다른 곳으로 보내면 뒤는 묻지 않는다 —
// Welcome은 모드를 가리지 않는 화면이고, 담당 학급 · 강좌가 없으면 모드도 결정되지 않았다.
router.beforeEach((to) => {
    const app = useAppStore()
    const welcome = welcomeRedirect(app, to)
    return welcome === true ? modeRedirect(app, to) : welcome
})

export default router
