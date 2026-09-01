import {defineConfig} from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

/**
 * 웹앱 설정. 데스크톱 앱과 완전히 분리되어 있다.
 *
 * base가 '/rollcall/'인 이유: GitHub Pages의 프로젝트 페이지는
 * `https<:>//<사용자>.github.io/<저장소>/` 아래에 놓인다. 이걸 빼면 자원 경로가
 * 전부 루트('/assets/...')로 나가서 배포된 페이지가 빈 화면이 된다.
 * 로컬 dev에서는 상관없지만 빌드 결과가 달라지므로 여기서 못 박는다.
 */
export default defineConfig({
    base: '/rollcall/',
    plugins: [tailwindcss(), vue()],
    test: {
        environment: 'node',
    },
    server: {
        port: 1430,
    },
})
