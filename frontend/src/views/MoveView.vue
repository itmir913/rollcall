<script setup>
/**
 * 이동 — 지금 보고 있는 학교와 학급 · 강좌를 바꾼다.
 *
 * 사이드바 구석의 드롭다운이 아니라 **한 화면**인 이유는, 옮기는 순간 명단도 기록도
 * 전부 바뀌기 때문이다. 모르고 지나갈 수 있는 변화가 아니라 눌렀다는 사실이 남아야 한다.
 *
 * **계층 그대로 보여준다 — 학년도 → 학교 → 담당 학급 · 강좌.** 순회 교사는 하루에
 * 학교를 옮겨 다니고, 학교가 결정되어야 담당 학급 · 강좌가 결정된다. 학년도는 여기서
 * 바꾸지 않는다 — 해가 바뀌면 학교부터 다시 선택해야 하므로 설정이 담당한다.
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

/** 지금 모드의 담당 학급 · 강좌. 스토어가 이미 역할로 구별해 둔 것을 선택만 한다. */
const classes = computed(() => (app.isHomeroom ? app.homeroomClasses : app.subjectClasses))

/**
 * 학교 이름표는 학교가 둘 이상일 때만 붙인다. 순회 교사가 아니면 모든 줄에 같은
 * 이름이 반복돼 정작 찾아야 할 학급 이름을 가린다.
 */
const manySchools = computed(() => app.schools.length > 1)

/**
 * 교과 강좌를 묶음별로 모은다. `프로그래밍A · B · C`가 흩어져 있으면 그중 하나를
 * 선택하려고 목록 전체를 훑게 된다. **묶음은 이름표일 뿐 분반 표가 아니다** —
 * 강좌는 저마다 독립된 줄이고 선택하는 것도 줄 하나다.
 *
 * 담임 학급은 묶지 않는다. 묶을 것이 없고, 머리글만 하나 더 생긴다.
 */
const groups = computed(() => {
    const items = classes.value
    if (app.isHomeroom) return [{key: 'all', name: '', items}]

    const found = new Map()
    for (const item of items) {
        const key = item.groupTagId ?? 'none'
        if (!found.has(key)) {
            found.set(key, {key: String(key), name: groupName(item), items: []})
        }
        found.get(key).items.push(item)
    }
    const list = [...found.values()]
    // 묶음 없는 것은 맨 뒤로 보낸다. 이름이 있는 묶음이 먼저다.
    return [...list.filter((g) => g.key !== 'none'), ...list.filter((g) => g.key === 'none')]
})

/** 머리글을 그릴지. 묶음이 하나뿐이면 이름표가 아니라 군더더기다. */
const showGroups = computed(() => groups.value.length > 1)

/**
 * 담당 학급 · 강좌가 하나뿐이면 이 화면에서 할 일이 없다. 빈손으로 돌려보내지 않고
 * 어디로 가야 하는지 적는다. 여럿이면 빈 문자열이라 아무것도 그리지 않는다.
 *
 * **없는 것과 하나뿐인 것을 구별한다.** 0개일 때 "하나뿐입니다"라고 적으면 화면이
 * 사실과 다른 말을 하고, 바로 아래의 "담당하는 학급이나 강좌가 없습니다"와도 어긋난다.
 */
const note = computed(() => {
    if (classes.value.length === 0) return '설정에서 추가할 수 있습니다.'
    if (classes.value.length === 1) return '담당 학급 · 강좌가 하나뿐입니다. 설정에서 추가할 수 있습니다.'
    return ''
})

function groupName(item) {
    if (item.groupTagId == null) return '묶음 없음'
    return item.groupTagName ?? '이름 없는 묶음'
}

function schoolName(item) {
    return app.schools.find((s) => s.id === item.schoolId)?.name ?? ''
}

/**
 * 명단 인원. 아직 담당 학급 · 강좌 목록이 인원을 세어 주지 않으므로 없으면 `—`로 둔다 —
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

/**
 * 학교를 옮긴다. **담당 학급 · 강좌 목록이 그 학교의 것으로 통째로 바뀐다** —
 * 학급과 강좌는 학교에 소속되어 있고, 순회 교사는 학교마다 다른 것을 담당한다.
 */
async function moveSchool(schoolId) {
    if (schoolId === app.schoolId) return
    try {
        await app.selectSchool(schoolId)
    } catch {
        // app.error가 화면에 그대로 보여준다.
    }
}
</script>

<template>
    <UiPage :subtitle="app.isHomeroom ? '담임 학급' : '교과 강좌'" title="이동">
        <template #actions>
            <UiButton @click="router.push('/settings')">설정에서 추가</UiButton>
        </template>

        <UiLedger hint="학년도 → 학교 → 학급과 강좌 · 위에서 선택한 것이 아래의 범위다" title="어디에서">
            <div class="set__row">
                <span class="set__label">학년도</span>
                <span class="set__value">
                    <span class="num">{{ app.currentYear?.year ?? '—' }}</span>학년도
                    <span class="set__hint">
                        학년도를 옮기면 학교부터 다시 선택해야 하므로 설정에서 바꿉니다
                    </span>
                </span>
            </div>

            <div class="set__row">
                <span class="set__label">학교</span>
                <span class="set__value">
                    <button v-for="item in app.schools" :key="item.id"
                            :class="['pick', app.schoolId === item.id ? 'is-on' : '']"
                            type="button" @click="moveSchool(item.id)">
                        {{ item.name }}
                    </button>
                    <span v-if="!app.schools.length" class="set__hint">
                        이 학년도에 추가한 학교가 없습니다. 설정에서 추가해주세요
                    </span>
                    <span v-else class="set__hint">누르면 아래 목록이 그 학교의 것으로 바뀝니다</span>
                </span>
            </div>
        </UiLedger>

        <UiLedger :empty="classes.length === 0" :note="note"
                  empty-text="이 모드에서 담당하는 학급이나 강좌가 없습니다."
                  hint="누르면 그 학급으로 옮긴다 · 명단도 기록도 함께 바뀐다"
                  title="담당 학급 · 강좌">
            <template v-for="group in groups" :key="group.key">
                <!-- 묶음 머리글. 이름표일 뿐이라 누르는 것이 아니다. -->
                <div v-if="showGroups && group.name" class="movegrp">
                    <b>{{ group.name }}</b>
                    <span class="set__hint"><span class="num">{{ group.items.length }}</span>개 강좌</span>
                </div>
                <button v-for="item in group.items" :key="item.id"
                        :class="['move', item.id === app.classId ? 'is-on' : '']"
                        type="button" @click="move(item)">
                    <span class="move__what">
                        <span v-if="manySchools" class="move__school">{{ schoolName(item) }}</span>
                        <b class="move__name">{{ item.name }}</b>
                    </span>
                    <span class="move__count num">{{ sizeText(item) }}</span>
                    <span class="move__here">{{ item.id === app.classId ? '지금 보는 중' : '' }}</span>
                </button>
            </template>
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

/* 묶음 머리글. 줄이 아니라 이름표라 한 단 눌러 둔다. */
.movegrp {
    display: flex;
    align-items: baseline;
    gap: var(--s-md);
    padding: var(--s-sm) var(--s-2xl);
    border-top: 1px solid var(--c-line-soft);
    background: var(--c-raised);
}
</style>
