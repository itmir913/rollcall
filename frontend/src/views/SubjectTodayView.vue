<script setup>
/**
 * 오늘 수업 — 교과 교사가 찍는 곳.
 *
 * 기록하는 것은 **내 수업에 있었는가 하나뿐**이다. 구분 · 종류 · 기간 · 서류 · 나이스가
 * 여기 오지 않는다 — 그것은 담임이 쓰는 말이다.
 *
 * 수업을 시작할 때 [교시 추가]로 칸을 만들고, 그 칸에서 빠진 학생만 번호를 눌러 적는다.
 * **칸이 있다는 것이 곧 그 교시를 불렀다는 뜻이다.** 결석자 행만으로는 "빠진 사람이
 * 없는 날"과 "아직 안 부른 날"이 구별되지 않는다.
 *
 * 교시는 버튼으로 표시한다. 드롭다운은 열기 전까지 후보가 보이지 않아 클릭이 한 번 더 든다.
 * **조회 · 종례는 없다** — 그 둘은 담임이 하루의 양 끝에서 보는 시간이지 누가 가르치는
 * 시간이 아니다.
 *
 * 담임으로 적어 둔 기록(`homeroomNote`)은 **읽기 전용 참고**로만 보인다. 아침에 담임이
 * 질병결석을 찍어 두었으면 그것을 보여주는 것이 기능이지만, 내가 이 화면에서 찍은 것과
 * 눈에 띄게 구별되어야 한다 — 그래서 격자에서는 아래 띠로만 표시하고 문장은 별도 장부에 둔다.
 */
import {computed, onMounted, ref, watch} from 'vue'
import {useRouter} from 'vue-router'
import {formatKorean, useAppStore} from '../stores/app'
import {useSubjectStore} from '../stores/subject'
import {UiButton, UiLedger, UiModal, UiNotice, UiPage, UiTrashIcon} from '../components/ui'

const app = useAppStore()
const subject = useSubjectStore()
const router = useRouter()

/** 지금 고른 교시들. [교시 추가]를 누르면 한꺼번에 칸이 된다. */
const picked = ref([])
/** 메모를 연 줄. 차시 메모는 `session:<id>`, 결석 메모는 학생 번호로 구분한다. */
const expanded = ref(null)
const draft = ref('')
const dropping = ref(null)

const dateLabel = computed(() => formatKorean(subject.date))

/** 교시 후보. 최대 교시는 학교가 들고 있는 값이라 앱 상수로 박아 두지 않는다. */
const slots = computed(() => Array.from({length: app.maxSlot}, (_, i) => i + 1))

/** 이미 만든 교시. 같은 칸을 또 누르지 않도록 눌러 둔다 — 칸은 그대로 남는다. */
const made = computed(() => new Set(subject.daySessions.map((s) => String(s.slot))))

/**
 * 날짜를 옮기면 고르던 교시와 열어 둔 메모 칸을 함께 비운다.
 *
 * 고른 교시는 **그 날짜의 것**이다. 9월 11일에서 3 · 4교시를 고른 채 12일로 옮기면
 * 버튼이 눌린 그대로 남아, [교시 추가]가 교사가 고른 적 없는 칸을 12일에 만든다.
 * 열어 둔 메모 칸도 앞 날짜의 차시 · 학생을 가리키므로 함께 닫는다.
 */
watch(() => subject.date, () => {
    picked.value = []
    expanded.value = null
    draft.value = ''
})

/**
 * 칸 하나에 붙는 설명. **학년 · 반이 먼저 온다** — 교과 강좌는 반이 섞이므로
 * 3학년 1반 4번과 3학년 6반 4번이 한 명단에 있다. 번호만으로는 구별되지 않는다.
 */
function seatTitle(row) {
    const who = `${classLabel(row)} ${row.number}번 ${row.name}`
    return row.homeroomNote ? `${who} · 담임 기록(참고) — ${row.homeroomNote}` : who
}

/** `3학년 6반`. 학적이 비어 있으면 자리는 남기고 값만 비운다 — 칸 높이가 줄마다 달라지면 안 된다. */
function classLabel(row) {
    return row.grade == null || row.classNo == null ? '—' : `${row.grade}학년 ${row.classNo}반`
}

/** 격자에 넣는 짧은 표기. `3-6` */
function classShort(row) {
    return row.grade == null || row.classNo == null ? '—' : `${row.grade}-${row.classNo}`
}

/**
 * 빈 명단에 적을 말. **`없다`와 `아직 고르지 않았다`는 다르다** —
 * 차시를 고르기 전에는 그 칸의 명단을 읽은 적조차 없으므로 없다고 단언할 수 없다.
 */
