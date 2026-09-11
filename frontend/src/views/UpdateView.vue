<script setup>
/**
 * 업데이트 확인.
 *
 * **아직 갱신 기능이 붙어 있지 않다.** 그래서 "최신입니다" 같은 말을 하지 않는다 —
 * 확인한 적이 없는데 확인했다고 말하는 것은 조용한 실패다.
 */
import {UiLedger, UiPage} from '../components/ui'

const version = __APP_VERSION__
</script>

<template>
    <UiPage subtitle="지금 쓰는 판" title="업데이트 확인">
        <UiLedger title="버전">
            <div class="set__row">
                <span class="set__label">출결관리</span>
                <span class="set__value"><span class="num">{{ version }}</span></span>
            </div>
            <div class="set__row">
                <span class="set__label">갱신 확인</span>
                <span class="set__value">
                    <span class="set__hint">
                        아직 갱신 기능이 연결되지 않았습니다. 새 판이 나왔는지 이 화면이 알 수 없으므로,
                        확인했다고 말하지 않습니다.
                    </span>
                </span>
            </div>
        </UiLedger>
    </UiPage>
</template>
