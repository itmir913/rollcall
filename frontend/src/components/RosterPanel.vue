<script setup>
/**
 * 명렬표 가져오기 — 파일에서 읽고, 차분을 보여주고, 확정본만 저장한다.
 *
 * **재가져오기는 교체가 아니라 차분이다.** 명단에서 빠진 번호는 지우지 않고 나간 날만
 * 적는다 — 지워 버리면 그 학생의 지난 출결이 함께 사라진다. 그것은 학교를 떠난 것
 * (전출)과 다른 일이라, 화면도 그렇게 적는다.
 *
 * **버린 줄은 조용히 넘기지 않는다.** 서른 명 중 스물아홉 명만 들어왔는데 아무 말이
 * 없으면 교사는 알 방법이 없다. 몇 번째 줄이 왜 빠졌는지 함께 보여준다.
 *
 * **명단이 붙는 곳은 `classId`가 정한다.** 비우면 지금 보고 있는 학급이다 — 설정은
 * 줄마다 다른 학급을 가리키고, 첫 실행은 방금 만든 학급에 차례로 넣는다. 파일이
 * 말하는 학년 · 반은 그 학생의 학적이지 소속이 아니라 이 자리를 정하지 않는다.
 *
 * **열쇠는 역할이 정한다.** 담임은 번호 하나가 열쇠이고, 교과 강좌는 (학년, 반, 번호)
 * 학적 자리 전체가 열쇠다 — 선택과목은 1반부터 n반까지 모여 3학년 1반 4번과 3학년
 * 6반 4번이 같은 강좌에 있다. 그래서 교과에서만 자리 칸과 반별 인원을 그린다.
 * 담임 화면에 `3학년 6반`이 서른 번 반복되면 읽는 데 방해만 된다.
 *
 * 설정과 첫 실행이 같은 것을 쓴다. 가져오기 규칙이 두 벌이 되면 한쪽에만 붙는다.
 */
import {computed, onMounted, ref, watch} from 'vue'
import {save} from '@tauri-apps/plugin-dialog'
import {useAppStore} from '../stores/app'
import {useDownloadStore} from '../stores/download'
import {useRosterStore} from '../stores/roster'
import {
    bufferToBase64,
    buildRosterWorkbook,
    buildSampleWorkbook,
    rosterRowsOf,
} from '../services/rosterFile'
import RosterImport from './RosterImport.vue'
import {UiButton, UiLedger, UiNotice} from './ui'

const props = defineProps({
    /** 명단이 붙을 학급. 비우면 지금 보고 있는 학급으로 간다. */
    classId: {type: Number, default: null},
})

const app = useAppStore()
const roster = useRosterStore()
const download = useDownloadStore()

/** 이 화면이 명단을 넣을 학급. 밖에서 정해 주지 않으면 지금 보고 있는 학급이다. */
const targetId = computed(() => props.classId ?? app.classId)
const target = computed(() => app.classes.find((c) => c.id === targetId.value) ?? null)

/** 교과 강좌인가. 열쇠도 화면도 이 하나로 갈린다. */
const isSubject = computed(() => target.value?.role === 'subject')

/**
 * 이 화면이 그리는 명단. 스토어의 `students`를 그대로 보지 않는 이유는, 같은 화면이
 * 학급마다 하나씩 뜰 수 있어 나중에 읽은 명단이 앞의 것을 덮기 때문이다.
 */
const students = ref([])
const entries = ref([])
const rows = ref([])
const parser = ref('')
const detected = ref(null)
/** 학년 · 반이 비어 위 줄에서 이어받은 줄 수. 조용히 바꾼 값은 알린다. */
const inherited = ref(0)
/** 학년 · 반 열이 통째로 없어 교과 강좌에서 멈춘 상태. */
const seatless = ref(false)
const effectiveDate = ref(app.today)
const message = ref('')
const warning = ref('')
const error = ref('')

