<script setup>
/**
 * 이동 — 지금 보고 있는 학급 · 강좌를 바꾼다.
 *
 * 사이드바 구석의 드롭다운이 아니라 **한 화면**인 이유는, 옮기는 순간 명단도 기록도
 * 전부 바뀌기 때문이다. 모르고 지나갈 수 있는 변화가 아니라 눌렀다는 사실이 남아야 한다.
 *
 * **현재 모드의 것만 나열한다.** 담임 학급과 교과 강좌를 한 목록에 두면 잘못 누른 줄
 * 하나로 화면 절반이 다른 모드의 것으로 바뀐다. 모드를 넘나드는 길은 사이드바의
 * 스위치 하나뿐이다.
 */
import {computed} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {UiButton, UiLedger, UiNotice, UiPage} from '../components/ui'

const app = useAppStore()
const router = useRouter()

/** 지금 모드에서 맡은 것. 스토어가 이미 역할로 갈라 둔 것을 고르기만 한다. */
const classes = computed(() => (app.isHomeroom ? app.homeroomClasses : app.subjectClasses))

/**
 * 학교 이름표는 학교가 둘 이상일 때만 붙인다. 순회 교사가 아니면 모든 줄에 같은
 * 이름이 반복돼 정작 찾아야 할 학급 이름을 가린다.
 */
const manySchools = computed(() => app.schools.length > 1)

/**
 * 맡은 것이 하나뿐이면 이 화면에서 할 일이 없다. 빈손으로 돌려보내지 않고
 * 어디로 가야 하는지 적는다. 여럿이면 빈 문자열이라 아무것도 그리지 않는다.
 */
const note = computed(() =>
    classes.value.length > 1 ? '' : '맡은 것이 하나뿐입니다. 설정에서 더할 수 있습니다.',
)

function schoolName(item) {
    return app.schools.find((s) => s.id === item.schoolId)?.name ?? ''
}

/**
 * 명단 인원. 아직 맡은 것 목록이 인원을 세어 주지 않으므로 없으면 `—`로 둔다 —
 * 0으로 적으면 명단이 비었다는 뜻으로 읽힌다.
 */
function sizeText(item) {
    return item.memberCount == null ? '—' : `${item.memberCount}명`
}

/** 이미 보고 있는 줄은 누를 것이 없다. 오류는 app.error에 담겨 아래에 나온다. */
async function move(item) {
    if (item.id === app.classId) return
    try {
        await app.selectClass(item.id)
    } catch {
        // app.error가 화면에 그대로 보여준다.
    }
}
</script>

<template>
    <UiPage :subtitle="app.isHomeroom ? '담임 학급' : '교과 강좌'" title="이동">
        <template #actions>
            <UiButton @click="router.push('/settings')">설정에서 더하기</UiButton>
        </template>

        <UiLedger :empty="classes.length === 0" :note="note"
                  empty-text="이 모드에서 맡은 것이 없습니다."
                  hint="누르면 그 학급으로 옮긴다 · 명단도 기록도 함께 바뀐다"
                  title="맡은 것">
            <button v-for="item in classes" :key="item.id"
                    :class="['move', item.id === app.classId ? 'is-on' : '']"
                    type="button" @click="move(item)">
                <span class="move__what">
                    <span v-if="manySchools" class="move__school">{{ schoolName(item) }}</span>
                    <b class="move__name">{{ item.name }}</b>
                </span>
                <span class="move__count num">{{ sizeText(item) }}</span>
                <span class="move__here">{{ item.id === app.classId ? '지금 보는 중' : '' }}</span>
            </button>
        </UiLedger>

        <UiNotice :text="app.error" kind="error"/>
    </UiPage>
</template>

<style scoped>
.move {
    display: grid;
    grid-template-columns: 1fr 88px 104px;
    align-items: center;
    gap: var(--s-lg);
    width: 100%;
    padding: var(--s-lg) var(--s-2xl);
    border: 0;
    border-left: 3px solid transparent;
    background: transparent;
    color: var(--c-ink);
    text-align: left;
    cursor: pointer;
}

.move + .move {
    border-top: 1px solid var(--c-line-soft);
}

.move:hover {
    background: var(--c-raised);
}

/* 지금 있는 곳은 왼쪽 띠와 바탕으로 말한다. 줄의 높이는 그대로다. */
.move.is-on {
    border-left-color: var(--c-accent);
    background: var(--c-accent-soft);
}

.move__what {
    display: flex;
    align-items: baseline;
    gap: var(--s-md);
    min-width: 0;
}

.move__school {
    color: var(--c-ink-3);
}

.move__name {
    font-size: var(--t-md);
}

.move__count {
    color: var(--c-ink-2);
    text-align: right;
}

/* 빈 줄에도 자리를 남긴다. 칸이 접히면 줄마다 열 폭이 달라져 목록이 흔들린다. */
.move__here {
    color: var(--c-accent);
    font-weight: 700;
    text-align: right;
}
</style>
