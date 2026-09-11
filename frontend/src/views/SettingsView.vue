<script setup>
/**
 * 설정 — 학교 · 한도 규정 · 학급 · 화면.
 *
 * **학교 단위 값이 여기 모인다.** 최대 교시와 제출 기한은 앱 상수가 아니라 학교가
 * 들고 있는 값이다. 순회 교사가 학교를 둘 이상 등록하는 날이 와도 자리를 옮기지 않는다.
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
const message = ref('')

/** 지우려고 고른 휴업일. 태그 · 규정의 [마감]과 달리 이것은 행을 지우는 DELETE다. */
const droppingOffDay = ref(null)

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
    <UiPage subtitle="학교 · 학급 · 화면" title="설정">
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

        <UiLedger hint="담임을 맡은 학급 하나" title="학급">
            <div class="set__row">
                <span class="set__label">학년도</span>
                <span class="set__value">
                    <button v-for="year in app.years" :key="year.id"
                            :class="['pick', 'num', app.yearId === year.id ? 'is-on' : '']"
                            type="button"
                            @click="app.selectClass({yearId: year.id, grade: app.grade, classNo: app.classNo})">
                        {{ year.year }}
                    </button>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">학급</span>
                <span class="set__value">
                    <input :value="app.grade" class="field num" min="1" type="number"
                           @change="app.selectClass({yearId: app.yearId, grade: Number($event.target.value), classNo: app.classNo})"/>
                    <span class="set__hint">학년</span>
                    <input :value="app.classNo" class="field num" min="1" type="number"
                           @change="app.selectClass({yearId: app.yearId, grade: app.grade, classNo: Number($event.target.value)})"/>
                    <span class="set__hint">반</span>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">명렬표</span>
                <span class="set__value">
                    <RosterPanel/>
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
    </UiPage>
</template>
