<script setup>
/**
 * 설정 — 학교 · 한도 규정 · 맡은 것 · 명렬표 · 화면.
 *
 * **학교 단위 값이 여기 모인다.** 최대 교시와 제출 기한은 앱 상수가 아니라 학교가
 * 들고 있는 값이다. 순회 교사가 학교를 둘 이상 등록하는 날이 와도 자리를 옮기지 않는다.
 *
 * **맡은 것을 더하고 마감하는 곳도 여기다.** 담임 학급과 교과 강좌가 한 목록에 나란히
 * 서는 화면은 여기뿐이다 — 기록 화면은 모드로 완전히 갈리지만, 무엇을 맡았는지 정하는
 * 일은 두 갈래를 함께 봐야 한다. 여기에 나오는 것은 맡은 것의 목록일 뿐 한쪽의 기록이
 * 다른 쪽에 새는 것이 아니다. **학급을 옮기는 길은 여기 두지 않는다** — 그것은 이동
 * 화면과 모드 스위치가 맡는다.
 *
 * 명렬표 가져오기도 여기 있다 — 학기에 한두 번 쓰는 일이라 사이드바에 둘 이유가 없다.
 */
import {computed, onMounted, ref} from 'vue'
import {useAppStore} from '../stores/app'
import {useAxisStore} from '../stores/axis'
import {useSchoolStore} from '../stores/school'
import RosterPanel from '../components/RosterPanel.vue'
import {UiButton, UiLedger, UiModal, UiNotice, UiPage, UiToggle, UiTrashIcon} from '../components/ui'
import {MAX_SLOT_CHOICES} from '../data/slotChoices'
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
const PERIODS = [
    {value: 'year', label: '학년도'},
    {value: 'semester', label: '학기'},
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
const newHomeroom = ref({grade: 3, classNo: 1})
const newSubjectName = ref('')
const newSchoolName = ref('')
const message = ref('')

/** 지우려고 고른 휴업일. 태그 · 규정의 [마감]과 달리 이것은 행을 지우는 DELETE다. */
const droppingOffDay = ref(null)

/** 마감하려고 고른 맡은 것. 지우는 것이 아니라 목록에서 내리는 것이다. */
const closingClass = ref(null)

/** 새로 만드는 담임 · 교과가 들어갈 학교. 고르기 전에는 지금 보고 있는 학교다. */
const targetSchoolId = ref(null)
const schoolFor = computed(() => targetSchoolId.value ?? app.schoolId)

/**
 * 명렬표를 넣을 학급. 고르기 전에는 지금 보고 있는 학급이다.
 *
 * 줄마다 명렬표 상자를 펼치지 않고 **고르개만 둔다** — 상자가 생겼다 사라지면 아래
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

/** 그 맡은 것이 걸린 학교 이름. 순회 교사는 같은 학년 · 반을 학교마다 가진다. */
function schoolNameOf(cls) {
    return app.schools.find((s) => s.id === cls.schoolId)?.name ?? ''
}

/**
 * 맡은 것을 더한다. **더하는 것은 옮겨 가는 일이 아니다.**
 *
 * 새로 만든 것을 곧바로 고르는 동작은 첫 실행 화면을 위한 것이라, 설정에서는 보던
 * 학급으로 되돌린다 — 교과 강좌 하나를 더했다고 화면 전체가 교과 모드로 넘어가면
 * 교사는 무엇을 잘못 눌렀는지 되짚게 된다.
 */
async function addClass(spec) {
    // **고르지 않는다.** 고르면 `app.classId`가 바뀌고, App.vue의 `RouterView :key`가
    // 이 화면을 통째로 다시 만든다 — 펼쳐 둔 명렬표 미리보기와 고르던 학교가 함께
    // 날아가, 이어서 누른 [추가]가 엉뚱한 학교로 들어간다.
    await school.createClass({schoolId: schoolFor.value, ...spec, select: false})
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

/** 교과 강좌는 여러 반에서 모이므로 학년 · 반을 두지 않는다. 이름만 받는다. */
async function addSubject() {
    const name = newSubjectName.value.trim()
    if (!name) {
        message.value = '강좌 이름을 적어주세요.'
        return
    }
    message.value = ''
    await addClass({role: 'subject', name})
    newSubjectName.value = ''
}

/** 이름을 고친다. **빈 이름은 되돌린다** — 이름 없는 학급은 목록에서 고를 수 없다. */
async function renameClass(cls, event) {
    const name = event.target.value.trim()
    if (!name || name === cls.name) {
        event.target.value = cls.name
        message.value = name ? '' : '학급 이름을 비울 수 없습니다.'
        return
    }
    message.value = ''
    await school.renameClass(cls.id, {
        name, grade: cls.grade ?? null, classNo: cls.classNo ?? null,
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

/** 학교를 더한다. 더한 뒤 위의 [학교] 고르개가 그 학교를 가리킨다. */
async function addSchool() {
    const name = newSchoolName.value.trim()
    if (!name) {
        message.value = '학교 이름을 적어주세요.'
        return
    }
    message.value = ''
    const id = await school.createSchool({name}).catch(() => null)
    if (id != null) targetSchoolId.value = id
    newSchoolName.value = ''
}

async function addRule() {
    const rule = {...newRule.value, limitN: Number(newRule.value.limitN)}
    if (!rule.name.trim()) {
        message.value = '규정 이름을 적어주세요.'
        return
    }
    message.value = ''
    await school.createRule(rule).catch(() => {
    })
    newRule.value = blankRule()
}

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
})
</script>

<template>
    <UiPage subtitle="학교 · 맡은 것 · 명렬표 · 화면" title="설정">
        <UiNotice :text="message" kind="warn"/>
        <UiNotice :text="school.error" kind="error"/>
        <UiNotice :text="axis.error" kind="error"/>

        <UiLedger hint="학교마다 따로 가지는 값이다" title="학교">
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
                        기본값 7. 이 학교의 하루는 조회 · 1~{{ school.school?.maxSlot ?? 7 }}교시 · 종례로 이어진다
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
                    <span class="set__hint">한도 규정이 이 이름을 가리킨다</span>
                </span>
            </div>
        </UiLedger>

        <UiLedger hint="세어야 하는 규정. 이 규정은 출결 입력을 막지 않는다 — 통계 화면에서 세어 알려줄 뿐이다"
                  title="한도 규정">
            <div v-for="rule in school.rules" :key="rule.id" class="set__row">
                <span class="set__label">{{ rule.name }}</span>
                <span class="set__value">
                    <span class="set__hint">
                        태그 {{ rule.tagName ?? '전체' }} ·
                        {{ PERIODS.find((p) => p.value === rule.period)?.label }}마다
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

        <UiLedger hint="마감은 삭제가 아니다. 목록에서 내려갈 뿐 지난 기록은 그대로 남는다"
                  title="맡은 것">
            <!-- 담임과 교과를 한 목록에 나란히 둔다. 기록 화면은 모드로 완전히 갈리지만,
                 무엇을 맡았는지 정하는 일은 두 갈래를 함께 봐야 한다.
                 **옮겨 가는 단추는 두지 않는다** — 모드를 넘나드는 길은 스위치 하나다. -->
            <div v-for="cls in app.homeroomClasses" :key="cls.id" class="set__row">
                <span class="set__label">담임</span>
                <span class="set__value">
                    <input :value="cls.name" class="field" type="text"
                           @change="renameClass(cls, $event)"/>
                    <span class="set__hint">{{ schoolNameOf(cls) }}</span>
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

            <div v-for="cls in app.subjectClasses" :key="cls.id" class="set__row">
                <span class="set__label">교과</span>
                <span class="set__value">
                    <input :value="cls.name" class="field" type="text"
                           @change="renameClass(cls, $event)"/>
                    <span class="set__hint">{{ schoolNameOf(cls) }}</span>
                    <!-- 교과 강좌는 반이 섞여 번호만으로 학생을 가릴 수 없다. 명렬표
                         들이기가 아직 담임만이라 단추를 열지 않는다 — 자리는 그대로
                         두어 담임 줄과 폭이 어긋나지 않게 한다. -->
                    <button class="pick" disabled title="다음 판에서 만듭니다"
                            type="button">명렬표는 아직</button>
                    <UiButton size="tight" @click="closingClass = cls">마감</UiButton>
                </span>
            </div>

            <div class="set__row">
                <span class="set__label">교과 추가</span>
                <span class="set__value">
                    <input v-model="newSubjectName" class="field" placeholder="3학년 통합사회"
                           type="text" @keyup.enter="addSubject"/>
                    <UiButton size="tight" variant="primary" @click="addSubject">추가</UiButton>
                    <span class="set__hint">강좌는 여러 반에서 모이므로 학년 · 반을 두지 않습니다</span>
                </span>
            </div>

            <div class="set__row">
                <span class="set__label">학교</span>
                <span class="set__value">
                    <button v-for="s in app.schools" :key="s.id"
                            :class="['pick', schoolFor === s.id ? 'is-on' : '']"
                            type="button" @click="targetSchoolId = s.id">
                        {{ s.name }}
                    </button>
                    <input v-model="newSchoolName" class="field" placeholder="새 학교"
                           type="text" @keyup.enter="addSchool"/>
                    <UiButton size="tight" @click="addSchool">학교 추가</UiButton>
                    <span class="set__hint">
                        여기서 고른 학교에 위의 담임 · 교과가 들어갑니다 — 순회 교사는 둘 이상을 맡습니다
                    </span>
                </span>
            </div>
        </UiLedger>

        <UiLedger :hint="rosterClass ? `${rosterClass.name} 명단에 넣습니다` : '학급을 먼저 만들어주세요'"
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
