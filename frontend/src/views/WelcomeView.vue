<script setup>
/**
 * 첫 실행 — 다섯 단계로 학년도 · 학교 · 맡은 것을 정하고 명렬표를 넣는다.
 *
 * ```
 * 1 시작하기   여기서 하는 일을 세 줄로 알린다
 * 2 학년도     오늘 날짜로 채워져 있다. 3월에 열린다
 * 3 학교       이름 · 최대 교시 · 서류 제출 기한
 * 4 맡은 것    담임 학급 · 교과 강좌를 더하고 각각 명렬표를 넣는다
 * 5 완료       등록한 것을 요약하고 개요로 보낸다
 * ```
 *
 * 이 화면은 **첫 실행에서만** 지나간다. 매일 열자마자 바로 입력할 수 있어야 하므로
 * 개요가 곧 기본 화면이고, 여기는 그 앞에 한 번 서는 자리다.
 *
 * **단계를 눌러 앞뒤로 오간다.** 앞 단계를 잠그면 학교 이름을 잘못 적은 교사가
 * 되돌아갈 길이 없어 앱을 껐다 켜게 된다. 잠그는 것은 하나뿐이다 — 맡은 것을 하나도
 * 등록하지 않으면 완료로 갈 수 없다. 그때도 **단추를 숨기지 않고 `disabled`로 둔다.**
 * 단추가 사라지면 화면이 움직이고, 교사는 자기가 무엇을 놓쳤는지 알 수 없다.
 *
 * **담임과 교과를 여기서는 함께 등록한다.** 화면이 갈리는 것은 등록을 마친 다음부터다 —
 * 맡은 것이 아직 하나도 없는 상태에서 모드를 먼저 고르게 하면, 교사는 자기가 무엇을
 * 고르는지 모르는 채 고르게 된다.
 */
import {computed, nextTick, onMounted, ref, watch} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useRosterStore} from '../stores/roster'
import {useSchoolStore} from '../stores/school'
import RosterPanel from '../components/RosterPanel.vue'
import {UiButton, UiLedger, UiNotice, UiPage} from '../components/ui'
import {MAX_SLOT_CHOICES} from '../data/slotChoices'
import {academicYearOf, yearSpanOf} from '../services/academicYear'

const app = useAppStore()
const roster = useRosterStore()
const school = useSchoolStore()
const router = useRouter()

const STEPS = ['시작하기', '학년도', '학교', '맡은 것', '완료']

/** 첫 단계에서 알리는 것. 세 줄을 넘기지 않는다 — 읽지 않는 안내는 없는 것과 같다. */
const INTRO = [
    '학년도와 학교를 정합니다. 최대 교시와 서류 제출 기한은 학교마다 다릅니다.',
    '맡은 것을 등록합니다. 담임 학급과 교과 강좌를 함께 넣을 수 있습니다.',
    '담임 학급의 명렬표를 파일에서 가져옵니다. 번호와 이름만 있으면 됩니다.',
]

const step = ref(1)
const error = ref('')
const dueDays = ref(7)
const homeroomForm = ref({grade: 3, classNo: 1})
const subjectForm = ref({name: ''})

/** 명렬표를 펼친 학급. 한 번에 하나만 연다 — 두 개가 열리면 어느 명단인지 흐려진다. */
const openId = ref(null)

/** 학급마다 지금 명단에 있는 인원. 넣고 나면 몇 명인지 보여야 한다. */
const counts = ref({})

/** 이번 학년도가 열리고 닫히는 날. 교사에게 되묻지 않고 오늘 날짜로 채운 값이다. */
const span = computed(() => yearSpanOf(app.currentYear?.year ?? academicYearOf(app.today)))

const hasClasses = computed(() => app.classes.length > 0)

/**
 * 명단에 올린 학생 수. **담임과 교과에 함께 있는 학생은 두 번 센다** —
 * 명단 단위의 합이라 그렇게 세는 것이 맞고, 화면에도 그렇게 적는다.
 */
