<script setup>
/**
 * 설정 — 학년도 · 학교 · 한도 규정 · 맡은 것 · 명렬표 · 화면.
 *
 * **계층이 그대로 화면의 차례다 — 학년도 → 학교 → 맡은 것.** 학년도를 고르면 그
 * 학년도의 학교가 나오고, 학교를 고르면 그 학교의 맡은 것이 나온다. 위에서 고른 것이
 * 아래의 범위다.
 *
 * **학년도를 바꾸는 길은 여기뿐이다.** 첫 실행 화면은 한 번 지나가면 닿을 수 없어,
 * 거기에만 두면 2027년 3월이 와도 새 학년도로 넘어갈 방법이 없다.
 *
 * **새 학년도에는 학교가 하나도 없다.** 학교는 학년도 안에 있고, 해가 바뀌면 출근하는
 * 학교가 달라진다(순회 교사). 그래서 비어 있는 것이 정상이고, 그 자리에서 바로 만든다.
 *
 * **학교 단위 값이 여기 모인다.** 최대 교시와 제출 기한은 앱 상수가 아니라 학교가
 * 들고 있는 값이다.
 *
 * **맡은 것을 더하고 마감하는 곳도 여기다.** 담임 학급과 교과 강좌가 한 목록에 나란히
 * 표시되는 화면은 여기뿐이다 — 기록 화면은 모드로 완전히 분리되지만, 무엇을 맡았는지 정하는
 * 일은 두 갈래를 함께 봐야 한다. 여기에 나오는 것은 맡은 것의 목록일 뿐 한쪽의 기록이
 * 다른 쪽에 새는 것이 아니다. **학급을 옮기는 길은 여기 두지 않는다** — 그것은 이동
 * 화면과 모드 스위치가 맡는다.
 *
 * 명렬표 가져오기도 여기 있다 — 학기에 한두 번 쓰는 일이라 사이드바에 둘 이유가 없다.
 */
import {computed, onMounted, ref, watch} from 'vue'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {useSchoolStore} from '../stores/school'
import RosterPanel from '../components/RosterPanel.vue'
import {UiButton, UiLedger, UiModal, UiNotice, UiPage, UiToggle, UiTrashIcon} from '../components/ui'
import {MAX_SLOT_CHOICES} from '../data/slotChoices'
import {academicYearOf, yearSpanOf} from '../services/academicYear'
import {useTheme} from '../composables/useTheme'

const app = useAppStore()
const axis = useAxisStore()
const school = useSchoolStore()
const theme = useTheme()

const THEME_CHOICES = [
    {value: 'light', label: '라이트'},
    {value: 'dark', label: '다크'},
    {value: 'system', label: '시스템 따름'},
]
/**
 * 한도를 세는 기간. **`year` · `month` 둘뿐이다.**
 *
 * 학기 단위를 두지 않는다 — 1 · 2학기 경계가 학교마다 달라 앱이 9월 1일로 단정하면
 * 그 학교의 한도가 조용히 틀린 값이 된다. Rust도 이 둘만 받으므로 여기에 셋째를
 * 두면 누를 수는 있는데 저장이 반드시 실패하는 단추가 생긴다.
 */
const PERIODS = [
    {value: 'year', label: '학년도'},
    {value: 'month', label: '달'},
]
const UNITS = [
    {value: 'day', label: '일'},
    {value: 'count', label: '회'},
]

const dueDays = ref(7)
const newOffDay = ref({date: '', label: ''})
const newTag = ref('')
const newRule = ref(blankRule())
const newYear = ref(nextYear())
const newHomeroom = ref({grade: 3, classNo: 1})
const newSubject = ref({name: '', groupTagId: null})
const newSchoolName = ref('')
const newClassTag = ref('')
const message = ref('')

/** 지우려고 고른 휴업일. 태그 · 규정의 [마감]과 달리 이것은 행을 지우는 DELETE다. */
const droppingOffDay = ref(null)

/** 지우려고 고른 강좌 묶음. 이것도 행이 사라지는 DELETE다. */
const droppingClassTag = ref(null)

/** 마감하려고 고른 맡은 것. 지우는 것이 아니라 목록에서 내리는 것이다. */
const closingClass = ref(null)

/** 마감하려고 고른 학교. 학교도 지우지 않고 목록에서 내린다. */
const closingSchool = ref(null)

/** 이 학년도에 등록한 학교가 없는 상태. 해가 바뀐 직후의 정상 상태다. */
const noSchool = computed(() => app.schools.length === 0)

const yearLabel = computed(() => `${app.currentYear?.year ?? academicYearOf(app.today)}학년도`)

