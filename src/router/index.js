import {createRouter, createWebHashHistory} from 'vue-router'

/**
 * 흐름: Welcome → 명렬표 가져오기 → HOME
 *
 * HOME은 대시보드다. 오늘 할 일(출결 입력, 미제출 서류)로 들어가는 입구이지
 * 입력 화면 자체가 아니다. 입력은 `/attendance`가 맡는다.
 */
const routes = [
    {path: '/', name: 'home', component: () => import('../views/HomeView.vue')},
    {path: '/welcome', name: 'welcome', component: () => import('../views/WelcomeView.vue'), meta: {bare: true}},
    {path: '/attendance', name: 'attendance', component: () => import('../views/AttendanceView.vue')},
    {path: '/roster', name: 'roster', component: () => import('../views/RosterView.vue')},
    {path: '/bulk', name: 'bulk', component: () => import('../views/BulkView.vue')},
    {path: '/pending', name: 'pending', component: () => import('../views/PendingView.vue')},
    {path: '/settings', name: 'settings', component: () => import('../views/SettingsView.vue')},
]

export default createRouter({history: createWebHashHistory(), routes})
