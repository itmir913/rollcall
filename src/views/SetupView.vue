<script setup>
/** 초기 설정 — Welcome 다음, HOME 앞. 학년도와 우리 반만 정한다. */
import {onMounted, ref} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {UiButton, UiCard, UiNotice} from '../components/ui'

const app = useAppStore()
const router = useRouter()

const year = ref(new Date().getFullYear())
const grade = ref(3)
const classNo = ref(6)
const selectedYearId = ref(null)
const error = ref('')

onMounted(async () => {
    await app.fetchYears()
    selectedYearId.value = app.yearId ?? app.years[0]?.id ?? null
    if (app.grade) grade.value = app.grade
    if (app.classNo) classNo.value = app.classNo
})

async function addYear() {
    error.value = ''
    try {
        const value = Number(year.value)
        selectedYearId.value = await app.createYear(
            value, `${value}-03-01`, `${value + 1}-02-28`,
        )
    } catch (e) {
        error.value = String(e)
    }
}

async function save() {
    error.value = ''
    if (!selectedYearId.value) return (error.value = '학년도를 먼저 만들어주세요.')
    try {
        await app.setContext(selectedYearId.value, Number(grade.value), Number(classNo.value))
        await router.push('/roster')
    } catch (e) {
        error.value = String(e)
    }
}
</script>

<template>
    <div class="setup">
        <div class="setup__inner">
            <header>
                <p class="setup__step">초기 설정</p>
                <h1 class="setup__title">학년도와 우리 반</h1>
            </header>

            <UiCard description="학생과 출결 기록은 학년도에 속합니다. 해가 바뀌어도 작년 기록은 그대로 남습니다."
                    title="학년도">
                <div v-if="app.years.length" class="setup__row">
                    <UiButton v-for="y in app.years" :key="y.id"
                              :variant="selectedYearId === y.id ? 'primary' : 'default'"
                              @click="selectedYearId = y.id">
                        {{ y.year }}학년도
                    </UiButton>
                </div>
                <div class="setup__row">
                    <input v-model="year" class="field setup__num" type="number"/>
                    <UiButton @click="addYear">학년도 추가</UiButton>
                </div>
            </UiCard>

            <UiCard title="우리 반">
                <div class="setup__row">
                    <input v-model="grade" class="field setup__num" min="1" type="number"/>
                    <span>학년</span>
                    <input v-model="classNo" class="field setup__num" min="1" type="number"/>
                    <span>반</span>
                </div>
            </UiCard>

            <UiNotice :text="error" kind="error"/>

            <UiButton variant="primary" @click="save">저장하고 명단 넣기</UiButton>

            <p class="setup__path">데이터 파일: {{ app.status?.path }}</p>
        </div>
    </div>
</template>

<style scoped>
.setup {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 40px 24px;
}

.setup__inner {
    width: 100%;
    max-width: 560px;
    display: flex;
    flex-direction: column;
    gap: 16px;
}

.setup__step {
    margin: 0;
    color: var(--c-ink-3);
}

.setup__title {
    margin: 4px 0 0;
    font-size: 1.8rem;
    font-weight: 700;
}

.setup__row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
}

.setup__num {
    width: 100px;
}

.setup__path {
    margin: 0;
    color: var(--c-ink-3);
    word-break: break-all;
}
</style>