/**
 * 줄마다 적히는 처리. **단추 글자가 곧 이 말이다.**
 *
 * `withdrawn`은 전출이 아니다 — 내 명단에서 빠지는 것과 학교를 떠나는 것은 다른 일이고,
 * 저장은 명단의 나간 날만 적는다.
 */
const ACTION_LABEL = {
    added: '새로 들어옴',
    unchanged: '그대로',
    renamed: '이름이 다름',
    withdrawn: '내 명단에서 뺌',
    linked: '명단에만 잇기',
    blocked: '넘김',
}

/** 줄의 왼쪽 띠. 상태를 색으로도 함께 말한다. */
const ACTION_TONE = {
    added: 'is-ok',
    unchanged: 'is-calm',
    renamed: 'is-bad',
    withdrawn: 'is-warn',
    linked: 'is-ok',
    blocked: 'is-warn',
}

/** 파일에서 온 줄. 빠진 줄(`withdrawn`)은 파일에 없으므로 인원에 세지 않는다. */
const FROM_FILE = ['added', 'unchanged', 'renamed', 'linked', 'blocked']

const counts = computed(() => {
    const out = {added: 0, unchanged: 0, renamed: 0, withdrawn: 0, linked: 0, blocked: 0}
    for (const row of rows.value) out[row.action] = (out[row.action] ?? 0) + 1
    return out
})

/** 머리에 적는 갈래별 수. 드문 갈래는 생겼을 때만 덧붙인다. */
const countNote = computed(() => {
    const c = counts.value
    const parts = [
        `새로 ${c.added}`,
        `그대로 ${c.unchanged}`,
        `이름 다름 ${c.renamed}`,
        `빠짐 ${c.withdrawn}`,
    ]
    if (c.linked) parts.push(`잇기 ${c.linked}`)
    if (c.blocked) parts.push(`넘김 ${c.blocked}`)
    return parts.join(' · ')
})

/**
 * 반별 인원. **교과 교사가 파일이 맞는지 판단하는 단위는 이름이 아니라 반별 인원이다** —
 * 서른 명의 이름을 훑는 것보다 `1반 4 · 2반 6`이 틀린 파일을 먼저 잡아낸다.
 */
const seatCounts = computed(() => {
    const groups = new Map()
    for (const row of rows.value) {
        if (!FROM_FILE.includes(row.action)) continue
        const key = `${row.grade ?? ''}/${row.classNo ?? ''}`
        const got = groups.get(key) ?? {grade: row.grade ?? null, classNo: row.classNo ?? null, n: 0}
        got.n += 1
        groups.set(key, got)
    }
    // 자리를 읽지 못한 묶음은 맨 뒤로 보낸다. 읽은 것부터 눈에 들어와야 한다.
    return [...groups.values()].sort(
        (a, b) => (a.grade ?? 1e9) - (b.grade ?? 1e9) || (a.classNo ?? 1e9) - (b.classNo ?? 1e9),
    )
})

const seatSummary = computed(() => {
    if (!isSubject.value || !seatCounts.value.length) return ''
    const parts = []
    let lastGrade = null
    for (const g of seatCounts.value) {
        if (g.classNo == null) {
            parts.push(`자리 모름 ${g.n}`)
            continue
        }
        // 같은 학년이 이어지면 학년을 다시 적지 않는다 — `3학년 1반 4 · 2반 6`.
        const head = g.grade != null && g.grade !== lastGrade ? `${g.grade}학년 ` : ''
        lastGrade = g.grade
        parts.push(`${head}${g.classNo}반 ${g.n}`)
    }
    const total = seatCounts.value.reduce((sum, g) => sum + g.n, 0)
    return `${parts.join(' · ')} — ${total}명`
})