const totalStudents = computed(() =>
    app.classes.reduce((sum, cls) => sum + (counts.value[cls.id] ?? 0), 0))

/**
 * 학교를 더할 수 있는가. 순회 교사만 누르는 자리라 점선 단추로 눈에 띄지 않게 둔다.
 *
 * 스토어에 액션이 아직 없으면 **잠근 채로 그린다.** 눌러도 아무 일이 없는 단추보다
 * 잠긴 단추가 낫고, 자리를 비우면 액션이 들어온 날 화면이 달라진다.
 */
const canAddSchool = computed(() => typeof school.createSchool === 'function')

/** 완료로 갈 수 없는 단계. 맡은 것이 하나도 없으면 요약할 것도 없다. */
function blocked(n) {
    return n === STEPS.length && !hasClasses.value
}

const canNext = computed(() => step.value === STEPS.length || !blocked(step.value + 1))

function go(n) {
    if (n < 1 || n > STEPS.length || blocked(n)) return
    step.value = n
    error.value = ''
}

/** 마지막 단계의 [다음]은 개요로 보낸다. 흐름은 `Welcome → 개요` 하나뿐이다. */
function next() {
    if (step.value === STEPS.length) router.push('/')
    else go(step.value + 1)
}

/**
 * 학년도를 고른다. **여기서는 앱 상태만 바꾼다** — 이 값이 DB에 적히는 것은
 * 학급을 고르는 순간이다(`app.selectClass`가 `yearId`를 함께 저장한다).
 * 학급마다 학년도가 붙어 있으므로, 학급 없이 학년도만 저장해 둘 자리가 없다.
 */
function pickYear(yearId) {
    app.yearId = yearId
}

/** 학교가 둘 이상일 때만 학급 줄에 붙는 이름표. 하나뿐이면 적을 것이 없다. */
function schoolLabel(cls) {
    if (app.schools.length < 2) return ''
    return app.schools.find((s) => s.id === cls.schoolId)?.name ?? ''
}

/** 그 학급의 명단 상태. 0명은 "아직 안 넣었다"는 뜻이라 숫자로 적지 않는다. */
function rosterLabel(classId) {
    const n = counts.value[classId] ?? 0
    return n > 0 ? `재학 ${n}명` : '명렬표를 아직 넣지 않았습니다'
}

async function saveName(name) {
    error.value = ''
    await school.saveSchool({name: name.trim() || '우리 학교'}).catch(() => {
    })
}

async function saveDueDays() {
    const days = Number(dueDays.value)
    if (!Number.isInteger(days) || days < 0) {
        error.value = '서류 제출 기한은 0 이상의 날 수로 적어주세요.'
        return
    }
    error.value = ''
    await school.saveSchool({dueDays: days}).catch(() => {
    })
}

async function addSchool() {
    if (!canAddSchool.value) return
    error.value = ''
    try {
        await school.createSchool({name: '새 학교', maxSlot: 7, dueDays: 7})
        await app.refreshSchool()
    } catch (e) {
        error.value = String(e)
    }
}

/**
 * 담임 학급을 더한다. 이름은 여기서 `3학년 6반`으로 짓는다 — 화면에 적히는 값이다.
 *
 * 같은 학년 · 반을 다시 더해도 학급이 늘지 않는 것은 `app.createClass`가 맡는다.
 * 교사가 [추가]를 두 번 누르는 것은 흔한 일이고, 그때마다 학급이 늘면 명단이
 * 어느 쪽에 붙었는지 알 수 없게 된다.
 */
async function addHomeroom() {
    const grade = Number(homeroomForm.value.grade)
    const classNo = Number(homeroomForm.value.classNo)
    if (!Number.isInteger(grade) || !Number.isInteger(classNo) || grade < 1 || classNo < 1) {
        error.value = '학년과 반을 1 이상의 숫자로 적어주세요.'
        return
    }
    await add({role: 'homeroom', name: `${grade}학년 ${classNo}반`, grade, classNo})
}

