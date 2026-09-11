<script setup>
/**
 * 첫 실행 — 다섯 단계로 학년도 · 학교 · 담당을 설정하고 명렬표를 넣는다.
 *
 * ```
 * 1 시작하기     여기서 하는 일을 세 줄로 알린다
 * 2 학년도       오늘 날짜로 채워져 있다. 직접 만들거나 지울 수도 있다
 * 3 학교         추가 · 마감 · 이름 · 최대 교시 · 서류 제출 기한
 * 4 학급과 강좌   담임 학급 · 교과 강좌를 더하고 각각 명렬표를 넣는다
 * 5 완료         추가한 것을 요약하고 개요로 보낸다
 * ```
 *
 * 이 화면은 **첫 실행에서만** 지나간다. 매일 열자마자 바로 입력할 수 있어야 하므로
 * 개요가 곧 기본 화면이고, 여기는 그 앞에 한 번 나타나는 자리다.
 *
 * **장부(`UiLedger`)를 쓰지 않는다.** 장부는 서른 행짜리 격자를 훑는 프리미티브라
 * 테두리 · 머리글 · 바닥글이 매 단계 반복되어 설정 화면과 똑같이 보인다. 온보딩은
 * 훑는 화면이 아니라 **한 번에 하나를 설정하는 화면**이라, 가운데로 모은 한 단짜리
 * 칸에 제목 · 한 줄 설명 · 입력만 둔다.
 *
 * **단계를 눌러 앞뒤로 오간다.** 앞 단계를 잠그면 학교 이름을 잘못 적은 교사가
 * 되돌아갈 길이 없어 앱을 껐다 켜게 된다. 잠그는 것은 하나뿐이다 — 담당 학급 · 강좌를
 * 하나도 추가하지 않으면 완료로 갈 수 없다. 그때도 **단추를 숨기지 않고 `disabled`로 둔다.**
 * 단추가 사라지면 화면이 움직이고, 교사는 자기가 무엇을 놓쳤는지 알 수 없다.
 *
 * **담임과 교과를 여기서는 함께 추가한다.** 화면이 분리되는 것은 추가를 마친 다음부터다 —
 * 담당 학급 · 강좌가 아직 하나도 없는 상태에서 모드를 먼저 선택하게 하면, 교사는 자기가
 * 무엇을 선택하는지 모르는 채 선택하게 된다.
 */
import {computed, nextTick, onMounted, ref, watch} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useRosterStore} from '../stores/roster'
import {useSchoolStore} from '../stores/school'
import RosterPanel from '../components/RosterPanel.vue'
import {UiButton, UiModal, UiNotice, UiTrashIcon} from '../components/ui'
import {MAX_SLOT_CHOICES} from '../data/slotChoices'
import {academicYearOf, yearSpanOf} from '../services/academicYear'

const app = useAppStore()
const roster = useRosterStore()
const school = useSchoolStore()
const router = useRouter()

/**
 * 단계 이름. 네 번째는 **담임 학급과 교과 강좌 둘**을 가리키는데, 단계 표시는 다섯 칸을
 * 한 줄에 나열하는 좁은 자리라 `담당 학급 · 강좌`를 그대로 적으면 줄이 넘친다.
 * 그래서 줄인 표현인 `학급과 강좌`를 쓴다 — **새 통칭을 지어내지 않는다.**
 */
const STEPS = ['시작하기', '학년도', '학교', '학급과 강좌', '완료']

/**
 * 단계마다 제목 아래에 붙는 한 줄.
 *
 * **아래 문장과 겹치지 않게 쓴다.** 한 화면에 같은 말이 두 번 나오면 교사는 두 번째를
 * 읽지 않고, 그러면 정작 거기에만 있는 말도 함께 놓친다.
 */
const LEADS = [
    '설정할 것은 세 가지입니다. 나중에 모두 수정할 수 있습니다.',
    '출결을 기록할 기준 연도입니다. 오늘 날짜로 채워 두었습니다.',
    '최대 교시와 서류 제출 기한은 학교마다 다릅니다.',
    '담임 학급과 교과 강좌를 추가합니다. 한쪽만 맡아도 됩니다.',
    '이대로 시작합니다.',
]

/**
 * 첫 단계에서 알리는 것. 세 줄을 넘기지 않는다 — 읽지 않는 안내는 없는 것과 같다.
 *
 * **번호를 붙이지 않는다.** 위 단계 표시가 이미 1~5를 쓰고 있어서, 여기에 1 · 2 · 3을
 * 달면 뜻이 다른 두 숫자가 한 화면에서 같은 모양으로 나온다. 이름표를 붙인다.
 */
const INTRO = [
    ['학년도와 학교', '출결을 기록할 기준 연도와 근무하는 학교를 설정합니다.'],
    ['학급과 강좌', '담임 학급과 교과 강좌를 추가합니다.'],
    ['명렬표', '학급과 강좌마다 학생 명단을 파일에서 읽어 등록합니다.'],
]

const step = ref(1)
const error = ref('')
const dueDays = ref(7)
const homeroomForm = ref({grade: 3, classNo: 1})
const subjectForm = ref({name: ''})
const newSchoolName = ref('')