/** 새로 만들 학년도가 열리고 닫히는 날. 3월에 열린다. */
const newYearSpan = computed(() =>
    yearSpanOf(Number(newYear.value) || academicYearOf(app.today)))

/**
 * 교과 강좌를 묶음별로 모은다. **분반 표가 아니라 이름표다** — 강좌는 저마다 독립된
 * 행이고, 묶음은 `프로그래밍A · B · C`를 화면에서 함께 보이게 하는 것뿐이다.
 *
 * 묶음이 없는 강좌는 맨 뒤에 따로 모은다. 묶음 목록에 없는 이름을 가리키는 강좌도
 * 그리로 간다 — **어느 쪽에도 들지 못해 목록에서 사라지는 강좌가 없어야 한다.**
 */
const subjectGroups = computed(() => {
    const classes = app.subjectClasses
    const groups = school.classTags
        .map((tag) => ({
            key: `tag-${tag.id}`,
            name: tag.name,
            items: classes.filter((cls) => cls.groupTagId === tag.id),
        }))
        .filter((group) => group.items.length > 0)

    const grouped = new Set(groups.flatMap((group) => group.items.map((cls) => cls.id)))
    const loose = classes.filter((cls) => !grouped.has(cls.id))
    return loose.length ? [...groups, {key: 'none', name: '묶음 없음', items: loose}] : groups
})

/**
 * 명렬표를 넣을 학급. 고르기 전에는 지금 보고 있는 학급이다.
 *
 * 줄마다 명렬표 상자를 열지 않고 **고르개만 둔다** — 상자가 생겼다 사라지면 아래
 * 단추들의 자리가 그때마다 달라진다. 자리는 그대로 두고 무엇을 가리키는지만 바꾼다.
 */
const rosterClassId = ref(null)
const rosterTarget = computed(() => rosterClassId.value ?? app.classId)
const rosterClass = computed(
    () => app.classes.find((c) => c.id === rosterTarget.value) ?? null,
)

const skipOffDays = computed({
    get: () => Boolean(school.school?.dueSkipOffdays),
    set: (value) => school.saveSchool({dueSkipOffdays: value}),
})

function blankRule() {
    return {name: '', tagId: null, period: 'year', limitN: 20, unit: 'day'}
}

/** 새 학년도 칸에 미리 채워 둘 해. 지금 학년도의 다음 해다. */
function nextYear() {
    return (app.currentYear?.year ?? academicYearOf(app.today)) + 1
}

/**
 * 학년도를 옮긴다. **학교도 맡은 것도 그 학년도의 것으로 통째로 바뀐다** —
 * 지난해 줄은 지난해 학교에 그대로 남고 사라지지 않는다.
 */
async function pickYear(yearId) {
    if (yearId === app.yearId) return
    message.value = ''
    await app.selectYear(yearId).catch(() => {
    })
}

/**
 * 학년도를 만든다. 만든 학년도로 옮겨 간다 — 만들어 두고 쓰지 않을 이유가 없다.
 *
 * 실패를 문구로 남긴다. 여는 날 · 닫는 날은 화면이 묻지 않고 스토어가 3월로 계산한다.
 */
async function addYear() {
    const year = Number(newYear.value)
    if (!Number.isInteger(year) || year < 1900) {
        message.value = '학년도를 네 자리 수로 적어주세요.'
        return
    }
    message.value = ''
    try {
        await app.createYear(year)
        newYear.value = year + 1
    } catch (e) {
        message.value = String(e)
    }
}

/** 학교를 옮긴다. 아래의 설정 · 맡은 것 · 명렬표가 전부 그 학교의 것이 된다. */
async function pickSchool(schoolId) {
    if (schoolId === app.schoolId) return
    message.value = ''
    await app.selectSchool(schoolId).catch(() => {
    })
}

/**
 * 학교를 더한다. **반드시 이 길로 만든다** — 학교를 만들 때 기본 출결 태그와
 * 한도 규정이 함께 들어가므로, 행만 따로 만들면 그 학교는 빈 목록으로 시작한다.
 */
async function addSchool() {
    const name = newSchoolName.value.trim()
    if (!name) {
        message.value = '학교 이름을 적어주세요.'
        return
    }
    message.value = ''
    const id = await school.createSchool({name}).catch(() => null)
    newSchoolName.value = ''
    // 만든 학교로 옮겨야 아래의 최대 교시 · 제출 기한이 그 학교를 가리킨다.
    if (id != null) await app.selectSchool(id).catch(() => {
    })
}

/**
 * 학교를 마감한다. **확인을 한 번 거치되 "지웁니다"라고 말하지 않는다** —
 * 지난 기록이 이 학교를 가리키므로 행은 그대로 남고 목록에서만 내려간다.
 */