/**
 * 자리를 읽지 못한 줄이 있다는 것. **그 줄은 짝 찾기에 참여하지 못한다** —
 * 그러면 그 줄이 가리키던 학생이 짝을 잃어 빠진 것으로 잡히므로, 저장이
 * 한 줄 때문에 명단에서 사람을 빼지 않도록 빠짐을 자동으로 표시하지 않는다.
 */
const blockedNote = computed(() => {
    const n = counts.value.blocked
    if (!n) return ''
    return `자리를 읽지 못한 줄이 ${n}개 있습니다. 그 줄이 가리키던 학생이 짝을 잃어 ` +
        '명단에서 빠지는 것을 막으려고, 빠짐은 자동으로 표시하지 않았습니다 — ' +
        '필요하면 그 줄의 단추를 눌러주세요.'
})

/** 위 줄에서 학년 · 반을 이어받은 줄. 파일을 고친 값이므로 알린다. */
const inheritedNote = computed(() =>
    inherited.value
        ? `학년 · 반이 비어 위 줄에서 이어받은 줄이 ${inherited.value}개 있습니다.`
        : '')

/**
 * 우리 반이 아닌 학생이 섞여 있는가. **막지 않고 알리기만 한다** — 다른 반 학생이
 * 담임 명렬표에 실리는 일이 실제로 있고, 프로그램이 강제하는 것은 기본 규칙까지다.
 *
 * **교과 강좌에는 띄우지 않는다.** 강좌는 애초에 여러 반에서 모이므로 늘 붙어 있는
 * 경고가 되고, 그러면 정작 담임 명렬표의 한 줄을 놓친다.
 */
const foreign = computed(() => {
    const cls = target.value
    if (!cls || cls.role !== 'homeroom' || cls.grade == null || cls.classNo == null) return ''

    const outsiders = entries.value.filter(
        (e) =>
            (e.grade != null && e.grade !== cls.grade) ||
            (e.classNo != null && e.classNo !== cls.classNo),
    ).length
    const {grade, classNo} = detected.value ?? {}
    const says =
        grade != null && classNo != null && (grade !== cls.grade || classNo !== cls.classNo)
            ? `파일은 ${grade}학년 ${classNo}반을 가리킵니다. `
            : ''
    if (!outsiders && !says) return ''

    const who = outsiders ? `${cls.name}이 아닌 학생 ${outsiders}명이 함께 들어왔습니다. ` : ''
    return `${says}${who}막지 않고 그대로 「${cls.name}」 명단에 넣습니다.`
})

/** 학적 자리. 교과에서만 그린다 — 어느 반 4번인지가 곧 그 학생이다. */
function seatOf(row) {
    if (row.grade == null || row.classNo == null) return '자리 모름'
    return `${row.grade}학년 ${row.classNo}반`
}

/**
 * 그 줄에 붙는 말. `blocked`의 이유는 **앱이 다시 해석하지 않고 그대로 적는다.**
 * 교과에서는 `4번 김하늘`이 두 줄 나란히 설 수 있으므로 파일 줄 번호를 함께 보인다.
 */
function noteOf(row) {
    if (row.action === 'renamed') return `${row.currentName} → ${row.incomingName}`
    // **`why`가 붙은 줄은 무엇이든 그 말을 적는다.** 담임이 남의 학적을 마감한다는
    // 저장 전 경고가 `added` 줄에 붙는데, 막힌 줄에만 적으면 그 경고가 한 글자도
    // 그려지지 않는다 — 되돌릴 수 없는 쓰기를 말없이 지나가게 된다.
    if (!row.why) return ''
    // 줄 번호는 **여기서만** 붙인다. Rust의 `why`는 문장만 담는다.
    const where = row.line != null ? `${row.line}번째 줄 — ` : ''
    return `${where}${row.why}`
}