const rollEmptyText = computed(() => (subject.current
    ? '이 차시에 빠진 학생이 없습니다.'
    : '아직 교시를 고르지 않았습니다. 위에서 교시를 고르면 그 칸의 명단이 열립니다.'))

/**
 * 담임 참고에 적을 말. 같은 이유로 둘을 구분한다 — 조회한 적 없는 날에 대해
 * "기록이 없습니다"라고 적으면, 교과 교사가 빈 자리를 보고 담임이 아무것도 적지
 * 않았다고 확인한 셈이 된다.
 */
const noteEmptyText = computed(() => (subject.current
    ? '그날 담임으로 적어 둔 기록이 없습니다.'
    : '아직 교시를 고르지 않았습니다. 교시를 고르면 그날 담임으로 적어 둔 기록도 함께 표시합니다.'))

function pick(slot) {
    picked.value = picked.value.includes(slot)
        ? picked.value.filter((s) => s !== slot)
        : [...picked.value, slot].sort((a, b) => a - b)
}

async function add() {
    const chosen = [...picked.value]
    picked.value = []
    await subject.addSessions(chosen).catch(() => {
    })
}

async function pickDate(iso) {
    subject.setDate(iso)
    await subject.fetchDay().catch(() => {
    })
}

/** 번호를 누르면 그 자리에서 찍힌다. 다시 누르면 취소이므로 묻지 않는다. */
async function toggle(studentId) {
    await subject.toggle(studentId).catch(() => {
    })
}

/** 메모 칸을 연다. 열 때만 원본을 읽는다 — 타이핑 중에 덮으면 커서가 튄다. */
function expand(key, value) {
    if (expanded.value === key) {
        expanded.value = null
        return
    }
    expanded.value = key
    draft.value = value ?? ''
}

async function saveSessionMemo(sessionId) {
    const memo = draft.value
    expanded.value = null
    await subject.setSessionMemo(sessionId, memo).catch(() => {
    })
}

async function saveAbsenceMemo(studentId) {
    const memo = draft.value
    expanded.value = null
    await subject.setAbsenceMemo(studentId, memo).catch(() => {
    })
}

async function confirmDrop() {
    const target = dropping.value
    dropping.value = null
    if (target) await subject.removeSession(target.id).catch(() => {
    })
}

onMounted(async () => {
    if (!app.ready) return
    if (!subject.date) subject.setDate(app.today)
    await subject.fetchDay().catch(() => {
    })
})
</script>

