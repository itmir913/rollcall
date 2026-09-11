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
 * **명단이 붙는 곳은 지금 고른 학급(`classId`)이다.** 파일이 말하는 학년 · 반은
 * 그 학생의 학적이지 소속이 아니라, 어느 명단에 넣을지를 정하지 않는다. 다른 반을
 * 가리키는 파일도 막지 않고 **알리기만 한다** — 막으면 현장의 예외를 담을 수 없다.
 *
 * 설정과 첫 실행이 같은 것을 쓴다. 가져오기 규칙이 두 벌이 되면 한쪽에만 붙는다.
 */
import {computed, onMounted, ref} from 'vue'
import {useAppStore} from '../stores/app'
import {useRosterStore} from '../stores/roster'
import RosterImport from './RosterImport.vue'
import {UiButton, UiLedger, UiNotice} from './ui'

const app = useAppStore()
const roster = useRosterStore()

const rows = ref([])
const skipped = ref([])
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
 * 파일이 가리키는 학급이 지금 학급과 다른가. 막지 않고 알리기만 한다 —
 * 다른 반 학생이 우리 반 명단에 실리는 일이 실제로 있고, 그 판단은 교사 몫이다.
 */
const mismatch = computed(() => {
    const cls = app.currentClass
    const {grade, classNo} = detected.value ?? {}
    if (!cls || grade == null || classNo == null) return ''
    if (grade === cls.grade && classNo === cls.classNo) return ''
    return `파일은 ${grade}학년 ${classNo}반을 가리킵니다 — 「${cls.name}」 명단으로 들어갑니다.`
})

/** 파일을 읽은 뒤. 어느 파서가 읽었는지와 버린 줄을 그대로 보여준다. */
async function onLoaded(result) {
    error.value = ''
    message.value = ''
    parser.value = result.parser ?? ''
    skipped.value = result.skipped ?? []
    try {
        detected.value = await roster.detectClass(result.entries)
        rows.value = await roster.preview(app.classId, result.entries)
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
        const result = await roster.apply(app.classId, effectiveDate.value, rows.value)
        rows.value = []
        message.value =
            `새로 ${result.added}명, 이름 고침 ${result.renamed}명, 전출 ${result.withdrawn}명을 저장했습니다.`
    } catch (e) {
        error.value = String(e)
    }
}

onMounted(() => {
    if (app.ready) {
        roster.fetchStudents(app.classId).catch(() => {
        })
    }
})
</script>

<template>
    <div class="roster">
        <RosterImport @loaded="onLoaded"/>

        <UiNotice :text="error" kind="error"/>
        <UiNotice :text="message" kind="ok"/>

        <UiNotice v-if="skipped.length"
                  :text="`읽지 못한 줄이 있습니다 — ${skipped.map((s) => `${s.line}번째 줄(${s.why})`).join(', ')}`"
                  kind="warn"/>

        <UiNotice :text="mismatch" kind="warn"/>

        <UiLedger v-if="rows.length"
                  :hint="parser ? `${parser}로 읽음` : ''"
                  :note="`새로 ${counts.added} · 그대로 ${counts.unchanged} · 이름 다름 ${counts.renamed} · 전출 ${counts.withdrawn}`"
                  title="가져올 내용">
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

        <UiLedger v-else-if="roster.students.length"
                  :note="`재학 ${roster.students.length}명`" title="지금 명단">
            <div v-for="student in roster.students" :key="student.id" class="row is-calm">
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