/**
 * 교과 강좌를 더한다. **학년 · 반을 받지 않는다** — 선택과목은 1반부터 n반까지
 * 섞여서 가리킬 반이 없다. 이름 하나가 그 강좌를 가리킨다.
 */
async function addSubject() {
    const name = subjectForm.value.name.trim()
    if (!name) {
        error.value = '강좌 이름을 적어주세요.'
        return
    }
    const id = await add({role: 'subject', name, grade: null, classNo: null})
    if (id != null) subjectForm.value.name = ''
}

/** 만들고, 인원을 다시 세고, 그 학급의 명렬표를 바로 펼친다. 다음에 할 일이 그것이다. */
async function add(payload) {
    error.value = ''
    try {
        const id = await app.createClass(payload)
        await refreshCounts()
        openId.value = id
        return id
    } catch (e) {
        error.value = String(e)
        return null
    }
}

/**
 * 명렬표를 펼친다. **그 학급을 함께 고른다** — 명단이 붙는 곳은 지금 고른 학급이라,
 * 펼친 줄과 고른 학급이 어긋나면 다른 반 명단에 들어간다.
 */
async function toggleRoster(cls) {
    if (openId.value === cls.id) {
        openId.value = null
        return
    }
    error.value = ''
    try {
        await app.selectClass(cls.id)
        openId.value = cls.id
    } catch (e) {
        error.value = String(e)
    }
}

/**
 * 세는 동안에는 아래 워처를 멈춘다.
 *
 * 스토어의 명단은 하나뿐이라, 학급을 돌며 읽는 사이에 워처가 끼어들어 **지금 읽은
 * 다른 학급의 인원**을 펼쳐 둔 학급 칸에 적는다. 30명을 막 넣은 줄이 `0명`으로
 * 되돌아가면, 교사는 저장이 안 된 줄 알고 다시 넣는다.
 */
let counting = false

/** 학급마다 인원을 다시 센다. */
async function refreshCounts() {
    counting = true
    try {
        for (const cls of app.classes) {
            // 한 학급을 못 읽어도 나머지는 센다. 실패는 roster.error가 보여준다.
            await roster.fetchStudents(cls.id).catch(() => {
            })
            counts.value = {...counts.value, [cls.id]: roster.students.length}
        }
    } finally {
        // 워처는 flush 전에 도므로 한 틱을 기다린 뒤 되살린다.
        await nextTick()
        counting = false
    }
}

// 명렬표를 저장하면 RosterPanel이 그 학급의 명단을 다시 읽는다. 그 결과를 그대로
// 받아 인원을 고친다 — 넣은 직후 몇 명인지 그 자리에서 보여야 한다.
watch(() => roster.students, (list) => {
    if (counting || openId.value == null) return
    counts.value = {...counts.value, [openId.value]: list.length}
})

// 읽기에 실패해도 화면은 그린다. 실패는 각 스토어의 error에 담겨 아래 UiNotice가
// 그대로 보여준다 — 첫 화면이 기본값만 놓인 멀쩡한 모습으로 보이면 안 된다.
onMounted(async () => {
    await school.fetchAll().catch(() => {
    })
    dueDays.value = school.school?.dueDays ?? 7
    await refreshCounts()
})
</script>

