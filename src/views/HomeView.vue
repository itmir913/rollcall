<script setup>
/**
 * HOME — 대시보드.
 *
 * 입력 화면이 아니라 **오늘 할 일로 들어가는 입구**다. 교사가 앱을 켜는 이유는
 * 둘뿐이다: 오늘 출결을 넣거나, 아직 안 받은 서류를 챙기거나.
 *
 * 숫자는 전부 Rust의 `home_summary`가 세어 준다. 화면이 커맨드 대여섯 개를
 * 조합해 숫자를 만들면 그 조합 규칙이 프론트엔드의 비즈니스 로직이 된다.
 */
import {computed, onMounted, ref} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {useCheckStore} from '../stores/check'
import {useDayStore} from '../stores/day'
import {UiButton, UiCard, UiNotice, UiPage, UiTable} from '../components/ui'
import {formatKorean} from '../stores/day'

const app = useAppStore()
const axis = useAxisStore()
const check = useCheckStore()
const day = useDayStore()
const router = useRouter()

const incomplete = ref([])
const error = ref('')

const summary = computed(() => check.home)

const PENDING_COLUMNS = [
    {key: 'number', label: '번호', width: '72px'},
    {key: 'name', label: '성명', width: '110px'},
    {key: 'date', label: '결석일', width: '150px'},
    {key: 'item', label: '항목'},
    {key: 'due', label: '마감', width: '150px'},
]

const INCOMPLETE_COLUMNS = [
    {key: 'date', label: '날짜', width: '150px'},
    {key: 'who', label: '학생'},
    {key: 'state', label: '지금 상태'},
]

async function load() {
    if (!app.ready) return
    error.value = ''
    try {
        await Promise.all([
            check.fetchHome(app.yearId, app.grade, app.classNo, day.date),
            check.fetchPending(app.yearId, app.grade, app.classNo, day.date),
            axis.fetchAll(),
        ])
        incomplete.value = await day.fetchIncomplete(app.yearId, app.grade, app.classNo)
    } catch (e) {
        error.value = String(e)
    }
}

/** 마감이 지난 것부터 몇 건만. 전체는 미제출 화면이 보여준다. */
const overdueFirst = computed(() => check.pending.slice(0, 6))

function studentName(span) {
    const row = day.rows.find((r) => r.studentId === span.studentId)
    return row ? `${row.number}번 ${row.name}` : `학생 #${span.studentId}`
}

onMounted(load)
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn"
              text="아직 학생 명단이 없습니다. 명단을 넣으면 시작할 수 있습니다."/>

    <UiPage v-else :subtitle="summary ? summary.dateLabel : ''" title="오늘">
        <template #actions>
            <UiButton variant="primary" @click="router.push('/attendance')">
                오늘의 출결 입력
            </UiButton>
        </template>

        <!-- 숫자 요약 -->
        <div class="tiles">
            <div class="tile">
                <span class="tile__label">재학</span>
                <b class="tile__value">{{ summary?.enrolled ?? '—' }}</b>
                <span class="tile__unit">명</span>
            </div>
            <div class="tile">
                <span class="tile__label">오늘 기록</span>
                <b class="tile__value">{{ summary?.recorded ?? '—' }}</b>
                <span class="tile__unit">명</span>
            </div>
            <RouterLink :class="['tile', summary?.incomplete ? 'is-warn' : '']" to="/attendance">
                <span class="tile__label">구분 · 종류 미정</span>
                <b class="tile__value">{{ summary?.incomplete ?? '—' }}</b>
                <span class="tile__unit">건</span>
            </RouterLink>
            <RouterLink :class="['tile', summary?.overdue ? 'is-warn' : '']" to="/pending">
                <span class="tile__label">마감 지난 미제출</span>
                <b class="tile__value">{{ summary?.overdue ?? '—' }}</b>
                <span class="tile__unit">건</span>
            </RouterLink>
        </div>

        <!-- 채워야 할 기록 -->
        <UiCard v-if="incomplete.length"
                description="안 왔는데 연락이 닿지 않아 비워 둔 기록입니다. 출결 입력 화면에서 구분과 종류를 채우면 됩니다."
                title="아직 못 정한 출결">
            <template #actions>
                <UiButton @click="router.push('/attendance')">채우러 가기</UiButton>
            </template>
            <UiTable :columns="INCOMPLETE_COLUMNS" :rows="incomplete">
                <template #row="{row}">
                    <td>{{ formatKorean(row.date) }}</td>
                    <td>{{ studentName(row) }}</td>
                    <td class="is-warn-text">{{ axis.describe(row.reasonId, row.typeId) }}</td>
                </template>
            </UiTable>
        </UiCard>

        <!-- 미제출 -->
        <UiCard description="마감이 지난 것부터 보여줍니다." title="서류 · 나이스 미제출">
            <template #actions>
                <UiButton @click="router.push('/pending')">전체 보기</UiButton>
            </template>
            <UiTable :columns="PENDING_COLUMNS" :rows="overdueFirst"
                     empty-text="미제출 항목이 없습니다.">
                <template #row="{row}">
                    <td>{{ row.number }}</td>
                    <td>{{ row.name }}</td>
                    <td>{{ formatKorean(row.date) }}</td>
                    <td>{{ row.itemName }}</td>
                    <td :class="row.daysOverdue > 0 ? 'is-warn-text' : 'is-muted'">
                        {{ row.dueDate ? formatKorean(row.dueDate) : '마감 없음' }}
                    </td>
                </template>
            </UiTable>
            <p v-if="check.pending.length > overdueFirst.length" class="more">
                외 {{ check.pending.length - overdueFirst.length }}건
            </p>
        </UiCard>

        <UiNotice :text="error" kind="error"/>
    </UiPage>
</template>

<style scoped>
.tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 12px;
}

.tile {
    display: flex;
    align-items: baseline;
    gap: 6px;
    flex-wrap: wrap;
    padding: 16px 18px;
    border-radius: 12px;
    border: 1px solid var(--c-line);
    background: var(--c-surface);
    color: inherit;
    text-decoration: none;
}

a.tile:hover {
    border-color: var(--c-accent);
}

.tile.is-warn {
    border-color: var(--c-warn);
}

.tile__label {
    width: 100%;
    color: var(--c-ink-3);
}

.tile__value {
    font-size: 1.9rem;
    font-weight: 700;
    line-height: 1.1;
}

.tile.is-warn .tile__value {
    color: var(--c-warn);
}

.tile__unit {
    color: var(--c-ink-3);
}

.is-warn-text {
    color: var(--c-warn);
}

.is-muted {
    color: var(--c-ink-3);
}

.more {
    margin: 0;
    color: var(--c-ink-3);
}
</style>
