import {defineConfig} from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

const host = process.env.TAURI_DEV_HOST

/**
 * 데스크톱(Tauri) 앱의 프런트엔드 설정.
 *
 * 소스는 `frontend/`에 있고, 웹앱은 `web/`에 자기 설정과 자기 package.json을
 * 따로 가진다. 앱마다 dev/build/test 세 명령이 하나씩 있어야 "지금 dev가 어느
 * 앱을 띄우는가"를 되묻지 않는다.
 *
 * 이 파일이 저장소 뿌리에 남아 있는 이유는 Tauri의 beforeDevCommand가 뿌리에서
 * 실행되기 때문이다. root만 frontend/로 돌린다.
 */
export default defineConfig(async () => ({
    root: 'frontend',
    plugins: [tailwindcss(), vue()],
    test: {
        environment: 'node',
    },
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host ? {protocol: 'ws', host, port: 1421} : undefined,
        watch: {
            ignored: ['**/src-tauri/**'],
        },
    },
}))
