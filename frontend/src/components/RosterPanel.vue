<script setup>
/**
 * 명렬표 가져오기 — 파일에서 읽고, 차분을 보여주고, 확정본만 저장한다.
 *
 * **재가져오기는 교체가 아니라 차분이다.** 사라진 번호는 삭제하지 않고 전출로 남긴다 —
 * 지워 버리면 그 학생의 지난 출결이 함께 사라진다.
 *
 * **버린 줄은 조용히 넘기지 않는다.** 서른 명 중 스물아홉 명만 들어왔는데 아무 말이
 * 없으면 교사는 알 방법이 없다. 몇 번째 줄이 왜 빠졌는지 함께 보여준다.
 *
 * **명단이 붙는 곳은 `classId`가 정한다.** 비우면 지금 보고 있는 학급이다 — 설정은
 * 줄마다 다른 학급을 가리키고, 첫 실행은 방금 만든 학급에 차례로 넣는다. 파일이
 * 말하는 학년 · 반은 그 학생의 학적이지 소속이 아니라 이 자리를 정하지 않는다.
 *
 * 설정과 첫 실행이 같은 것을 쓴다. 가져오기 규칙이 두 벌이 되면 한쪽에만 붙는다.
 */
import {computed, onMounted, ref, watch} from 'vue'
import {useAppStore} from '../stores/app'
import {useRosterStore} from '../stores/roster'
import RosterImport from './RosterImport.vue'
import {UiButton, UiLedger, UiNotice} from './ui'

const props = defineProps({
    /** 명단이 붙을 학급. 비우면 지금 보고 있는 학급으로 간다. */
    classId: {type: Number, default: null},
})

const app = useAppStore()
const roster = useRosterStore()

/** 이 화면이 명단을 넣을 학급. 밖에서 정해 주지 않으면 지금 보고 있는 학급이다. */
const targetId = computed(() => props.classId ?? app.classId)
const target = computed(() => app.classes.find((c) => c.id === targetId.value) ?? null)

/**
 * 이 화면이 그리는 명단. 스토어의 `students`를 그대로 보지 않는 이유는, 같은 화면이
 * 학급마다 하나씩 뜰 수 있어 나중에 읽은 명단이 앞의 것을 덮기 때문이다.
 */
const students = ref([])
const entries = ref([])
const rows = ref([])
const parser = ref('')
const detected = ref(null)
const effectiveDate = ref(app.today)
const message = ref('')
const error = ref('')

const ACTION_LABEL = {
    added: '새로 들어옴',
    unchanged: '그대로',
    renamed: '이름이 다름',
    withdrawn: '명단에서 사라짐 (전출)',
}

const counts = computed(() => {
    const out = {added: 0, unchanged: 0, renamed: 0, withdrawn: 0}
    for (const row of rows.value) out[row.action] = (out[row.action] ?? 0) + 1
    return out
})

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

/** 파일을 읽은 뒤. 어느 파서가 읽었는지와 버린 줄을 그대로 보여준다. */
async function onLoaded(result) {
    error.value = ''
    message.value = ''
    parser.value = result.parser ?? ''
    entries.value = result.entries ?? []
    try {
        detected.value = await roster.detectClass(entries.value)
        rows.value = await roster.preview(targetId.value, entries.value)
    } catch (e) {
        error.value = String(e)
    }
}

/** 교사가 그 줄의 처리를 바꾼다. 프로그램이 판정하지 않는다. */
function toggleAction(row) {
    if (row.action === 'withdrawn') row.action = 'unchanged'
    else if (row.action === 'unchanged' && row.studentId) row.action = 'withdrawn'
    else if (row.action === 'renamed') row.action = 'unchanged'
    else if (row.action === 'unchanged' && !row.studentId) row.action = 'added'
    else if (row.action === 'added') row.action = 'unchanged'
}

async function apply() {
    error.value = ''
    try {
        const result = await roster.apply(targetId.value, effectiveDate.value, rows.value)
        rows.value = []
        entries.value = []
        message.value =
            `새로 ${result.added}명, 이름 고침 ${result.renamed}명, 전출 ${result.withdrawn}명을 저장했습니다.`
        await reload()
    } catch (e) {
        error.value = String(e)
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
    message.value = ''
    error.value = ''
    reload()
})

onMounted(reload)
</script>

<template>
    <div class="roster">
        <RosterImport @loaded="onLoaded"/>

        <UiNotice :text="error" kind="error"/>
        <UiNotice :text="message" kind="ok"/>

        <!-- 버린 줄은 RosterImport가 이유까지 적어 보여준다. 여기서 한 번 더 세면
             같은 말이 두 곳에 남아 한쪽만 고쳐진다. -->
        <UiNotice :text="foreign" kind="warn"/>

        <UiLedger v-if="rows.length"
                  :hint="parser ? `${parser}로 읽음` : ''"
                  :note="`새로 ${counts.added} · 그대로 ${counts.unchanged} · 이름 다름 ${counts.renamed} · 전출 ${counts.withdrawn}`"
                  :title="target ? `${target.name}에 가져올 내용` : '가져올 내용'">
            <template #actions>
                <input v-model="effectiveDate" class="field num" type="date"/>
                <UiButton variant="primary" @click="apply">저장</UiButton>
            </template>

            <div v-for="row in rows" :key="row.number"
                 :class="['row', 'list--roster',
                          row.action === 'withdrawn' ? 'is-warn'
                          : row.action === 'added' ? 'is-ok'
                          : row.action === 'renamed' ? 'is-bad' : 'is-calm']">
                <span class="row__no num">{{ row.number }}</span>
                <span class="row__name"><b>{{ row.incomingName ?? row.currentName }}</b></span>
                <span class="row__what">
                    {{ row.action === 'renamed' ? `${row.currentName} → ${row.incomingName}` : '' }}
                </span>
                <span class="row__acts">
                    <UiButton size="tight" @click="toggleAction(row)">
                        {{ ACTION_LABEL[row.action] }}
                    </UiButton>
                </span>
            </div>

            <template #foot>
                <span>사라진 번호는 지우지 않고 전출로 남깁니다 — 지난 출결이 함께 사라지지 않도록.</span>
            </template>
        </UiLedger>

        <UiLedger v-else-if="students.length"
                  :note="`재학 ${students.length}명`"
                  :title="target ? `${target.name} 명단` : '지금 명단'">
            <div v-for="student in students" :key="student.id" class="row is-calm">
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
</style>
