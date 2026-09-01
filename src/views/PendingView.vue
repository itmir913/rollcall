<script setup>
import {onMounted, ref} from 'vue'
import {save} from '@tauri-apps/plugin-dialog'
import {useAppStore} from '../stores/app'
import {useCheckStore} from '../stores/check'
import {formatKorean} from '../stores/day'
import {UiButton, UiCard, UiNotice, UiPage, UiTable} from '../components/ui'

const app = useAppStore()
const check = useCheckStore()

const today = ref(new Date().toISOString().slice(0, 10))
const message = ref('')
const error = ref('')

const COLUMNS = [
    {key: 'number', label: '번호', width: '80px'},
    {key: 'name', label: '성명', width: '120px'},
    {key: 'date', label: '결석일', width: '160px'},
    {key: 'item', label: '항목'},
    {key: 'due', label: '마감일', width: '160px'},
    {key: 'state', label: '상태', width: '130px'},
    {key: 'contact', label: '연락처', width: '190px'},
]

async function load() {
    if (!app.ready) return
    error.value = ''
    try {
        await check.fetchPending(app.yearId, app.grade, app.classNo, today.value)
    } catch (e) {
        error.value = String(e)
    }
}

function overdueLabel(row) {
    if (row.daysOverdue === null) return '마감 없음'
    if (row.daysOverdue > 0) return `${row.daysOverdue}일 경과`
    if (row.daysOverdue === 0) return '오늘 마감'
    return `${-row.daysOverdue}일 남음`
}

async function exportCsv() {
    error.value = ''
    message.value = ''
    try {
        const dest = await save({
            defaultPath: `미제출자_${today.value}.csv`,
            filters: [{name: 'CSV', extensions: ['csv']}],
        })
        if (!dest) return
        await check.exportPendingCsv(app.yearId, app.grade, app.classNo, today.value, dest)
        message.value = `저장했습니다: ${dest}`
    } catch (e) {
        error.value = String(e)
    }
}

onMounted(load)
</script>

<template>
    <UiNotice v-if="!app.ready" kind="warn" text="먼저 설정에서 학년도와 학급을 정해주세요."/>

    <UiPage v-else :title="`미제출 ${check.pending.length}건`"
            subtitle="문자 발송 시스템이 학년·반·번호로 수신자를 찾으므로 CSV의 앞 세 열이 그것입니다.">
        <template #actions>
            <input v-model="today" class="field" type="date" @change="load"/>
            <UiButton variant="primary" @click="exportCsv">미제출자 CSV</UiButton>
        </template>

        <UiCard>
            <UiTable :columns="COLUMNS" :rows="check.pending"
                     empty-text="미제출 항목이 없습니다.">
                <template #row="{row}">
                    <td>{{ row.number }}</td>
                    <td>{{ row.name }}</td>
                    <td>{{ formatKorean(row.date) }}</td>
                    <td>{{ row.itemName }}</td>
                    <td>{{ row.dueDate ? formatKorean(row.dueDate) : '—' }}</td>
                    <td :class="row.daysOverdue > 0 ? 'is-overdue' : 'is-calm'">
                        {{ overdueLabel(row) }}
                    </td>
                    <td class="is-calm">
                        <template v-if="row.contactValue">
                            {{ row.contactLabel }} {{ row.contactValue }}
                        </template>
                        <template v-else>—</template>
                    </td>
                </template>
            </UiTable>
        </UiCard>

        <UiNotice :text="message" kind="ok"/>
        <UiNotice :text="error" kind="error"/>
    </UiPage>
</template>

<style scoped>
.is-overdue {
    color: var(--c-warn);
    font-weight: 600;
}

.is-calm {
    color: var(--c-ink-3);
}
</style>
