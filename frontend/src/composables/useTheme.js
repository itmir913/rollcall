import {computed, ref} from 'vue'

/**
 * 테마 — light(기본) | dark | system
 *
 * 값은 DB가 아니라 localStorage에 둔다. index.html의 부트 스크립트가 첫 페인트
 * 전에 읽어야 하는데, DB 조회는 비동기라 그 시점에 값을 알 수 없어 흰 화면이
 * 한 번 번쩍인다. 테마는 이 기기의 화면 취향일 뿐 출결 데이터가 아니므로
 * 백업에 들어갈 이유도 없다.
 *
 * localStorage 접근은 실패할 수 있다(시크릿 모드, 사이트 데이터 차단). 실패해도
 * 라이트로 동작해야 하므로 읽기·쓰기를 모두 감싼다.
 */

const KEY = 'theme'
const MODES = ['light', 'dark', 'system']

function read() {
    try {
        const saved = localStorage.getItem(KEY)
        return MODES.includes(saved) ? saved : 'light'
    } catch {
        return 'light'
    }
}

function write(value) {
    try {
        localStorage.setItem(KEY, value)
    } catch {
        // 저장하지 못해도 이번 실행 동안은 적용된다.
    }
}

function prefersDark() {
    return window.matchMedia('(prefers-color-scheme: dark)').matches
}

export function resolveTheme(mode) {
    if (mode === 'dark') return 'dark'
    if (mode === 'system') return prefersDark() ? 'dark' : 'light'
    return 'light'
}

const mode = ref(read())
const resolved = computed(() => resolveTheme(mode.value))

function apply() {
    document.documentElement.setAttribute('data-theme', resolved.value)
}

// 시스템 따름일 때만 OS 설정 변화를 좇는다.
let watching = false

function watchSystem() {
    if (watching) return
    watching = true
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        if (mode.value === 'system') apply()
    })
}

export function useTheme() {
    watchSystem()
    apply()

    function setMode(next) {
        if (!MODES.includes(next)) return
        mode.value = next
        write(next)
        apply()
    }

    return {mode, resolved, modes: MODES, setMode}
}