/** 파일을 읽은 뒤. 어느 파서가 읽었는지와 버린 줄을 그대로 보여준다. */
async function onLoaded(result) {
    error.value = ''
    message.value = ''
    warning.value = ''
    seatless.value = false
    detected.value = null
    rows.value = []
    parser.value = result.parser ?? ''
    inherited.value = Number(result.inherited ?? 0)
    entries.value = result.entries ?? []

    // 학년 · 반 열이 통째로 없는 파일. 교과 강좌는 그 둘이 열쇠의 일부라 여기서 멈춘다 —
    // 서른 줄을 전부 같은 이유로 세우면 그것이 곧 늘 붙어 있는 경고가 된다.
    const missing = result.missing ?? []
    if (isSubject.value && missing.includes('grade') && missing.includes('classNo')) {
        seatless.value = true
        return
    }

    try {
        // **교과에서는 학급을 묻지 않는다.** 반이 섞인 것이 정상이라 값이 늘 `mixed`로
        // 오고, 그것을 읽는 자리가 생기면 교과에 늘 붙는 경고가 된다.
        if (!isSubject.value) detected.value = await roster.detectClass(entries.value)
        rows.value = await roster.preview(targetId.value, entries.value)
    } catch (e) {
        error.value = String(e)
    }
}

/**
 * 교사가 그 줄의 처리를 바꾼다. 프로그램이 판정하지 않는다.
 *
 * **단추는 줄마다 하나다.** 갈래마다 단추를 늘리면 서른 줄에서 무엇을 눌렀는지
 * 흐려진다. `blocked`는 넘김 ↔ 명단에만 잇기로 돈다 — 학적이 이미 있는 줄에서만
 * 이을 수 있고, 잇기는 학적을 건드리지 않고 명단에만 더한다.
 */
function toggleAction(row) {
    if (row.action === 'blocked') row.action = row.studentId ? 'linked' : 'blocked'
    else if (row.action === 'linked') row.action = 'blocked'
    else if (row.action === 'withdrawn') row.action = 'unchanged'
    else if (row.action === 'renamed') row.action = 'unchanged'
    else if (row.action === 'added') row.action = 'unchanged'
    else if (row.action === 'unchanged') row.action = row.studentId ? 'withdrawn' : 'added'
}

/** 더 고를 것이 없는 줄. 학적이 없는 `blocked`는 넘기는 것 말고 할 일이 없다. */
function locked(row) {
    return row.action === 'blocked' && !row.studentId
}

async function apply() {
    error.value = ''
    try {
        const result = await roster.apply(targetId.value, effectiveDate.value, rows.value)
        const added = result.added ?? 0
        const created = result.created ?? 0
        const seatClosed = result.seatClosed ?? 0
        rows.value = []
        entries.value = []

        const parts = [
            `명단에 새로 ${added}명`,
            `이름 고침 ${result.renamed ?? 0}명`,
            `내 명단에서 뺌 ${result.withdrawn ?? 0}명`,
        ]
        // 학적을 새로 만든 수는 파일이 맞는지 알려주는 값이라 생겼을 때 반드시 적는다.
        if (created) parts.push(`학적을 새로 만든 것 ${created}명`)
        if (result.blocked) parts.push(`앉히지 못해 넘긴 줄 ${result.blocked}개`)
        message.value = `${parts.join(' · ')}을 저장했습니다.`

        const alerts = []
        // 되돌릴 수 없는 쓰기가 일어난 유일한 자리다. 경고 위계로 말한다.
        if (seatClosed) {
            alerts.push(
                `그 자리에 있던 학적 ${seatClosed}건을 마감했습니다 — 되돌릴 수 없습니다.`,
            )
        }
        // 교과에서 들어온 전원의 학적을 새로 만들었다면 파일의 학년 · 반이 통째로 다르다.
        if (isSubject.value && added > 0 && created === added) {
            alerts.push(
                `들어온 ${added}명 전부의 학적을 새로 만들었습니다. ` +
                '파일의 학년 · 반이 통째로 다른지 확인해주세요.',
            )
        }
        warning.value = alerts.join(' ')

        await reload()
    } catch (e) {
        error.value = String(e)
    }
}