async function confirmRetireSchool() {
    const target = closingSchool.value
    closingSchool.value = null
    if (target) {
        await school.retireSchool(target.id).catch(() => {
        })
    }
}

async function saveDueDays() {
    const days = Number(dueDays.value)
    if (!Number.isInteger(days) || days < 0) {
        message.value = '제출 기한은 0 이상의 날 수로 적어주세요.'
        return
    }
    message.value = ''
    await school.saveSchool({dueDays: days}).catch(() => {
    })
}

async function addOffDay() {
    if (!newOffDay.value.date) return
    await school.addOffDay(newOffDay.value.date, newOffDay.value.label || null).catch(() => {
    })
    newOffDay.value = {date: '', label: ''}
}

/**
 * 휴업일 삭제. **확인을 한 번 거친다** — 태그와 규정은 마감이라 되살릴 수 있지만
 * 이것은 행이 사라지는 DELETE다.
 */
async function confirmRemoveOffDay() {
    const target = droppingOffDay.value
    droppingOffDay.value = null
    if (target) {
        await school.removeOffDay(target.id).catch(() => {
        })
    }
}

async function addTag() {
    const name = newTag.value.trim()
    if (!name) return
    await school.createTag(name).catch(() => {
    })
    newTag.value = ''
}

/**
 * 강좌 묶음을 만든다. **만든 묶음을 곧바로 고른다** — 묶음을 만드는 이유가 지금 더하는
 * 강좌를 거기 넣으려는 것이라, 만들고 다시 누르게 하면 클릭이 한 번 더 든다.
 */
async function addClassTag() {
    const name = newClassTag.value.trim()
    if (!name) return
    message.value = ''
    const id = await school.createClassTag(name).catch(() => null)
    newClassTag.value = ''
    if (id != null) newSubject.value.groupTagId = id
}

/** 묶음 이름을 고친다. **빈 이름은 되돌린다** — 이름 없는 묶음은 고를 수 없다. */
async function renameClassTag(tag, event) {
    const name = event.target.value.trim()
    if (!name || name === tag.name) {
        event.target.value = tag.name
        message.value = name ? '' : '묶음 이름을 비울 수 없습니다.'
        return
    }
    message.value = ''
    await school.renameClassTag(tag.id, name).catch(() => {
    })
}

/** 묶음 삭제. 강좌는 그대로 남고 묶음 없음으로 간다 — 그래도 행이 사라지므로 한 번 묻는다. */
async function confirmDropClassTag() {
    const target = droppingClassTag.value
    droppingClassTag.value = null
    if (target) {
        await school.deleteClassTag(target.id).catch(() => {
        })
    }
}

/**
 * 맡은 것을 더한다. **더하는 것은 옮겨 가는 일이 아니다.**
 *
 * 새로 만든 것을 곧바로 고르는 동작은 첫 실행 화면을 위한 것이라, 설정에서는 보던
 * 학급으로 되돌린다 — 교과 강좌 하나를 더했다고 화면 전체가 교과 모드로 넘어가면
 * 교사는 무엇을 잘못 눌렀는지 확인하게 된다.
 */
async function addClass(spec) {
    // **고르지 않는다.** 고르면 `app.classId`가 바뀌고, App.vue의 `RouterView :key`가
    // 이 화면을 통째로 다시 만든다 — 열어 둔 명렬표 미리보기가 함께 날아간다.
    await school.createClass({schoolId: app.schoolId, ...spec, select: false})
        .catch(() => {
        })
}

/** 담임 학급 이름은 학년 · 반으로 짓는다 — 첫 실행 화면과 같은 규칙이다. */
async function addHomeroom() {
    const grade = Number(newHomeroom.value.grade)
    const classNo = Number(newHomeroom.value.classNo)
    if (!Number.isInteger(grade) || !Number.isInteger(classNo) || grade < 1 || classNo < 1) {
        message.value = '학년과 반을 1 이상의 수로 적어주세요.'
        return
    }
    message.value = ''
    await addClass({role: 'homeroom', name: `${grade}학년 ${classNo}반`, grade, classNo})
}

/** 교과 강좌는 여러 반에서 모이므로 학년 · 반을 두지 않는다. 이름과 묶음만 받는다. */
async function addSubject() {
    const name = newSubject.value.name.trim()
    if (!name) {
        message.value = '강좌 이름을 적어주세요.'
        return
    }
    message.value = ''
    await addClass({role: 'subject', name, groupTagId: newSubject.value.groupTagId})
    newSubject.value.name = ''
}

