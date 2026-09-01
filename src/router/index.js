import {createRouter, createWebHashHistory} from 'vue-router'

/**
 * 흐름: Welcome → 초기 설정 → HOME
 *
 * HOME이 일일 입력 격자다. 매일 열자마자 바로 입력할 수 있어야 하므로 `/`가 곧
 * 그 화면이고, Welcome과 설정은 첫 실행에서만 지나간다.
 */
const routes = [
    {path: '/', name: 'home', component: () => import('../views/HomeView.vue')},
    {path: '/welcome', name: 'welcome', component: () => import('../views/WelcomeView.vue'), meta: {bare: true}},
    {path: '/setup', name: 'setup', component: () => import('../views/SetupView.vue'), meta: {bare: true}},
    {path: '/roster', name: 'roster', component: () => import('../views/RosterView.vue')},
    {path: '/bulk', name: 'bulk', component: () => import('../views/BulkView.vue')},
    {path: '/pending', name: 'pending', component: () => import('../views/PendingView.vue')},
    {path: '/settings', name: 'settings', component: () => import('../views/SettingsView.vue')},
]

export default createRouter({history: createWebHashHistory(), routes})