/**
 * 명렬표 양식을 내려받는다. **고칠 수단이 멈춘 자리에 함께 있어야 한다** —
 * 학년 · 반 열이 없어 멈춘 교사가 위로 올라가 다른 단추를 찾게 하지 않는다.
 */
async function downloadSample() {
    error.value = ''
    try {
        const path = await save({
            title: '명렬표 양식 저장',
            defaultPath: '명렬표_양식.xlsx',
            filters: [{name: '엑셀 파일', extensions: ['xlsx']}],
        })
        if (!path) return
        await download.saveBytes(path, bufferToBase64(await buildSampleWorkbook()))
        message.value = `양식을 저장했습니다: ${path}`
    } catch (e) {
        error.value = `양식을 저장하지 못했습니다: ${e}`
    }
}

/**
 * 지금 명단을 **자체 양식**으로 내보낸다.
 *
 * 이 앱이 언제나 읽고 쓸 수 있는 것은 이 양식 하나다 — 나이스 엑셀 가져오기는 그 위에
 * 얹는 어댑터다. 그래서 여기서 내보낸 파일은 위의 [명렬표 파일] 자리에 그대로 다시 넣을
 * 수 있다. 학교를 옮기든 컴퓨터를 바꾸든 명단은 이 파일로 따라간다.
 */
async function exportRoster() {
    error.value = ''
    message.value = ''
    try {
        const name = target.value?.name ?? '명단'
        const path = await save({
            title: '명렬표 내보내기',
            defaultPath: `${name} 명렬표.xlsx`,
            filters: [{name: '엑셀 파일', extensions: ['xlsx']}],
        })
        // 취소는 실패가 아니다. 아무 말도 하지 않는다.
        if (!path) return
        const buffer = await buildRosterWorkbook(rosterRowsOf(students.value))
        await download.saveBytes(path, bufferToBase64(buffer))
        message.value = `명렬표를 내보냈습니다 — ${path}`
    } catch (e) {
        error.value = `명렬표를 내보내지 못했습니다: ${e}`
    }
}

/**
 * 그 학급의 지금 명단을 읽는다. **늦게 온 응답은 버린다** — 학급을 옮긴 뒤에 앞의
 * 응답이 도착하면 머리글과 명단이 서로 다른 학급을 가리킨 채로 남는다.
 */
async function reload() {
    const id = targetId.value
    if (id == null) return
    const list = await roster.fetchStudents(id).catch(() => [])
    if (id === targetId.value) students.value = list ?? []
}

/** 학급이 바뀌면 읽던 것을 비운다. 앞 학급의 차분을 다음 학급에 저장할 수 없게 한다. */
watch(targetId, () => {
    rows.value = []
    entries.value = []
    detected.value = null
    parser.value = ''
    inherited.value = 0
    seatless.value = false
    message.value = ''
    warning.value = ''
    error.value = ''
    reload()
})

onMounted(reload)
</script>