/**
 * 이름을 고친다. **빈 이름은 되돌린다** — 이름 없는 학급은 목록에서 고를 수 없다.
 * 묶음은 함께 실어 보낸다. 빼면 이름만 고쳐도 묶음이 벗겨진다.
 */
async function renameClass(cls, event) {
    const name = event.target.value.trim()
    if (!name || name === cls.name) {
        event.target.value = cls.name
        message.value = name ? '' : '학급 이름을 비울 수 없습니다.'
        return
    }
    message.value = ''
    await school.renameClass(cls.id, {
        name,
        grade: cls.grade ?? null,
        classNo: cls.classNo ?? null,
        groupTagId: cls.groupTagId ?? null,
    }).catch(() => {
    })
}

/** 강좌의 묶음을 바꾼다. 이름은 그대로 실어 보낸다 — 고치는 것은 묶음뿐이다. */
async function setGroup(cls, groupTagId) {
    if ((cls.groupTagId ?? null) === groupTagId) return
    message.value = ''
    await school.renameClass(cls.id, {
        name: cls.name,
        grade: cls.grade ?? null,
        classNo: cls.classNo ?? null,
        groupTagId,
    }).catch(() => {
    })
}

/**
 * 맡은 것을 마감한다. **확인을 한 번 거치되 "지웁니다"라고 말하지 않는다** —
 * 지난 출결이 이 학급을 가리키므로 행은 그대로 남고 목록에서만 내려간다.
 */
async function confirmRetireClass() {
    const target = closingClass.value
    closingClass.value = null
    if (!target) return
    // 명렬표 고르개가 마감한 학급을 가리킨 채 남지 않게 한다.
    if (rosterClassId.value === target.id) rosterClassId.value = null
    await school.retireClass(target.id).catch(() => {
    })
}

/**
 * 한도 규정을 추가한다. **실패하면 적은 것을 비우지 않는다** — 이름 · 태그 · 한도를
 * 네 번 눌러 채운 칸이라, 비워 버리면 교사가 처음부터 다시 입력해야 하고 왜 실패했는지도
 * 알 수 없다. 성공했을 때만 비우고, 실패는 문구로 남긴다.
 */
async function addRule() {
    const rule = {...newRule.value, limitN: Number(newRule.value.limitN)}
    if (!rule.name.trim()) {
        message.value = '규정 이름을 적어주세요.'
        return
    }
    message.value = ''
    try {
        await school.createRule(rule)
    } catch (e) {
        message.value = `한도 규정을 저장하지 못했습니다: ${e}`
        return
    }
    newRule.value = blankRule()
}

// **학교가 바뀌면 학교 설정을 다시 읽는다.** 학년도 · 학교를 여기서 옮기므로, 다시
// 읽지 않으면 아래 칸들이 옮기기 전 학교의 값을 그대로 보여준다 — 그 상태에서 저장하면
// 새 학교에 옛 학교의 값이 적힌다.
watch(() => app.schoolId, async () => {
    await school.fetchAll().catch(() => {
    })
})

// 제출 기한 칸은 입력을 받는 자리라 스토어를 그대로 비추지 못한다. 학교가 바뀌거나
// 저장이 끝날 때마다 읽어 온 값으로 되맞춘다.
watch(() => school.school, (row) => {
    dueDays.value = row?.dueDays ?? 7
})

// 한쪽이 실패해도 나머지는 그린다. 실패는 각 스토어의 error에 담겨 화면 위의
// UiNotice가 그대로 보여준다 — 빈 태그 목록만 남겨 두면 설정이 비었다고 읽힌다.
onMounted(async () => {
    await Promise.all([
        school.fetchAll().catch(() => {
        }),
        axis.fetchAll().catch(() => {
        }),
    ])
    dueDays.value = school.school?.dueDays ?? 7
    newYear.value = nextYear()
})
</script>