/** 직접 만들 학년도. 비워 두면 아래 단추가 잠긴다. */
const newYear = ref('')

/** 마감하려는 학교. 확인을 한 번 거친다 — 되돌리기 어려운 것은 삭제뿐이다. */
const closingSchool = ref(null)

/**
 * 지우려는 학년도. **학교를 마감하는 것과 다른 동작이다** — 학교는 행이 남고 목록에서만
 * 사라지지만, 학년도는 행까지 지우고 `ON DELETE CASCADE`가 그 아래를 통째로 가져간다.
 * 그래서 확인 대화상자의 문구도 따로 쓴다.
 */
const deletingYear = ref(null)

/**
 * 학년도를 지우면 함께 사라지는 것. **묻기 전에 수로 보여준다** — 무엇이 사라지는지
 * 모르는 채 누르게 하면, 되돌릴 수 없는 쓰기를 짐작으로 승인하는 셈이 된다.
 * 다섯 수는 목록(`get_years`)에 이미 실려 오므로 대화상자를 여는 순간 기다리지 않는다.
 *
 * **담임 출결과 교과 차시를 한 수로 합치지 않는다.** 두 기록은 표부터 다르고,
 * 합친 수는 `출결 300건`이 어느 쪽인지 말해 주지 못한다.
 */
const GONE = [
    ['schoolCount', '학교', '개'],
    ['classCount', '담당 학급 · 강좌', '개'],
    ['studentCount', '학생', '명'],
    ['spanCount', '담임 출결', '건'],
    ['sessionCount', '교과 차시', '개'],
]

const yearCounts = computed(() =>
    GONE.map(([key, label, unit]) => ({label, unit, n: deletingYear.value?.[key] ?? 0})))

/**
 * 아직 아무것도 추가하지 않은 학년도인가.
 *
 * **0이라는 사실을 그대로 말한다.** 잘못 만든 학년도를 정리하는 일이 첫 실행에서
 * 가장 흔한데, 안전하다는 것을 알리지 않으면 교사는 무엇이 사라질지 몰라 그대로 둔다.
 */
const yearIsEmpty = computed(() => yearCounts.value.every((row) => row.n === 0))

/**
 * 마지막 학년도는 지울 수 없다. 커맨드도 거절하지만 화면이 먼저 잠근다 —
 * 누를 수 있는 단추가 언제나 거절로 끝나면 교사는 그것을 고장으로 읽는다.
 * **숨기지 않고 잠근다.** 사라지면 다른 학년도에서는 왜 보였는지 알 수 없다.
 */
const canDeleteYear = computed(() => app.years.length > 1)

/** 명렬표를 연 학급. 한 번에 하나만 연다 — 두 개가 열리면 어느 명단인지 흐려진다. */
const openId = ref(null)

/** 학급마다 지금 명단에 있는 인원. 넣고 나면 몇 명인지 보여야 한다. */
const counts = ref({})

/** 이번 학년도가 열리고 닫히는 날. 교사에게 되묻지 않고 오늘 날짜로 채운 값이다. */
const span = computed(() => yearSpanOf(app.currentYear?.year ?? academicYearOf(app.today)))

const yearLabel = computed(() => `${app.currentYear?.year ?? academicYearOf(app.today)}학년도`)

const hasClasses = computed(() => app.classes.length > 0)

/**
 * 명단에 올린 학생 수. **담임과 교과에 함께 있는 학생은 두 번 센다** —
 * 명단 단위의 합이라 그렇게 세는 것이 맞고, 화면에도 그렇게 적는다.
 */
const totalStudents = computed(() =>
    app.classes.reduce((sum, cls) => sum + (counts.value[cls.id] ?? 0), 0))

/** 완료로 갈 수 없는 단계. 담당 학급 · 강좌가 하나도 없으면 요약할 것도 없다. */
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
 * 학년도를 선택한다. **스토어 액션을 거친다** — 상태에 직접 대입하면 학교 목록도
 * 담당 학급 · 강좌 목록도 지난 학년도의 것으로 남는다. 학년도가 둘 이상이 되는 순간
 * 다음 단계가 다른 학년도의 학교를 보여주게 된다.
 */
async function pickYear(yearId) {
    if (yearId === app.yearId) return
    error.value = ''
    try {
        await app.selectYear(yearId)
    } catch (e) {
        error.value = String(e)
    }
}

/**
 * 학년도를 직접 만든다.
 *
 * **오늘 날짜로 채워 둔 값이 늘 맞지는 않는다** — 2월에 다음 학년도를 미리 준비하거나,
 * 지난해 기록을 옮겨 적는 교사가 있다. 선택만 되면 그 교사는 앱을 쓸 수 없다.
 * 시작일 · 종료일은 묻지 않는다. 학년도는 3월에 열린다는 것이 규칙이고,
 * `yearSpanOf`가 그 규칙으로 채운다.
 */
