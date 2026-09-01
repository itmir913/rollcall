<script setup>
/**
 * 첫 실행 인사.
 *
 * 여기서 설명하는 것은 셋뿐이다: 무엇을 하는 프로그램인지, 어디에 저장되는지,
 * 다음에 무엇을 해야 하는지. 기능 소개를 늘어놓지 않는다 — 교사는 오늘 출결을
 * 넣으러 온 것이지 매뉴얼을 읽으러 온 것이 아니다.
 */
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {UiButton, UiCard} from '../components/ui'

const app = useAppStore()
const router = useRouter()
</script>

<template>
    <div class="welcome">
        <div class="welcome__inner">
            <p class="welcome__eyebrow">담임 교사를 위한 출결 기록</p>
            <h1 class="welcome__title">출결관리</h1>
            <p class="welcome__lead">
                번호와 단축키만 치면 한 건이 들어갑니다.<br/>
                나이스 입력과 증빙 서류를 놓치지 않게 함께 챙깁니다.
            </p>

            <UiCard class="welcome__card">
                <dl class="welcome__facts">
                    <div>
                        <dt>어떻게 입력하나요</dt>
                        <dd>
                            구분을 누르고 종류를 누른 뒤, 학생 이름을 누르면 한 건이 들어갑니다.
                            같은 사유면 계속 눌러 나가면 됩니다.
                        </dd>
                    </div>
                    <div>
                        <dt>어디에 저장되나요</dt>
                        <dd>
                            이 컴퓨터 안에만 저장됩니다. 인터넷으로 나가지 않습니다.
                            <span class="welcome__path">{{ app.status?.path }}</span>
                        </dd>
                    </div>
                    <div>
                        <dt>다음 순서</dt>
                        <dd>
                            명렬표 엑셀 파일을 끌어다 놓으면 끝입니다. 학년·반도 파일에서
                            읽습니다. 양식이 없으면 내려받을 수 있고, 학년도는 앱이 정합니다.
                        </dd>
                    </div>
                </dl>
            </UiCard>

            <div class="welcome__actions">
                <UiButton variant="primary" @click="router.push('/roster')">
                    명렬표 넣고 시작하기
                </UiButton>
                <UiButton variant="ghost" @click="router.push('/')">
                    나중에 하기
                </UiButton>
            </div>

            <p class="welcome__note">
                아직 데이터 파일이 암호화되지 않습니다. 건강정보가 들어가므로,
                공유 폴더나 클라우드 동기화 폴더에는 두지 마세요.
            </p>
        </div>
    </div>
</template>

<style scoped>
.welcome {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 40px 24px;
}

.welcome__inner {
    width: 100%;
    max-width: 560px;
    display: flex;
    flex-direction: column;
    gap: 18px;
}

.welcome__eyebrow {
    margin: 0;
    color: var(--c-ink-3);
}

.welcome__title {
    margin: 0;
    font-size: 2.4rem;
    font-weight: 700;
    letter-spacing: -0.02em;
}

.welcome__lead {
    margin: 0;
    color: var(--c-ink-2);
    line-height: 1.7;
}

.welcome__card {
    margin-top: 6px;
}

.welcome__facts {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
}

.welcome__facts dt {
    color: var(--c-ink-3);
    margin-bottom: 4px;
}

.welcome__facts dd {
    margin: 0;
    line-height: 1.6;
}

.welcome__facts code {
    padding: 2px 6px;
    border-radius: 5px;
    background: var(--c-raised);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.welcome__path {
    display: block;
    margin-top: 4px;
    color: var(--c-ink-3);
    word-break: break-all;
}

.welcome__actions {
    display: flex;
    gap: 8px;
    align-items: center;
}

.welcome__note {
    margin: 0;
    color: var(--c-warn);
    line-height: 1.6;
}
</style>