<template>
    <div :class="['roster', isSubject ? 'roster--subject' : '']">
        <RosterImport @loaded="onLoaded"/>

        <UiNotice :text="error" kind="error"/>
        <UiNotice :text="message" kind="ok"/>

        <!-- 버린 줄은 RosterImport가 이유까지 적어 보여준다. 여기서 한 번 더 세면
             같은 말이 두 곳에 남아 한쪽만 고쳐진다. -->
        <UiNotice :text="foreign" kind="warn"/>
        <UiNotice :text="warning" kind="warn"/>
        <UiNotice :text="inheritedNote" kind="warn"/>
        <UiNotice :text="blockedNote" kind="warn"/>

        <!-- 학년 · 반 열이 통째로 없는 파일. 한 문장으로 멈추고 고칠 수단을 옆에 둔다. -->
        <div v-if="seatless" class="roster__stop">
            <span>
                교과 강좌는 학년 · 반 · 번호로 학생을 가리는데 이 파일에는 학년 · 반 열이
                없습니다 — 머리글에 학년과 반을 넣어 다시 가져와 주세요.
            </span>
            <UiButton @click="downloadSample">양식 내려받기</UiButton>
        </div>

        <UiLedger v-if="rows.length"
                  :hint="parser ? `${parser}로 읽음` : ''"
                  :note="countNote"
                  :title="target ? `${target.name}에 가져올 내용` : '가져올 내용'">
            <template #actions>
                <input v-model="effectiveDate" class="field num" type="date"/>
                <UiButton variant="primary" @click="apply">저장</UiButton>
            </template>

            <!-- 반별 인원이 먼저다. 교과 교사는 이름이 아니라 이 숫자로 파일을 확인한다. -->
            <div v-if="seatSummary" class="roster__seats">
                <span class="roster__seats-label">반별 인원</span>
                <span class="num">{{ seatSummary }}</span>
            </div>

            <div v-for="row in rows" :key="row.key"
                 :class="['row', ACTION_TONE[row.action] ?? 'is-calm']">
                <span v-if="isSubject" class="row__seat num">{{ seatOf(row) }}</span>
                <span class="row__no num">{{ row.number }}</span>
                <span class="row__name"><b>{{ row.incomingName ?? row.currentName }}</b></span>
                <span class="row__what">{{ noteOf(row) }}</span>
                <span class="row__acts">
                    <UiButton :disabled="locked(row)" size="tight" @click="toggleAction(row)">
                        {{ ACTION_LABEL[row.action] ?? row.action }}
                    </UiButton>
                </span>
            </div>

            <template #foot>
                <span>
                    명단에서 빠진 번호는 지우지 않고 나간 날만 적습니다 — 학적과 지난 출결은
                    그대로 남습니다. 학교를 떠난 것(전출)은 이것과 다른 일입니다.
                </span>
            </template>
        </UiLedger>

        <UiLedger v-else-if="students.length"
                  :note="`재학 ${students.length}명`"
                  :title="target ? `${target.name} 명단` : '지금 명단'">
            <template #actions>
                <UiButton @click="exportRoster">명렬표 내보내기</UiButton>
            </template>
            <div v-for="student in students" :key="student.id" class="row is-calm">
                <span v-if="isSubject" class="row__seat num">{{ seatOf(student) }}</span>
                <span class="row__no num">{{ student.number }}</span>
                <span class="row__name"><b>{{ student.name }}</b></span>
                <span class="row__what"></span>
                <span class="row__acts"></span>
            </div>
        </UiLedger>
    </div>
</template>

<style scoped>
.roster {
    display: flex;
    flex-direction: column;
    gap: var(--s-lg);
    width: 100%;
}

.roster .row {
    grid-template-columns: 48px 120px 1fr auto;
}

/* 교과 강좌는 번호 하나로 학생을 가릴 수 없다. 어느 반 4번인지 보이지 않으면
   같은 번호가 여럿인 목록을 눈으로 맞춰야 한다. */
.roster--subject .row {
    grid-template-columns: 124px 48px 120px 1fr auto;
}

.row__seat {
    color: var(--c-ink-2);
}

.roster__seats {
    display: flex;
    align-items: baseline;
    gap: var(--s-md);
    padding: var(--s-md) var(--s-2xl);
    border-bottom: 1px solid var(--c-line-soft);
    color: var(--c-ink-2);
}

.roster__seats-label {
    color: var(--c-ink-3);
}

/* 멈춘 자리. 고칠 수단(양식 내려받기)이 이 상자 안에 함께 있다. */
.roster__stop {
    display: flex;
    align-items: center;
    gap: var(--s-lg);
    flex-wrap: wrap;
    padding: var(--s-lg) var(--s-xl);
    border: 1px solid var(--c-warn);
    border-radius: var(--r-md);
    color: var(--c-warn);
}
</style>