async function addYear() {
    const year = Number(newYear.value)
    if (!Number.isInteger(year) || year < 1900) {
        error.value = '학년도를 1900 이상의 연도로 적어주세요.'
        return
    }
    error.value = ''
    try {
        await app.createYear(year)
        newYear.value = ''
    } catch (e) {
        error.value = String(e)
    }
}

/**
 * 학년도를 지운다. **한 번 묻고 지운다** — 되돌릴 수 없는 쓰기는 삭제뿐이고,
 * 이것이 그 하나다.
 *
 * 지우고 나면 학교 설정과 인원을 다시 읽는다. 지금 보고 있던 학년도를 지웠으면
 * 스토어가 다른 학년도로 옮겨 놓으므로, 화면에 남은 값은 지워진 학년도의 것이다.
 */
async function confirmDeleteYear() {
    const target = deletingYear.value
    deletingYear.value = null
    if (!target) return
    error.value = ''
    try {
        // 마지막 학년도 거절 같은 판단은 커맨드가 한다. 여기서 문구를 다시 쓰면
        // 두 곳이 곧 어긋난다 — 실패는 그대로 받아 위 알림에 표시한다.
        await app.deleteYear(target.id)
        await school.fetchAll()
        dueDays.value = school.school?.dueDays ?? 7
        await refreshCounts()
    } catch (e) {
        error.value = String(e)
    }
}

/** 학교를 선택한다. 아래의 최대 교시 · 제출 기한과 다음 단계의 담당 학급 · 강좌가 그 학교의 것이 된다. */
async function pickSchool(schoolId) {
    if (schoolId === app.schoolId) return
    error.value = ''
    try {
        await app.selectSchool(schoolId)
        await school.fetchAll()
        dueDays.value = school.school?.dueDays ?? 7
    } catch (e) {
        error.value = String(e)
    }
}

/**
 * 학교를 마감한다. **"지웁니다"라고 말하지 않는다** — 지난 기록이 이 학교를
 * 가리키므로 행은 그대로 남고 목록에서만 사라진다. 잘못 만든 학교를 정리하는 길이
 * 없으면 오타 하나가 첫 화면에 영원히 남는다.
 */