<template>
    <UiPage subtitle="학년도 · 학교 · 맡은 것을 한 번만 정하면, 다음부터는 개요가 바로 열립니다"
            title="출결관리를 시작합니다">
        <div class="steps">
            <template v-for="(name, i) in STEPS" :key="name">
                <span v-if="i > 0" class="step__arrow">→</span>
                <button :class="['step', step === i + 1 ? 'is-now' : step > i + 1 ? 'is-done' : '']"
                        :disabled="blocked(i + 1)" type="button" @click="go(i + 1)">
                    <span class="step__no num">{{ step > i + 1 ? '✓' : i + 1 }}</span>{{ name }}
                </button>
            </template>
        </div>

        <UiNotice :text="error" kind="error"/>
        <UiNotice :text="app.error" kind="error"/>
        <UiNotice :text="school.error" kind="error"/>
        <UiNotice :text="roster.error" kind="error"/>

        <!-- 1 시작하기 ─ 여기서 하는 일 -->
        <UiLedger v-if="step === 1" hint="세 가지뿐입니다" title="여기서 하는 일">
            <div v-for="(line, i) in INTRO" :key="i" class="row is-calm lead">
                <span class="row__no num">{{ i + 1 }}</span>
                <span class="row__what">{{ line }}</span>
            </div>
            <template #foot>
                <span>여기서 정한 것은 전부 설정에서 다시 바꿀 수 있습니다.</span>
            </template>
        </UiLedger>

        <!-- 2 학년도 ─ 오늘 날짜로 채워 둔다 -->
        <UiLedger v-if="step === 2" hint="오늘 날짜로 채워 두었습니다" title="학년도">
            <div class="set__row">
                <span class="set__label">이번 학년도</span>
                <span class="set__value">
                    <button v-for="year in app.years" :key="year.id"
                            :class="['pick', app.yearId === year.id ? 'is-on' : '']"
                            type="button" @click="pickYear(year.id)">
                        {{ year.year }}학년도
                    </button>
                    <span class="set__hint">
                        학년도는 3월에 열립니다 —
                        <span class="num">{{ span.startsOn }}</span> ~
                        <span class="num">{{ span.endsOn }}</span>.
                        1 · 2월은 지난해에 열린 학년도의 끝자락입니다.
                    </span>
                </span>
            </div>
            <template #foot>
                <span>학년도는 학급이 들고 있습니다. 다음 단계에서 만드는 학급이 이 학년도에 붙습니다.</span>
            </template>
        </UiLedger>

        <!-- 3 학교 ─ 이름 · 최대 교시 · 서류 제출 기한 -->
        <UiLedger v-if="step === 3" hint="학교마다 따로 가지는 값입니다" title="학교">
            <div class="set__row">
                <span class="set__label">학교 이름</span>
                <span class="set__value">
                    <!-- 읽지 못했으면 잠근다. 빈 칸에 적어 저장하면 무엇을 고치는지 알 수 없다. -->
                    <input :disabled="!school.school" :value="school.school?.name ?? ''"
                           class="field" placeholder="한빛고등학교" type="text"
                           @change="saveName($event.target.value)"/>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">최대 교시</span>
                <span class="set__value">
                    <button v-for="n in MAX_SLOT_CHOICES" :key="n"
                            :class="['pick', 'pick--slot', school.school?.maxSlot === n ? 'is-on' : '']"
                            :disabled="!school.school" type="button"
                            @click="school.saveSchool({maxSlot: n}).catch(() => {})">
                        {{ n }}
                    </button>
                    <span class="set__hint">조회와 종례는 언제나 하루의 양 끝입니다</span>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">서류 제출 기한</span>
                <span class="set__value">
                    <input v-model="dueDays" :disabled="!school.school" class="field num" min="0"
                           type="number" @change="saveDueDays"/>
                    <span class="set__hint">일</span>
                    <span class="set__hint">결석일로부터 셉니다. 미제출자 명단의 마감이 이 값으로 계산됩니다</span>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">학교 추가</span>
                <span class="set__value">
                    <button :disabled="!canAddSchool" class="dashed" type="button" @click="addSchool">
                        ＋ 학교 추가
                    </button>
                    <span class="set__hint">
                        {{
                            canAddSchool
                                ? '순회 교사만 누릅니다. 학교가 둘이 되면 학급 줄에 학교 이름표가 붙습니다'
                                : '학교를 더하는 것은 아직 열리지 않았습니다 — 지금은 학교 하나를 씁니다'
                        }}
                    </span>
                </span>
            </div>
        </UiLedger>

        <!-- 4 맡은 것 ─ 담임 학급 · 교과 강좌 -->
        <template v-if="step === 4">
            <UiLedger class="mine mine--homeroom" hint="학년 · 반으로 이름을 짓습니다" title="담임 학급">
                <template v-for="cls in app.homeroomClasses" :key="cls.id">
                    <div class="row is-calm">
                        <span class="row__name"><b>{{ cls.name }}</b></span>
                        <span class="row__what">{{ schoolLabel(cls) }}</span>
                        <span class="row__when">{{ rosterLabel(cls.id) }}</span>
                        <span class="row__acts">
                            <UiButton size="tight" @click="toggleRoster(cls)">
                                {{ openId === cls.id ? '명렬표 닫기' : '명렬표 넣기' }}
                            </UiButton>
                        </span>
                    </div>
                    <div v-if="openId === cls.id" class="mine__panel">
                        <RosterPanel :class-id="cls.id"/>
                    </div>
                </template>

                <div class="set__row">
                    <span class="set__label">담임 학급 추가</span>
                    <span class="set__value">
                        <input v-model="homeroomForm.grade" class="field num" min="1" type="number"/>
                        <span class="set__hint">학년</span>
                        <input v-model="homeroomForm.classNo" class="field num" min="1" type="number"/>
                        <span class="set__hint">반</span>
                        <UiButton size="tight" variant="primary" @click="addHomeroom">추가</UiButton>
                        <span class="set__hint">이름은 3학년 6반처럼 짓습니다</span>
                    </span>
                </div>
            </UiLedger>

            <UiLedger class="mine mine--subject" hint="이름만 받습니다" title="교과 강좌">
                <template v-for="cls in app.subjectClasses" :key="cls.id">
                    <div class="row is-calm">
                        <span class="row__name"><b>{{ cls.name }}</b></span>
                        <span class="row__what">{{ schoolLabel(cls) }}</span>
                        <span class="row__when">{{ rosterLabel(cls.id) }}</span>
                        <span class="row__acts">
                            <!-- 교과 강좌는 반이 섞여 번호만으로 학생을 가릴 수 없다.
                                 명렬표 들이기가 아직 담임만이라 단추를 열지 않는다 —
                                 눌러도 되지 않는 단추가 아무 말 없이 실패하는 것보다
                                 아직 없다고 적는 편이 낫다. -->
                            <UiButton disabled size="tight" title="다음 판에서 만듭니다">
                                명렬표는 아직
                            </UiButton>
                        </span>
                    </div>
                </template>

                <div class="set__row">
                    <span class="set__label">교과 강좌 추가</span>
                    <span class="set__value">
                        <input v-model="subjectForm.name" class="field" placeholder="인공지능기초A"
                               type="text" @keyup.enter="addSubject"/>
                        <UiButton size="tight" variant="primary" @click="addSubject">추가</UiButton>
                        <span class="set__hint">
                            반이 섞입니다 — 선택과목은 1반부터 n반까지 모이므로 학년 · 반을 묻지 않습니다
                        </span>
                    </span>
                </div>

                <template #foot>
                    <span>하나도 등록하지 않으면 완료로 갈 수 없습니다. 담임만, 교과만 맡아도 됩니다.</span>
                </template>
            </UiLedger>
        </template>

        <!-- 5 완료 ─ 등록한 것을 요약한다 -->
        <UiLedger v-if="step === 5" hint="이대로 시작합니다" title="등록한 것">
            <div class="set__row">
                <span class="set__label">학년도</span>
                <span class="set__value">
                    <span class="num">{{ app.currentYear?.year ?? academicYearOf(app.today) }}</span>학년도
                    <span class="set__hint">
                        <span class="num">{{ span.startsOn }}</span> ~
                        <span class="num">{{ span.endsOn }}</span>
                    </span>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">학교</span>
                <span class="set__value">
                    {{ school.school?.name ?? app.school?.name ?? '이름 없음' }}
                    <span class="set__hint">
                        최대 <span class="num">{{ app.maxSlot }}</span>교시 ·
                        서류 제출 기한 <span class="num">{{ school.school?.dueDays ?? dueDays }}</span>일
                    </span>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">담임 학급</span>
                <span class="set__value">
                    <span v-for="cls in app.homeroomClasses" :key="cls.id" class="sum">
                        {{ cls.name }} · <span class="num">{{ counts[cls.id] ?? 0 }}</span>명
                    </span>
                    <span class="set__hint">{{ app.homeroomClasses.length ? '' : '맡은 담임 학급이 없습니다' }}</span>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">교과 강좌</span>
                <span class="set__value">
                    <span v-for="cls in app.subjectClasses" :key="cls.id" class="sum">
                        {{ cls.name }} · <span class="num">{{ counts[cls.id] ?? 0 }}</span>명
                    </span>
                    <span class="set__hint">{{ app.subjectClasses.length ? '' : '맡은 교과 강좌가 없습니다' }}</span>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">명단에 올린 학생</span>
                <span class="set__value">
                    <b class="num">{{ totalStudents }}</b>명
                    <span class="set__hint">담임과 교과에 함께 있는 학생은 두 번 셉니다</span>
                </span>
            </div>
            <template #foot>
                <span>명렬표는 학기 중에도 다시 가져올 수 있습니다. 사라진 번호는 지우지 않고 전출로 남습니다.</span>
            </template>
        </UiLedger>

        <div class="wiz__acts">
            <UiButton :disabled="step === 1" @click="go(step - 1)">뒤로</UiButton>
            <UiButton :disabled="!canNext" variant="primary" @click="next">
                {{ step === STEPS.length ? '개요로 가기' : '다음' }}
            </UiButton>
        </div>
    </UiPage>