<template>
    <!-- 강좌를 하나도 등록하지 않았다. 무엇을 해야 하는지와 가는 길을 함께 둔다. -->
    <UiPage v-if="app.subjectClasses.length === 0" title="오늘 수업">
        <UiNotice kind="warn"
                  text="아직 수업을 등록하지 않았습니다. 강좌를 만들면 교시를 추가하고 결석을 찍을 수 있습니다."/>
        <div class="lead">
            <UiButton size="wide" variant="primary" @click="router.push('/settings')">
                수업 등록하기
            </UiButton>
        </div>
    </UiPage>

    <!-- 맡은 강좌는 있는데 아직 고르지 않았다. 등록하라고 적으면 이미 한 일을 다시 시킨다. -->
    <UiPage v-else-if="!app.ready" title="오늘 수업">
        <UiNotice kind="warn" text="보고 있는 수업이 없습니다. 맡은 강좌 중 하나를 고르세요."/>
        <div class="lead">
            <UiButton size="wide" variant="primary" @click="router.push('/move')">
                수업 고르기
            </UiButton>
        </div>
    </UiPage>

    <UiPage v-else :subtitle="`${app.currentClass?.name ?? ''} · 차시 ${subject.daySessions.length}개`"
            :title="dateLabel">
        <template #actions>
            <UiButton @click="subject.move(-1)">◀ 어제</UiButton>
            <input :value="subject.date" class="field num" type="date"
                   @change="pickDate($event.target.value)"/>
            <UiButton @click="subject.move(1)">내일 ▶</UiButton>
        </template>

        <UiLedger hint="조회 · 종례는 교과 수업이 아니다 · 연강도 두 칸이다" title="교시 추가">
            <template #actions>
                <UiButton :disabled="picked.length === 0 || subject.busy" variant="primary"
                          @click="add">
                    교시 추가
                </UiButton>
            </template>
            <div class="filters pickline">
                <span class="filters__label">교시</span>
                <button v-for="slot in slots" :key="slot"
                        :class="['pick', 'pick--slot', picked.includes(slot) ? 'is-on' : '']"
                        :disabled="made.has(String(slot))"
                        :title="made.has(String(slot)) ? '이미 만든 교시입니다' : ''"
                        type="button" @click="pick(slot)">
                    {{ slot }}
                </button>
            </div>
        </UiLedger>

        <UiLedger :empty="subject.daySessions.length === 0"
                  :note="`빠진 사람 ${subject.dayAbsentTotal}명`"
                  class="list--sess"
                  empty-text="아직 만든 교시가 없습니다. 위에서 교시를 골라 추가하세요."
                  hint="칸을 만든 것이 곧 그 교시를 불렀다는 뜻이다" title="오늘의 차시">
            <template v-for="session in subject.daySessions" :key="session.id">
                <div :class="['row', session.id === subject.sessionId ? 'is-ok' : 'is-calm']">
                    <span class="row__what">
                        <button class="cellbtn cellbtn--plain" type="button"
                                @click="subject.select(session.id)">
                            <b class="num">{{ session.slot }}교시</b>
                        </button>
                    </span>
                    <span class="row__when num">빠진 사람 {{ session.absentCount }} / {{ session.total }}명</span>
                    <span class="row__memo">
                        <button :class="['cellbtn', session.memo ? 'has-value' : '']" type="button"
                                @click="expand(`session:${session.id}`, session.memo)">
                            {{ session.memo || '메모를 입력하세요' }}
                        </button>
                    </span>
                    <span class="row__acts">
                        <UiButton aria-label="차시 지우기" icon title="차시 지우기" variant="danger"
                                  @click="dropping = session">
                            <UiTrashIcon/>
                        </UiButton>
                    </span>
                </div>
                <div v-if="expanded === `session:${session.id}`" class="edit">
                    <textarea v-model="draft" class="edit__area" placeholder="수행평가 · 보강처럼 그 차시에 남길 말"
                              rows="2"></textarea>
                    <div class="edit__row">
                        <UiButton class="edit__done" size="tight"
                                  @click="saveSessionMemo(session.id)">
                            완료
                        </UiButton>
                    </div>
                </div>
            </template>
        </UiLedger>

        <UiLedger :hint="subject.current
                      ? '번호를 누르면 결석이 찍힙니다 · 다시 누르면 취소됩니다'
                      : '위에서 교시를 고르면 그 칸의 명단이 열립니다'"
                  :note="subject.current ? `빠진 사람 ${subject.absentRows.length} / ${subject.roll.length}명` : ''"
                  :title="subject.current ? `${subject.current.slot}교시 명단` : '명단'">
            <div class="roll">
                <p v-if="!subject.current" class="muted">아직 교시를 고르지 않았습니다.</p>
                <template v-else>
                    <!-- 학년 · 반이 번호 위에 온다. 교과 강좌는 반이 섞이므로
                         3학년 1반 4번과 3학년 6반 4번이 같은 격자에 나란히 놓인다. -->
                    <div class="seats">
                        <button v-for="row in subject.roll" :key="row.studentId"
                                :class="['seat', row.absent ? 'is-marked' : '']"
                                :disabled="subject.busy"
                                :title="seatTitle(row)"
                                type="button" @click="toggle(row.studentId)">
                            <span class="seat__cls num">{{ classShort(row) }}</span>
                            <span class="seat__no num">{{ row.number }}</span>
                            <span class="seat__name">{{ row.name }}</span>
                            <i :class="['seat__note', row.homeroomNote ? 'is-on' : '']"
                               aria-hidden="true"></i>
                        </button>
                    </div>
                    <div class="legend">
                        <span><i class="sw-ok"></i>내가 찍은 결석</span>
                        <span><i class="sw-line"></i>출석</span>
                        <span><i class="sw-note"></i>담임 기록 있음 — 참고이지 내 기록이 아니다</span>
                    </div>
                </template>
            </div>
        </UiLedger>

        <UiLedger :empty="subject.absentRows.length === 0" :empty-text="rollEmptyText"
                  class="list--roll"
                  hint="빠진 사람이 없는 것과 아직 부르지 않은 것은 다르다"
                  title="이 차시에 빠진 학생">
            <template v-for="row in subject.absentRows" :key="row.studentId">
                <div class="row is-ok">
                    <span class="row__cls num">{{ classLabel(row) }}</span>
                    <span class="row__no num">{{ row.number }}</span>
                    <span class="row__name"><b>{{ row.name }}</b></span>
                    <span class="row__what">결석</span>
                    <span class="row__memo">
                        <button :class="['cellbtn', row.memo ? 'has-value' : '']" type="button"
                                @click="expand(row.studentId, row.memo)">
                            {{ row.memo || '메모를 입력하세요' }}
                        </button>
                    </span>
                    <span class="row__acts">
                        <UiButton size="tight" @click="toggle(row.studentId)">무르기</UiButton>
                    </span>
                </div>
                <div v-if="expanded === row.studentId" class="edit">
                    <textarea v-model="draft" class="edit__area" placeholder="메모를 입력하세요"
                              rows="2"></textarea>
                    <div class="edit__row">
                        <UiButton class="edit__done" size="tight"
                                  @click="saveAbsenceMemo(row.studentId)">
                            완료
                        </UiButton>
                    </div>
                </div>
            </template>
        </UiLedger>

        <!-- 의도 7. 담임으로 적어 둔 기록을 여기서 고칠 수 없다. 읽기 전용 참고다. -->
        <UiLedger :empty="subject.notedRows.length === 0" :empty-text="noteEmptyText"
                  class="list--note"
                  hint="담임 화면에서 적은 것이다 · 읽기 전용이라 여기서 고칠 수 없다"
                  title="담임 기록 — 참고">
            <div v-for="row in subject.notedRows" :key="row.studentId" class="row is-calm">
                <span class="row__cls num">{{ classLabel(row) }}</span>
                <span class="row__no num">{{ row.number }}</span>
                <span class="row__name"><b>{{ row.name }}</b></span>
                <span class="row__what note">{{ row.homeroomNote }}</span>
                <span class="row__flag">읽기 전용</span>
            </div>
        </UiLedger>

        <UiNotice :text="subject.error" kind="error"/>

        <UiModal :open="Boolean(dropping)" title="이 차시를 지웁니다" @close="dropping = null">
            <div v-if="dropping" class="modal__what">
                <span class="modal__key">날짜</span>
                <span class="modal__val num">{{ dateLabel }}</span>
                <span class="modal__key">교시</span>
                <span class="modal__val"><b class="num">{{ dropping.slot }}교시</b></span>
                <span class="modal__key">빠진 사람</span>
                <span class="modal__val num">{{ dropping.absentCount }}명</span>
                <span class="modal__key">메모</span>
                <span class="modal__val">{{ dropping.memo || '—' }}</span>
            </div>
            <p class="modal__note">
                그 차시에 적은 결석도 함께 사라지고, 되돌릴 수 없습니다.
            </p>
            <template #foot>
                <UiButton size="wide" @click="dropping = null">취소</UiButton>
                <UiButton fill size="wide" variant="danger" @click="confirmDrop">지우기</UiButton>
            </template>
        </UiModal>
    </UiPage>