async function confirmRetireSchool() {
    const target = closingSchool.value
    closingSchool.value = null
    if (!target) return
    error.value = ''
    try {
        await school.retireSchool(target.id)
        await school.fetchAll()
        dueDays.value = school.school?.dueDays ?? 7
    } catch (e) {
        error.value = String(e)
    }
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

/**
 * 학교를 만든다. **첫 실행에는 학교가 하나도 없다** — 시드가 만들지 않기 때문이다.
 * 학교는 학년도 안에 있고 해마다 달라지므로, 미리 하나 만들어 두면 그 학교가 어느
 * 학년도의 것인지 아무도 정한 적이 없는 상태가 된다.
 *
 * **반드시 이 길로 만든다.** 학교를 만들 때 기본 출결 태그와 한도 규정이 함께
 * 들어가므로, 행만 따로 만들면 그 학교는 빈 목록으로 시작한다.
 */
async function addSchool() {
    const name = newSchoolName.value.trim()
    if (!name) {
        error.value = '학교 이름을 적어주세요.'
        return
    }
    error.value = ''
    try {
        const id = await school.createSchool({name})
        newSchoolName.value = ''
        // 만든 학교로 옮겨야 아래의 최대 교시 · 제출 기한이 그 학교를 가리킨다.
        if (id != null) await app.selectSchool(id)
        await school.fetchAll()
        dueDays.value = school.school?.dueDays ?? 7
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

/** 만들고, 인원을 다시 세고, 그 학급의 명렬표를 바로 연다. 다음에 할 일이 그것이다. */
async function add(payload) {
    error.value = ''
    // 담당 학급 · 강좌는 학교에 소속된다. 학교 없이 만들면 커맨드가 거절하는데, 그 문구는
    // 앞 단계로 돌아가라는 말을 하지 않는다.
    if (app.schoolId == null) {
        error.value = '학교를 먼저 추가해주세요. 앞 단계에서 이름을 적으면 됩니다.'
        return null
    }
    try {
        // **교과 강좌를 더했다고 모드가 넘어가지 않는다.** `selectClass`가 모드를
        // `app_config`에 저장하므로, 교과를 마지막으로 더한 교사는 다음 실행이
        // 교과 모드로 열린다. 다만 **아직 아무것도 선택하지 않았으면 선택한다** —
        // 교과만 맡은 비담임 교사에게는 그 강좌가 유일한 자리다.
        const select = payload.role === 'homeroom' || app.classId == null
        const id = await app.createClass({...payload, select})
        await refreshCounts()
        openId.value = id
        return id
    } catch (e) {
        error.value = String(e)
        return null
    }
}

/**
 * 명렬표를 연다. **담임 학급이면 그 학급을 함께 선택한다** — 명단이 붙는 곳은 지금
 * 선택한 학급이라, 열린 줄과 선택한 학급이 어긋나면 다른 반 명단에 들어간다.
 *
 * **교과 강좌에서는 선택하지 않는다.** `selectClass`는 모드를 `app_config`에 저장하므로,
 * 온보딩 끝에 교과 명렬표를 마지막으로 만진 교사는 다음 실행이 교과 모드로 열린다.
 * 이 화면은 `meta.bare`라 그 전환이 눈에 보이지도 않는다. 명단이 붙는 곳은
 * `RosterPanel`에 넘기는 `classId`가 이미 결정하므로 선택하지 않아도 어긋나지 않는다.
 */
async function toggleRoster(cls) {
    if (openId.value === cls.id) {
        openId.value = null
        return
    }
    error.value = ''
    try {
        if (cls.role === 'homeroom') await app.selectClass(cls.id)
        openId.value = cls.id
    } catch (e) {
        error.value = String(e)
    }
}

/**
 * 세는 동안에는 아래 워처를 멈춘다.
 *
 * 스토어의 명단은 하나뿐이라, 학급을 돌며 읽는 사이에 워처가 끼어들어 **지금 읽은
 * 다른 학급의 인원**을 열어 둔 학급 칸에 적는다. 30명을 막 넣은 줄이 `0명`으로
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
    <div class="wiz">
        <header class="wiz__head">
            <h1 class="wiz__title">출결관리를 시작합니다</h1>
            <p class="wiz__sub">설정은 처음 한 번뿐입니다. 끝내면 바로 출결을 입력할 수 있습니다</p>
        </header>

        <!-- 단계 표시. 눌러서 앞뒤로 오간다 — 잘못 적은 교사가 되돌아갈 길이다. -->
        <nav class="steps">
            <button v-for="(name, i) in STEPS" :key="name"
                    :class="['step', step === i + 1 ? 'is-now' : step > i + 1 ? 'is-done' : '']"
                    :disabled="blocked(i + 1)" type="button" @click="go(i + 1)">
                <span class="step__no num">{{ step > i + 1 ? '✓' : i + 1 }}</span>
                <span class="step__name">{{ name }}</span>
            </button>
        </nav>

        <UiNotice :text="error" kind="error"/>
        <UiNotice :text="app.error" kind="error"/>
        <UiNotice :text="school.error" kind="error"/>
        <UiNotice :text="roster.error" kind="error"/>

        <section class="pane">
            <h2 class="pane__title">{{ STEPS[step - 1] }}</h2>
            <p class="pane__lead">{{ LEADS[step - 1] }}</p>

            <!-- 1 시작하기 ─ 여기서 하는 일 -->
            <div v-if="step === 1" class="pane__body">
                <dl class="intro">
                    <template v-for="[name, line] in INTRO" :key="name">
                        <dt class="intro__name">{{ name }}</dt>
                        <dd class="intro__line">{{ line }}</dd>
                    </template>
                </dl>
            </div>

            <!-- 2 학년도 ─ 오늘 날짜로 채워 두고, 직접 만들 수도 있다 -->
            <div v-if="step === 2" class="pane__body">
                <div class="field__group">
                    <span class="field__label">이번 학년도</span>
                    <div class="chips">
                        <!-- 학교와 같은 모양으로 선택과 지우기를 한 덩어리로 둔다.
                             휴지통이 칩 밖에 있으면 어느 학년도를 지우는지 눈으로 연결해야 한다. -->
                        <span v-for="year in app.years" :key="year.id" class="chip">
                            <button :class="['pick', 'pick--big', app.yearId === year.id ? 'is-on' : '']"
                                    type="button" @click="pickYear(year.id)">
                                {{ year.year }}학년도
                            </button>
                            <button :disabled="!canDeleteYear" class="drop"
                                    :title="canDeleteYear
                                        ? '이 학년도와 그 안의 모든 기록을 지웁니다'
                                        : '마지막 학년도는 지울 수 없습니다'"
                                    type="button" @click="deletingYear = year">
                                <UiTrashIcon/>
                            </button>
                        </span>
                    </div>
                    <p class="field__hint">
                        3월에 열려 이듬해 2월에 닫힙니다 —
                        <span class="num">{{ span.startsOn }}</span> ~
                        <span class="num">{{ span.endsOn }}</span>.
                        1 · 2월은 지난 학년도에 속합니다
                    </p>
                </div>

                <div class="field__group">
                    <span class="field__label">직접 추가</span>
                    <div class="chips">
                        <input v-model="newYear" class="field num" max="2200" min="1900"
                               placeholder="2027" type="number" @keyup.enter="addYear"/>
                        <UiButton :disabled="!String(newYear).trim()" @click="addYear">
                            학년도 추가
                        </UiButton>
                    </div>
                    <p class="field__hint">
                        다음 학년도를 미리 준비하거나 지난 기록을 정리할 때 씁니다.
                        시작일과 종료일은 위 규칙으로 채웁니다
                    </p>
                </div>

                <p class="pane__note">
                    학교는 학년도에 속합니다. 다음 단계에서 만드는 학교가 이 학년도로 들어갑니다.
                    학년도를 지우면 그 안의 학교 · 담당 학급 · 강좌 · 출결이 모두 함께 사라지고,
                    마지막 하나는 지울 수 없습니다.
                </p>
            </div>

            <!-- 3 학교 ─ 추가 · 마감 · 이름 · 최대 교시 · 서류 제출 기한 -->
            <div v-if="step === 3" class="pane__body">
                <div class="field__group">
                    <span class="field__label">{{ yearLabel }}의 학교</span>
                    <div class="chips">
                        <!-- 선택과 마감을 한 덩어리로 둔다. 휴지통이 칩 밖에 있으면
                             어느 학교를 마감하는지 눈으로 연결해야 한다. -->
                        <span v-for="s in app.schools" :key="s.id" class="chip">
                            <button :class="['pick', 'pick--big', app.schoolId === s.id ? 'is-on' : '']"
                                    type="button" @click="pickSchool(s.id)">
                                {{ s.name }}
                            </button>
                            <button class="drop" title="이 학교를 마감합니다" type="button"
                                    @click="closingSchool = s">
                                <UiTrashIcon/>
                            </button>
                        </span>
                    </div>
                    <div class="chips">
                        <input v-model="newSchoolName" class="field" placeholder="한빛고등학교"
                               type="text" @keyup.enter="addSchool"/>
                        <UiButton variant="primary" @click="addSchool">학교 추가</UiButton>
                    </div>
                    <p class="field__hint">
                        순회 교사는 둘 이상을 추가합니다. 선택한 학교에 아래 설정과 담당 학급 · 강좌가 속합니다
                    </p>
                </div>

                <!-- 첫 실행의 정상 상태다. 빈 칸만 나열하면 무엇을 고치는지 알 수 없다. -->
                <p v-if="!school.school" class="pane__empty">
                    <b>아직 없습니다.</b>
                    위에 이름을 적어 학교를 추가하면 최대 교시와 제출 기한을 설정할 수 있습니다.
                </p>

                <template v-else>
                    <div class="field__group">
                        <span class="field__label">학교 이름</span>
                        <input :value="school.school?.name ?? ''" class="field field--wide"
                               placeholder="한빛고등학교" type="text"
                               @change="saveName($event.target.value)"/>
                    </div>
                    <div class="field__group">
                        <span class="field__label">최대 교시</span>
                        <div class="chips">
                            <button v-for="n in MAX_SLOT_CHOICES" :key="n"
                                    :class="['pick', 'pick--slot', school.school?.maxSlot === n ? 'is-on' : '']"
                                    type="button"
                                    @click="school.saveSchool({maxSlot: n}).catch(() => {})">
                                {{ n }}
                            </button>
                        </div>
                        <p class="field__hint">조회와 종례는 언제나 하루의 양 끝입니다</p>
                    </div>
                    <div class="field__group">
                        <span class="field__label">서류 제출 기한</span>
                        <div class="chips">
                            <input v-model="dueDays" class="field num" min="0" type="number"
                                   @change="saveDueDays"/>
                            <span class="field__unit">일</span>
                        </div>
                        <p class="field__hint">
                            결석일부터 셉니다. 미제출자 목록의 기한일이 이 값으로 계산됩니다
                        </p>
                    </div>
                </template>
            </div>

            <!-- 4 학급과 강좌 ─ 담임 학급 · 교과 강좌 -->
            <div v-if="step === 4" class="pane__body">
                <div class="mine mine--homeroom">
                    <h3 class="pane__sub">담임 학급</h3>
                    <template v-for="cls in app.homeroomClasses" :key="cls.id">
                        <div class="mine__row">
                            <b class="mine__name">{{ cls.name }}</b>
                            <span class="mine__where">{{ schoolLabel(cls) }}</span>
                            <span class="mine__count">{{ rosterLabel(cls.id) }}</span>
                            <UiButton size="tight" @click="toggleRoster(cls)">
                                {{ openId === cls.id ? '명렬표 닫기' : '명렬표 열기' }}
                            </UiButton>
                        </div>
                        <div v-if="openId === cls.id" class="mine__panel">
                            <RosterPanel :class-id="cls.id"/>
                        </div>
                    </template>

                    <div class="chips">
                        <input v-model="homeroomForm.grade" class="field num" min="1" type="number"/>
                        <span class="field__unit">학년</span>
                        <input v-model="homeroomForm.classNo" class="field num" min="1" type="number"/>
                        <span class="field__unit">반</span>
                        <UiButton variant="primary" @click="addHomeroom">추가</UiButton>
                    </div>
                    <p class="field__hint">이름은 3학년 6반처럼 자동으로 짓습니다</p>
                </div>

                <div class="mine mine--subject">
                    <h3 class="pane__sub">교과 강좌</h3>
                    <template v-for="cls in app.subjectClasses" :key="cls.id">
                        <div class="mine__row">
                            <b class="mine__name">{{ cls.name }}</b>
                            <span class="mine__where">{{ schoolLabel(cls) }}</span>
                            <span class="mine__count">{{ rosterLabel(cls.id) }}</span>
                            <UiButton size="tight" @click="toggleRoster(cls)">
                                {{ openId === cls.id ? '명렬표 닫기' : '명렬표 열기' }}
                            </UiButton>
                        </div>
                        <div v-if="openId === cls.id" class="mine__panel">
                            <RosterPanel :class-id="cls.id"/>
                        </div>
                    </template>

                    <div class="chips">
                        <input v-model="subjectForm.name" class="field field--wide"
                               placeholder="인공지능기초A" type="text" @keyup.enter="addSubject"/>
                        <UiButton variant="primary" @click="addSubject">추가</UiButton>
                    </div>
                    <p class="field__hint">
                        반이 섞입니다. 선택과목은 여러 반에서 모이므로 학년 · 반을 묻지 않고,
                        명렬표 파일의 학년 · 반 열로 학생을 구별합니다
                    </p>
                </div>

                <p class="pane__note">하나도 추가하지 않으면 완료로 넘어갈 수 없습니다.</p>
            </div>

            <!-- 5 완료 ─ 추가한 것을 요약한다 -->
            <div v-if="step === 5" class="pane__body">
                <h3 class="pane__sub">추가한 것</h3>
                <dl class="sum">
                    <dt class="sum__key">학년도</dt>
                    <dd class="sum__val">
                        <span class="num">{{ app.currentYear?.year ?? academicYearOf(app.today) }}</span>학년도
                        <span class="sum__hint">
                            <span class="num">{{ span.startsOn }}</span> ~
                            <span class="num">{{ span.endsOn }}</span>
                        </span>
                    </dd>

                    <dt class="sum__key">학교</dt>
                    <dd class="sum__val">
                        {{ school.school?.name ?? app.school?.name ?? '이름 없음' }}
                        <span class="sum__hint">
                            최대 <span class="num">{{ app.maxSlot }}</span>교시 ·
                            서류 제출 기한 <span class="num">{{ school.school?.dueDays ?? dueDays }}</span>일
                        </span>
                    </dd>

                    <dt class="sum__key">담임 학급</dt>
                    <dd class="sum__val">
                        <span v-for="cls in app.homeroomClasses" :key="cls.id" class="tagish">
                            {{ cls.name }} · <span class="num">{{ counts[cls.id] ?? 0 }}</span>명
                        </span>
                        <span v-if="!app.homeroomClasses.length" class="sum__hint">
                            추가한 담임 학급이 없습니다
                        </span>
                    </dd>

                    <dt class="sum__key">교과 강좌</dt>
                    <dd class="sum__val">
                        <span v-for="cls in app.subjectClasses" :key="cls.id" class="tagish">
                            {{ cls.name }} · <span class="num">{{ counts[cls.id] ?? 0 }}</span>명
                        </span>
                        <span v-if="!app.subjectClasses.length" class="sum__hint">
                            추가한 교과 강좌가 없습니다
                        </span>
                    </dd>

                    <dt class="sum__key">명단에 등록한 학생</dt>
                    <dd class="sum__val">
                        <b class="num">{{ totalStudents }}</b>명
                        <span class="sum__hint">담임과 교과에 함께 있는 학생은 두 번 셉니다</span>
                    </dd>
                </dl>
                <p class="pane__note">
                    명렬표는 학기 중에도 다시 열 수 있습니다. 명단에서 제외한 번호는 지우지 않고
                    나간 날짜만 기록하므로, 지난 출결은 그대로 남습니다.
                </p>
            </div>
        </section>

        <!-- 뒤로와 다음은 언제나 같은 자리에 있다. 단계마다 단추가 옮겨 다니면
             교사는 매번 눈으로 찾아야 한다. -->
        <div class="wiz__acts">
            <UiButton :disabled="step === 1" size="wide" @click="go(step - 1)">뒤로</UiButton>
            <UiButton :disabled="!canNext" fill size="wide" variant="primary" @click="next">
                {{ step === STEPS.length ? '개요로 가기' : '다음' }}
            </UiButton>
        </div>

        <UiModal :open="Boolean(closingSchool)" title="이 학교를 마감합니다"
                 @close="closingSchool = null">
            <div v-if="closingSchool" class="modal__what">
                <span class="modal__key">학교</span>
                <span class="modal__val">{{ closingSchool.name }}</span>
                <span class="modal__key">학년도</span>
                <span class="modal__val">{{ yearLabel }}</span>
            </div>
            <p class="modal__note">
                지우지 않고 마감만 합니다. 이 학교의 담당 학급 · 강좌와 지난 출결은
                그대로 남습니다 — 다시 필요하면 설정에서 같은 이름으로 만들면 됩니다.
            </p>

            <template #foot>
                <UiButton size="wide" @click="closingSchool = null">취소</UiButton>
                <UiButton fill size="wide" variant="danger" @click="confirmRetireSchool">
                    마감
                </UiButton>
            </template>
        </UiModal>

        <!-- 학교를 마감하는 대화상자와 **다른 말을 쓴다.** 학교는 행이 남지만 학년도는
             행까지 지우고, 그 안의 기록이 전부 함께 사라진다. 같은 문장을 돌려 쓰면
             교사는 학교와 같은 정도의 일로 읽는다. -->
        <UiModal :open="Boolean(deletingYear)"
                 :subtitle="deletingYear ? `${deletingYear.year}학년도` : ''"
                 title="이 학년도를 지웁니다" @close="deletingYear = null">
            <dl class="gone">
                <template v-for="row in yearCounts" :key="row.label">
                    <dt class="gone__what">{{ row.label }}</dt>
                    <dd class="gone__n"><b class="num">{{ row.n }}</b>{{ row.unit }}</dd>
                </template>
            </dl>
            <p v-if="yearIsEmpty" class="modal__note">
                <b>아직 아무것도 추가하지 않은 학년도입니다.</b>
                지워도 사라지는 기록이 없습니다.
            </p>
            <p class="modal__note">
                위의 것이 <b>모두 함께 사라집니다.</b> 학교를 마감하는 것과 달리
                학년도는 기록까지 지우며, 되돌릴 수 없습니다. 서류 · NEIS 표시와 태그 ·
                한도 규정도 이 학년도의 것은 함께 사라집니다.
            </p>

            <template #foot>
                <UiButton size="wide" @click="deletingYear = null">취소</UiButton>
                <UiButton fill size="wide" variant="danger" @click="confirmDeleteYear">
                    지우기
                </UiButton>
            </template>
        </UiModal>
    </div>
</template>

<style scoped>
/* 가운데로 모은 한 단. 온보딩은 훑는 화면이 아니라 한 번에 하나를 설정하는 화면이라,
 * 화면 폭을 다 쓰면 눈이 좌우로 오간다. */
.wiz {
    display: flex;
    flex-direction: column;
    gap: var(--s-3xl);
    width: 100%;
    max-width: 720px;
    margin: 0 auto;
    padding: var(--s-5xl) var(--s-3xl);
}

.wiz__head {
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
}

.wiz__title {
    margin: 0;
    font-size: var(--t-3xl);
    font-weight: 800;
    letter-spacing: -.02em;
    color: var(--c-ink);
}

.wiz__sub {
    margin: 0;
    color: var(--c-ink-3);
}

/* 단계 표시. 모양은 style.css의 `.step`이 결정하고(NEIS 검증과 같은 것을 쓴다),
 * 여기서는 버튼의 기본 상자를 지우고 일정하게 벌려 놓는다. */
.steps .step {
    flex: 1;
    background: transparent;
    border: 0;
    padding: 0;
    text-align: left;
    cursor: pointer;
}

.steps .step:disabled {
    cursor: default;
    opacity: .55;
}

.step__name {
    white-space: nowrap;
}

/* 단계 한 칸. 테두리를 한 겹만 두른다 — 장부처럼 머리글과 바닥글을 겹쳐 두면
 * 설정 화면과 구별되지 않는다. */
.pane {
    display: flex;
    flex-direction: column;
    gap: var(--s-sm);
    padding: var(--s-4xl);
    border: 1px solid var(--c-line);
    border-radius: var(--r-xl);
    background: var(--c-surface);
    box-shadow: var(--c-shadow);
}

.pane__title {
    margin: 0;
    font-size: var(--t-2xl);
    font-weight: 700;
    color: var(--c-ink);
}

.pane__lead {
    margin: 0;
    color: var(--c-ink-3);
}

.pane__body {
    display: flex;
    flex-direction: column;
    gap: var(--s-4xl);
    margin-top: var(--s-3xl);
}

.pane__sub {
    margin: 0;
    font-size: var(--t-lg);
    font-weight: 700;
    color: var(--c-ink);
}

.pane__note {
    margin: 0;
    padding-top: var(--s-lg);
    border-top: 1px solid var(--c-line-soft);
    color: var(--c-ink-3);
}

.pane__empty {
    margin: 0;
    padding: var(--s-3xl);
    border: 1px dashed var(--c-line);
    border-radius: var(--r-lg);
    color: var(--c-ink-3);
    text-align: center;
}

/* 라벨 · 입력 · 설명 한 덩어리. 설정 화면의 라벨/값 두 칸과 달리 세로로 쌓는다 —
 * 온보딩은 한 번에 하나만 설정하므로 좌우로 나눌 이유가 없다. */
.field__group {
    display: flex;
    flex-direction: column;
    gap: var(--s-md);
}

.field__label {
    font-weight: 700;
    color: var(--c-ink);
}

.field__hint {
    margin: 0;
    color: var(--c-ink-3);
}

.field__unit {
    color: var(--c-ink-3);
}

.field--wide {
    width: 100%;
}

.chips {
    display: flex;
    align-items: center;
    gap: var(--s-sm);
    flex-wrap: wrap;
}

/* 온보딩의 선택 단추는 매일 쓰는 격자보다 크다. 처음 한 번 누르는 단추라
 * 작게 두면 무엇을 눌러야 하는지 찾게 된다. */
.pick--big {
    padding: var(--s-md) var(--s-3xl);
}

/* 선택과 마감을 한 덩어리로 묶는다. */
.chip {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2xs);
}

/* 지우기는 글자가 아니라 휴지통 모양 + 경고색이다. 다른 단추와 섞이면 안 된다. */
.drop {
    display: grid;
    place-items: center;
    padding: var(--s-xs);
    border: 1px solid transparent;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--c-ink-3);
    cursor: pointer;
}

.drop:hover:not(:disabled) {
    border-color: var(--c-danger);
    background: var(--c-danger-bg);
    color: var(--c-danger);
}

/* 마지막 학년도의 휴지통. **숨기지 않고 잠근다** — 사라지면 다른 줄에서는 왜 보였는지
 * 알 수 없다. 잠긴 것은 `.pick:disabled`와 같은 방식으로 흐리게만 표시한다. */
.drop:disabled {
    opacity: .38;
    cursor: default;
}

/* 이름표와 설명 한 쌍. 번호를 쓰지 않는 이유는 위 단계 표시가 이미 숫자를
 * 쓰고 있어서다 — 뜻이 다른 두 숫자가 같은 모양으로 나오면 서로를 가리킨다고 읽힌다. */
.intro {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: var(--s-xl) var(--s-2xl);
    margin: 0;
}

.intro__name {
    font-weight: 700;
    color: var(--c-accent);
}

.intro__line {
    margin: 0;
    color: var(--c-ink-2);
}

/* 담당 학급 · 강좌 한 줄. 이름 · 학교 이름표 · 인원 · 단추. 학교가 하나면 두 번째 칸은
 * 비지만 자리는 지킨다 — 칸을 없애면 학교를 더하는 순간 줄 전체가 밀린다. */
.mine {
    display: flex;
    flex-direction: column;
    gap: var(--s-md);
}

.mine__row {
    display: grid;
    grid-template-columns: minmax(120px, 1fr) auto 1fr auto;
    gap: var(--s-lg);
    align-items: center;
    padding: var(--s-lg) var(--s-2xl);
    border: 1px solid var(--c-line);
    border-radius: var(--r-lg);
    background: var(--c-raised);
}

.mine__where,
.mine__count {
    color: var(--c-ink-3);
}

.mine__panel {
    padding: var(--s-lg) var(--s-2xl);
    border: 1px solid var(--c-line);
    border-top: 0;
    border-radius: 0 0 var(--r-lg) var(--r-lg);
}

/* 요약. 라벨과 값을 두 칸으로 배치하되 장부 테두리를 두르지 않는다. */
.sum {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: var(--s-lg) var(--s-2xl);
    margin: 0;
}

.sum__key {
    color: var(--c-ink-3);
}

.sum__val {
    display: flex;
    align-items: center;
    gap: var(--s-sm);
    flex-wrap: wrap;
    margin: 0;
    color: var(--c-ink);
}

.sum__hint {
    color: var(--c-ink-3);
}

/* 요약의 학급 이름표. 누르는 것이 아니므로 칩을 쓰지 않는다 —
 * 칩은 손 모양 커서가 붙어 눌러야 하는 것처럼 읽힌다. */
.tagish {
    padding: var(--s-2xs) var(--s-md);
    border: 1px solid var(--c-line);
    border-radius: var(--r-full);
    color: var(--c-ink-2);
}

/* **화면 아래에 고정한다.** 단계마다 내용 길이가 달라서 문서 흐름에 두면 [다음]이
 * 접힌 자리 아래로 밀린다 — 담당 학급 · 강좌를 여럿 추가하거나 명렬표를 열면 한참 스크롤해야
 * 단추가 나온다. 다음으로 넘어가는 것은 이 화면에서 늘 할 수 있어야 하는 일이다.
 *
 * `sticky`가 아니라 `fixed`인 이유는, 이 화면이 `meta.bare`라 스크롤 상자가 문서
 * 자신이기 때문이다. `sticky`는 넘치지 않는 단계에서 제자리에 머물러 화면 가운데
 * 떠 있게 된다 — 단추가 단계마다 움직이면 교사는 매번 눈으로 찾는다. */
.wiz__acts {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-lg);
    min-height: var(--acts-h);
    padding: var(--s-lg) var(--s-3xl);
    border-top: 1px solid var(--c-line);
    background: var(--c-surface);
}

/* 고정한 띠가 마지막 내용을 가리지 않게 **그 높이만큼** 비워 둔다.
 * 띠에 `min-height`를 함께 걸어 두 값이 어긋나지 않게 한다 — 여백만 짐작으로 두면
 * 단추 크기가 바뀌는 날 마지막 줄이 다시 가려지고, 그것은 화면에서 보이지 않는다. */
.wiz {
    --acts-h: 72px;
    padding-bottom: calc(var(--acts-h) + var(--s-3xl));
}

/* 학년도를 지울 때 함께 사라지는 것. `.modal__what`을 쓰지 않는 이유는 그쪽 라벨 칸이
 * 76px이라 `담당 학급 · 강좌`가 두 줄로 접히기 때문이다. 왼쪽의 경고색 띠는 같다. */
.gone {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--s-md) var(--s-2xl);
    margin: 0;
    padding: var(--s-lg) var(--s-2xl);
    border: 1px solid var(--c-line);
    border-left: 3px solid var(--c-danger);
    border-radius: var(--r-lg);
}

.gone__what {
    color: var(--c-ink-3);
}

.gone__n {
    margin: 0;
    color: var(--c-ink);
}
</style>