<template>
    <UiPage subtitle="학년도 · 학교 · 맡은 것 · 명렬표 · 화면" title="설정">
        <UiNotice :text="message" kind="warn"/>
        <UiNotice :text="school.error" kind="error"/>
        <UiNotice :text="axis.error" kind="error"/>

        <UiLedger hint="학교와 맡은 것이 학년도 안에 있다" title="학년도">
            <div class="set__row">
                <span class="set__label">이번 학년도</span>
                <span class="set__value">
                    <button v-for="year in app.years" :key="year.id"
                            :class="['pick', app.yearId === year.id ? 'is-on' : '']"
                            type="button" @click="pickYear(year.id)">
                        {{ year.year }}학년도
                    </button>
                    <span class="set__hint">
                        학년도를 옮기면 학교부터 다시 정합니다. 지난해 학교와 맡은 것은
                        지난 학년도에 그대로 남습니다
                    </span>
                </span>
            </div>

            <div class="set__row">
                <span class="set__label">학년도 추가</span>
                <span class="set__value">
                    <input v-model="newYear" class="field num" min="1900" type="number"
                           @keyup.enter="addYear"/>
                    <UiButton size="tight" variant="primary" @click="addYear">만들기</UiButton>
                    <span class="set__hint">
                        학년도는 3월에 열립니다 —
                        <span class="num">{{ newYearSpan.startsOn }}</span> ~
                        <span class="num">{{ newYearSpan.endsOn }}</span>.
                        만들면 그 학년도로 옮겨 갑니다
                    </span>
                </span>
            </div>
        </UiLedger>

        <UiLedger hint="학년도 안에 있다 · 학교마다 따로 가지는 값이다" title="학교">
            <div class="set__row">
                <span class="set__label">{{ yearLabel }}의 학교</span>
                <span class="set__value">
                    <button v-for="s in app.schools" :key="s.id"
                            :class="['pick', app.schoolId === s.id ? 'is-on' : '']"
                            type="button" @click="pickSchool(s.id)">
                        {{ s.name }}
                    </button>
                    <input v-model="newSchoolName" class="field" placeholder="새 학교"
                           type="text" @keyup.enter="addSchool"/>
                    <UiButton size="tight" @click="addSchool">학교 추가</UiButton>
                    <span class="set__hint">
                        고른 학교에 아래의 설정과 맡은 것이 붙습니다 — 순회 교사는 둘 이상을 맡습니다
                    </span>
                </span>
            </div>

            <!-- 해가 바뀐 직후의 정상 상태다. 빈 화면으로 두면 고장으로 읽힌다. -->
            <div v-if="noSchool" class="set__row">
                <span class="set__label">아직 없습니다</span>
                <span class="set__value">
                    <span class="set__hint">
                        {{ yearLabel }}에 등록한 학교가 없습니다. 위에 이름을 적어 만들면
                        최대 교시 · 제출 기한과 맡은 것을 여기서 정합니다
                    </span>
                </span>
            </div>

            <template v-if="school.school">
                <div class="set__row">
                    <span class="set__label">학교 이름</span>
                    <span class="set__value">
                        <input :value="school.school?.name ?? ''" class="field" type="text"
                               @change="school.saveSchool({name: $event.target.value})"/>
                    </span>
                </div>

                <div class="set__row">
                    <span class="set__label">최대 교시</span>
                    <span class="set__value">
                        <button v-for="n in MAX_SLOT_CHOICES" :key="n"
                                :class="['pick', 'pick--slot', school.school?.maxSlot === n ? 'is-on' : '']"
                                type="button" @click="school.saveSchool({maxSlot: n})">
                            {{ n }}
                        </button>
                        <span class="set__hint">
                            기본값 7. 이 학교의 하루는 조회 · 1~{{ school.school?.maxSlot ?? 7 }}교시 · 종례로 구성된다
                        </span>
                    </span>
                </div>

                <div class="set__row">
                    <span class="set__label">조회 · 종례</span>
                    <span class="set__value">
                        <span class="set__hint">언제나 하루의 양 끝이다. 끄고 켜지 않는다.</span>
                    </span>
                </div>

                <div class="set__row">
                    <span class="set__label">서류 제출 기한</span>
                    <span class="set__value">
                        <input v-model="dueDays" class="field num" min="0" type="number"
                               @change="saveDueDays"/>
                        <span class="set__hint">일</span>
                        <UiToggle v-model="skipOffDays" off-label="달력 그대로"
                                  on-label="주말 · 휴업일 제외"/>
                        <span class="set__hint">결석일로부터 센다. 미제출자 명단의 마감이 이 값으로 계산된다</span>
                    </span>
                </div>

                <div class="set__row">
                    <span class="set__label">휴업일</span>
                    <span class="set__value">
                        <input v-model="newOffDay.date" class="field num" type="date"/>
                        <input v-model="newOffDay.label" class="field" placeholder="개교기념일"
                               type="text"/>
                        <UiButton size="tight" @click="addOffDay">추가</UiButton>
                        <span class="set__hint">공휴일 목록을 내려받지 않는다. 재량휴업일은 어차피 학교마다 다르다</span>
                    </span>
                </div>

                <div v-for="day in school.offDays" :key="day.id" class="set__row">
                    <span class="set__label num">{{ day.date }}</span>
                    <span class="set__value">
                        <span class="set__hint">{{ day.label || '이름 없음' }}</span>
                        <UiButton aria-label="휴업일 지우기" icon title="휴업일 지우기" variant="danger"
                                  @click="droppingOffDay = day">
                            <UiTrashIcon/>
                        </UiButton>
                    </span>
                </div>

                <div class="set__row">
                    <span class="set__label">태그</span>
                    <span class="set__value">
                        <button v-for="tag in school.tags" :key="tag.id" class="pick is-on"
                                title="누르면 마감한다" type="button" @click="school.retireTag(tag.id)">
                            {{ tag.name }} ✕
                        </button>
                        <input v-model="newTag" class="field" placeholder="새 태그" type="text"
                               @keyup.enter="addTag"/>
                        <UiButton size="tight" @click="addTag">태그 추가</UiButton>
                        <span class="set__hint">출결 한 건에 붙는 태그다. 한도 규정이 이 이름을 가리킨다</span>
                    </span>
                </div>

                <div class="set__row">
                    <span class="set__label">학교 마감</span>
                    <span class="set__value">
                        <UiButton size="tight" @click="closingSchool = school.school">마감</UiButton>
                        <span class="set__hint">
                            목록에서 내려갑니다. 이 학교에 쌓인 기록은 그대로 남습니다
                        </span>
                    </span>
                </div>
            </template>
        </UiLedger>

        <UiLedger v-if="school.school"
                  hint="세어야 하는 규정. 이 규정은 출결 입력을 막지 않는다 — 통계 화면에서 세어 알려줄 뿐이다"
                  title="한도 규정">
            <div v-for="rule in school.rules" :key="rule.id" class="set__row">
                <span class="set__label">{{ rule.name }}</span>
                <span class="set__value">
                    <span class="set__hint">
                        <!-- 모르는 기간은 빈칸으로 두지 않고 값을 그대로 적는다.
                             비우면 `마다 20일`이 되어 무엇을 세는 규정인지 사라진다. -->
                        태그 {{ rule.tagName ?? '전체' }} ·
                        {{ PERIODS.find((p) => p.value === rule.period)?.label ?? rule.period }}마다
                        <span class="num">{{ rule.limitN }}</span>{{ rule.unit === 'day' ? '일' : '회' }}
                    </span>
                    <UiButton size="tight" @click="school.retireRule(rule.id)">마감</UiButton>
                </span>
            </div>

            <div class="set__row">
                <span class="set__label">규정 추가</span>
                <span class="set__value">
                    <input v-model="newRule.name" class="field" placeholder="체험학습 연 20일"
                           type="text"/>
                    <button v-for="tag in school.tags" :key="tag.id"
                            :class="['pick', newRule.tagId === tag.id ? 'is-on' : '']"
                            type="button"
                            @click="newRule.tagId = newRule.tagId === tag.id ? null : tag.id">
                        {{ tag.name }}
                    </button>
                    <button v-for="period in PERIODS" :key="period.value"
                            :class="['pick', newRule.period === period.value ? 'is-on' : '']"
                            type="button" @click="newRule.period = period.value">
                        {{ period.label }}
                    </button>
                    <input v-model="newRule.limitN" class="field num" min="1" type="number"/>
                    <button v-for="unit in UNITS" :key="unit.value"
                            :class="['pick', newRule.unit === unit.value ? 'is-on' : '']"
                            type="button" @click="newRule.unit = unit.value">
                        {{ unit.label }}
                    </button>
                    <UiButton size="tight" variant="primary" @click="addRule">추가</UiButton>
                </span>
            </div>
        </UiLedger>

        <UiLedger v-if="school.school"
                  :hint="`${school.school?.name ?? ''}에서 맡은 것 · 마감은 삭제가 아니다`"
                  title="맡은 것">
            <!-- 담임과 교과를 한 목록에 나란히 둔다. 기록 화면은 모드로 완전히 분리되지만,
                 무엇을 맡았는지 정하는 일은 두 갈래를 함께 봐야 한다.
                 **옮겨 가는 단추는 두지 않는다** — 모드를 넘나드는 길은 스위치 하나다. -->
            <div v-for="cls in app.homeroomClasses" :key="cls.id" class="set__row">
                <span class="set__label">담임</span>
                <span class="set__value">
                    <input :value="cls.name" class="field" type="text"
                           @change="renameClass(cls, $event)"/>
                    <button :class="['pick', rosterTarget === cls.id ? 'is-on' : '']"
                            type="button" @click="rosterClassId = cls.id">명렬표</button>
                    <UiButton size="tight" @click="closingClass = cls">마감</UiButton>
                </span>
            </div>

            <div class="set__row">
                <span class="set__label">담임 추가</span>
                <span class="set__value">
                    <input v-model="newHomeroom.grade" class="field num" min="1" type="number"/>
                    <span class="set__hint">학년</span>
                    <input v-model="newHomeroom.classNo" class="field num" min="1" type="number"/>
                    <span class="set__hint">반</span>
                    <UiButton size="tight" variant="primary" @click="addHomeroom">추가</UiButton>
                    <span class="set__hint">이름은 학년 · 반으로 짓습니다. 나중에 고칠 수 있습니다</span>
                </span>
            </div>

            <!-- 묶음별로 모아 보여준다. 머리글은 이름표일 뿐이라 누르는 것이 아니다. -->
            <template v-for="group in subjectGroups" :key="group.key">
                <div class="grp">
                    <b>{{ group.name }}</b>
                    <span class="set__hint"><span class="num">{{ group.items.length }}</span>개 강좌</span>
                </div>
                <div v-for="cls in group.items" :key="cls.id" class="set__row">
                    <span class="set__label">교과</span>
                    <span class="set__value">
                        <input :value="cls.name" class="field" type="text"
                               @change="renameClass(cls, $event)"/>
                        <span class="set__hint">묶음</span>
                        <button :class="['pick', cls.groupTagId == null ? 'is-on' : '']"
                                type="button" @click="setGroup(cls, null)">없음</button>
                        <button v-for="tag in school.classTags" :key="tag.id"
                                :class="['pick', cls.groupTagId === tag.id ? 'is-on' : '']"
                                type="button" @click="setGroup(cls, tag.id)">
                            {{ tag.name }}
                        </button>
                        <button :class="['pick', rosterTarget === cls.id ? 'is-on' : '']"
                                type="button" @click="rosterClassId = cls.id">명렬표</button>
                        <UiButton size="tight" @click="closingClass = cls">마감</UiButton>
                    </span>
                </div>
            </template>

            <div class="set__row">
                <span class="set__label">교과 추가</span>
                <span class="set__value">
                    <input v-model="newSubject.name" class="field" placeholder="프로그래밍A"
                           type="text" @keyup.enter="addSubject"/>
                    <span class="set__hint">묶음</span>
                    <button :class="['pick', newSubject.groupTagId == null ? 'is-on' : '']"
                            type="button" @click="newSubject.groupTagId = null">없음</button>
                    <button v-for="tag in school.classTags" :key="tag.id"
                            :class="['pick', newSubject.groupTagId === tag.id ? 'is-on' : '']"
                            type="button" @click="newSubject.groupTagId = tag.id">
                        {{ tag.name }}
                    </button>
                    <UiButton size="tight" variant="primary" @click="addSubject">추가</UiButton>
                    <span class="set__hint">강좌는 여러 반에서 모이므로 학년 · 반을 두지 않습니다</span>
                </span>
            </div>

            <div class="set__row">
                <span class="set__label">강좌 묶음</span>
                <span class="set__value">
                    <span v-for="tag in school.classTags" :key="tag.id" class="grp__edit">
                        <input :value="tag.name" class="field grp__name" type="text"
                               @change="renameClassTag(tag, $event)"/>
                        <UiButton :aria-label="`${tag.name} 묶음 지우기`" :title="`${tag.name} 묶음 지우기`"
                                  icon variant="danger" @click="droppingClassTag = tag">
                            <UiTrashIcon/>
                        </UiButton>
                    </span>
                    <input v-model="newClassTag" class="field" placeholder="새 묶음" type="text"
                           @keyup.enter="addClassTag"/>
                    <UiButton size="tight" @click="addClassTag">묶음 추가</UiButton>
                    <span class="set__hint">
                        프로그래밍A · B · C를 프로그래밍으로 묶습니다. 분반 표가 아니라 이름표입니다
                    </span>
                </span>
            </div>
        </UiLedger>

        <UiLedger v-if="school.school"
                  :hint="rosterClass ? `${rosterClass.name} 명단에 넣습니다` : '학급을 먼저 만들어주세요'"
                  title="명렬표">
            <div class="set__row">
                <span class="set__label">파일에서 가져오기</span>
                <span class="set__value">
                    <!-- 자리는 늘 여기다. 위에서 [명렬표]를 누르면 상자가 새로 생기는 것이
                         아니라 이 상자가 가리키는 학급만 바뀐다. -->
                    <RosterPanel :class-id="rosterTarget"/>
                </span>
            </div>
        </UiLedger>

        <UiLedger hint="이 값만 DB가 아니라 이 컴퓨터에 저장된다" title="화면">
            <div class="set__row">
                <span class="set__label">테마</span>
                <span class="set__value">
                    <button v-for="choice in THEME_CHOICES" :key="choice.value"
                            :class="['pick', theme.mode.value === choice.value ? 'is-on' : '']"
                            type="button" @click="theme.setMode(choice.value)">
                        {{ choice.label }}
                    </button>
                    <span class="set__hint">
                        첫 페인트 전에 읽어야 하므로 DB가 아니라 localStorage에 둔다
                    </span>
                </span>
            </div>
        </UiLedger>

        <UiModal :open="Boolean(droppingOffDay)" title="이 휴업일을 지웁니다"
                 @close="droppingOffDay = null">
            <div v-if="droppingOffDay" class="modal__what">
                <span class="modal__key">날짜</span>
                <span class="modal__val num">{{ droppingOffDay.date }}</span>
                <span class="modal__key">이름</span>
                <span class="modal__val">{{ droppingOffDay.label || '—' }}</span>
            </div>
            <p class="modal__note">
                지운 휴업일은 되돌릴 수 없습니다. 이미 계산해 둔 마감일은 그대로 남고,
                앞으로 만드는 출결의 마감만 이 날을 일수에 포함합니다.
            </p>

            <template #foot>
                <UiButton size="wide" @click="droppingOffDay = null">취소</UiButton>
                <UiButton fill size="wide" variant="danger" @click="confirmRemoveOffDay">
                    지우기
                </UiButton>
            </template>
        </UiModal>

        <UiModal :open="Boolean(droppingClassTag)" title="이 묶음을 지웁니다"
                 @close="droppingClassTag = null">
            <div v-if="droppingClassTag" class="modal__what">
                <span class="modal__key">묶음</span>
                <span class="modal__val">{{ droppingClassTag.name }}</span>
                <span class="modal__key">묶인 강좌</span>
                <span class="modal__val num">{{ droppingClassTag.classCount ?? 0 }}개</span>
            </div>
            <p class="modal__note">
                강좌는 지워지지 않습니다. 이 묶음에 들어 있던 강좌는 묶음 없음으로 내려가고,
                지운 묶음은 되돌릴 수 없습니다.
            </p>

            <template #foot>
                <UiButton size="wide" @click="droppingClassTag = null">취소</UiButton>
                <UiButton fill size="wide" variant="danger" @click="confirmDropClassTag">
                    지우기
                </UiButton>
            </template>
        </UiModal>

        <UiModal :open="Boolean(closingSchool)" title="이 학교를 목록에서 내립니다"
                 @close="closingSchool = null">
            <div v-if="closingSchool" class="modal__what">
                <span class="modal__key">학교</span>
                <span class="modal__val">{{ closingSchool.name }}</span>
                <span class="modal__key">학년도</span>
                <span class="modal__val">{{ yearLabel }}</span>
            </div>
            <p class="modal__note">
                지우는 것이 아닙니다. 목록에서 내려가 더 고를 수 없게 될 뿐이고,
                이 학교에 쌓인 맡은 것과 지난 기록은 그대로 남습니다.
            </p>

            <template #foot>
                <UiButton size="wide" @click="closingSchool = null">취소</UiButton>
                <UiButton size="wide" variant="primary" @click="confirmRetireSchool">마감</UiButton>
            </template>
        </UiModal>

        <UiModal :open="Boolean(closingClass)" title="이 학급을 목록에서 내립니다"
                 @close="closingClass = null">
            <div v-if="closingClass" class="modal__what">
                <span class="modal__key">맡은 것</span>
                <span class="modal__val">{{ closingClass.name }}</span>
                <span class="modal__key">구분</span>
                <span class="modal__val">
                    {{ closingClass.role === 'homeroom' ? '담임 학급' : '교과 강좌' }}
                </span>
            </div>
            <p class="modal__note">
                지우는 것이 아닙니다. 목록에서 내려가 더 고를 수 없게 될 뿐이고,
                이 학급에 쌓인 지난 기록은 그대로 남습니다.
            </p>

            <template #foot>
                <UiButton size="wide" @click="closingClass = null">취소</UiButton>
                <UiButton size="wide" variant="primary" @click="confirmRetireClass">마감</UiButton>
            </template>
        </UiModal>
    </UiPage>
</template>

<style scoped>
/* 묶음 머리글. 줄이 아니라 이름표라 한 단 눌러 둔다 — 누르는 것이 아니다. */
.grp {
    display: flex;
    align-items: baseline;
    gap: var(--s-md);
    padding: var(--s-sm) var(--s-2xl);
    border-top: 1px solid var(--c-line-soft);
    background: var(--c-raised);
}

/* 묶음 이름 칸과 지우기 단추는 한 벌로 움직인다. */
.grp__edit {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2xs);
}

.grp__name {
    width: 140px;
}
</style>