</template>

<style scoped>
/* 단계 표시를 눌러 오간다. 모양은 style.css의 `.step`이 정하고, 여기서는
 * 버튼의 기본 상자만 지운다 — 색과 글자 굵기를 다시 적으면 두 곳이 갈라진다. */
.steps .step {
    background: transparent;
    border: 0;
    padding: 0;
    text-align: left;
    cursor: pointer;
}

.steps .step:disabled {
    cursor: default;
}

.lead {
    grid-template-columns: 28px 1fr;
}

/* 이름 · 학교 이름표 · 인원 · 단추. 학교가 하나면 두 번째 칸은 비지만 자리는 지킨다 —
 * 칸을 없애면 학교를 더하는 순간 줄 전체가 밀린다. */
.mine .row {
    grid-template-columns: 160px 1fr auto auto;
}

.mine__panel {
    padding: var(--s-lg) var(--s-2xl);
    border-top: 1px solid var(--c-line-soft);
}

/* 순회 교사만 누르는 자리라 점선으로 눌러 둔다. */
.dashed {
    padding: var(--s-xs) var(--s-lg);
    border: 1px dashed var(--c-line);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--c-ink-3);
    cursor: pointer;
}

.dashed:hover:not(:disabled) {
    border-color: var(--c-accent);
    color: var(--c-accent);
}

.dashed:disabled {
    cursor: default;
}

/* 요약의 학급 이름표. 누르는 것이 아니므로 칩(`.chip`)을 쓰지 않는다 —
 * 칩은 손 모양 커서가 붙어 눌러야 하는 것처럼 읽힌다. */
.sum {
    padding: var(--s-2xs) var(--s-md);
    border: 1px solid var(--c-line);
    border-radius: var(--r-full);
    color: var(--c-ink-2);
}

/* 뒤로와 다음은 언제나 같은 자리에 있다. 단계마다 단추가 옮겨 다니면
 * 교사는 매번 눈으로 찾아야 한다. */
.wiz__acts {
    display: flex;
    justify-content: space-between;
    gap: var(--s-lg);
}
</style>