</template>

<style scoped>
/* 교시 버튼 줄. 장부 안쪽 여백을 행과 맞춘다. */
.pickline {
    padding: var(--s-lg) var(--s-2xl);
}

/* 교시 · 빠진 사람 · 메모 · 지우기. 칸 순서는 어느 줄에서나 같다. */
.list--sess .row {
    grid-template-columns: 120px 190px 1fr 46px;
}

/* 학년 · 반 · 번호 · 이름 · 결석 · 메모 · 무르기 */
.list--roll .row {
    grid-template-columns: 104px 48px 92px 92px 1fr 96px;
}

/* 학년 · 반 · 번호 · 이름 · 담임이 적은 문장 · 읽기 전용 표시 */
.list--note .row {
    grid-template-columns: 104px 48px 92px 1fr 108px;
}

/* 학적 칸. 번호와 같은 3차 위계다 — 누구인지 가리는 참고이지 주인공이 아니다. */
.row__cls {
    color: var(--c-ink-3);
}

/* 격자 · 범례가 장부 테두리에 닿지 않게 한다. */
.roll {
    display: flex;
    flex-direction: column;
    gap: var(--s-lg);
    padding: var(--s-lg) var(--s-2xl);
}

/* 격자 칸의 학적. 반이 섞이는 강좌라 번호만으로는 두 학생이 같은 `4`로 보인다. */
.seat__cls {
    color: var(--c-ink-3);
    letter-spacing: .02em;
}

/* 담임 기록이 있는 칸에 붙는 아래 띠. 자리는 언제나 남는다 —
 * 생겼다 사라지면 칸 높이가 줄마다 달라진다. */
.seat__note {
    width: 100%;
    height: 3px;
    border-radius: var(--r-xs);
    background: transparent;
}

.seat__note.is-on {
    background: var(--c-ink-3);
}

.legend .sw-note {
    background: var(--c-ink-3);
}

/* 할 일이 하나뿐인 화면의 단추 자리. 왼쪽에 붙여 다음 걸음이 어디인지 바로 보이게 한다. */
.lead {
    display: flex;
    gap: var(--s-md);
}

/* 담임이 적은 문장은 점선 안에 둔다. 내가 이 화면에서 찍은 줄과 눈으로 구별된다. */
.note {
    padding-left: var(--s-md);
    border-left: 2px dashed var(--c-line);
    color: var(--c-ink-2);
}
</style>
