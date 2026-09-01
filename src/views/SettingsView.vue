<script setup>
import {onMounted, ref} from 'vue'
import {save} from '@tauri-apps/plugin-dialog'
import {useAppStore} from '../stores/app'
import {useCheckStore} from '../stores/check'
import {useCodeStore} from '../stores/code'
import {useTheme} from '../composables/useTheme'
import {UiButton, UiCard, UiNotice, UiPage, UiTable, UiToggle} from '../components/ui'

const app = useAppStore()
const check = useCheckStore()
const codeStore = useCodeStore()
const theme = useTheme()

const THEME_LABELS = {light: '라이트 (기본)', dark: '다크', system: '시스템 따름'}

const newItem = ref({name: '', dueDays: null, includeWeekend: false, defaultDone: false})
const message = ref('')
const error = ref('')

const ITEM_COLUMNS = [
    {key: 'name', label: '이름'},
    {key: 'due', label: '마감 일수', width: '120px'},
    {key: 'weekend', label: '주말 포함', width: '150px'},
    {key: 'default', label: '기본값', width: '150px'},
    {key: 'actions', label: '', width: '180px', align: 'right'},
]

const CODE_COLUMNS = [
    {key: 'label', label: '코드', width: '140px'},
    {key: 'reason', label: '사유', width: '110px'},
    {key: 'type', label: '유형', width: '90px'},
    {key: 'shortcut', label: '단축키', width: '90px'},
    {key: 'pattern', label: '문구 패턴'},
]

async function load() {
    await Promise.all([check.fetchItems(false), codeStore.fetchCodes()])
}

function toNumberOrNull(value) {
    return value === null || value === '' ? null : Number(value)
}

async function addItem() {
    error.value = ''
    try {
        await check.createItem({
            name: newItem.value.name,
            dueDays: toNumberOrNull(newItem.value.dueDays),
            includeWeekend: newItem.value.includeWeekend,
            defaultDone: newItem.value.defaultDone,
            sortOrder: (check.items.length + 1) * 10,
        })
        newItem.value = {name: '', dueDays: null, includeWeekend: false, defaultDone: false}
        message.value = '항목을 추가했습니다. HOME 격자에 열이 하나 늘어납니다.'
    } catch (e) {
        error.value = String(e)
    }
}

async function saveItem(item) {
    error.value = ''
    try {
        await check.updateItem({
            id: item.id,
            name: item.name,
            dueDays: toNumberOrNull(item.dueDays),
            includeWeekend: item.includeWeekend,
            defaultDone: item.defaultDone,
            sortOrder: item.sortOrder,
        })
        message.value = '저장했습니다. 이미 저장된 마감일은 그대로 둡니다.'
    } catch (e) {
        error.value = String(e)
    }
}

async function deactivate(item) {
    error.value = ''
    try {
        await check.deactivateItem(item.id)
    } catch (e) {
        error.value = String(e)
    }
}

async function pickPath(defaultPath, filter) {
    return await save({defaultPath, filters: [filter]})
}

async function backup() {
    error.value = ''
    message.value = ''
    try {
        const stamp = new Date().toISOString().slice(0, 10)
        const dest = await pickPath(`rollcall-${stamp}.db`, {name: '데이터 파일', extensions: ['db']})
        if (!dest) return
        await app.backupTo(dest)
        message.value = `백업했습니다: ${dest}`
    } catch (e) {
        error.value = String(e)
    }
}

async function backupCsv() {
    error.value = ''
    message.value = ''
    try {
        const stamp = new Date().toISOString().slice(0, 10)
        const dest = await pickPath(`출결전체_${stamp}.csv`, {name: 'CSV', extensions: ['csv']})
        if (!dest) return
        await check.exportBackupCsv(app.yearId, app.grade, app.classNo, dest)
        message.value = `저장했습니다: ${dest}`
    } catch (e) {
        error.value = String(e)
    }
}

onMounted(load)
</script>

<template>
    <UiPage title="설정">
        <UiCard description="기본은 라이트입니다. '시스템 따름'을 고르면 윈도우 설정을 따라갑니다. 이 설정은 이 PC에만 저장되며 백업에 들어가지 않습니다."
                title="화면">
            <div class="settings__row">
                <UiButton v-for="m in theme.modes" :key="m"
                          :variant="theme.mode.value === m ? 'primary' : 'default'"
                          @click="theme.setMode(m)">
                    {{ THEME_LABELS[m] }}
                </UiButton>
            </div>
        </UiCard>

        <UiCard description="항목을 추가하면 HOME 격자에 열이 하나 붙습니다. 삭제 대신 비활성으로 두는 것은, 지우면 과거 체크 기록이 함께 사라지기 때문입니다."
                title="체크 항목">
            <UiTable :columns="ITEM_COLUMNS" :rows="check.items">
                <template #row="{row}">
                    <td><input v-model="row.name" class="field settings__grow"/></td>
                    <td>
                        <input v-model="row.dueDays" class="field settings__num" min="0"
                               placeholder="없음" type="number"/>
                    </td>
                    <td>
                        <UiToggle v-model="row.includeWeekend" block off-label="평일만"
                                  on-label="주말 포함"/>
                    </td>
                    <td>
                        <UiToggle v-model="row.defaultDone" block off-label="미완료"
                                  on-label="완료"/>
                    </td>
                    <td class="settings__cell-right">
                        <UiButton @click="saveItem(row)">저장</UiButton>
                        <UiButton v-if="row.active" variant="danger" @click="deactivate(row)">
                            비활성
                        </UiButton>
                    </td>
                </template>
            </UiTable>

            <div class="settings__row">
                <input v-model="newItem.name" class="field" placeholder="새 항목 이름"/>
                <input v-model="newItem.dueDays" class="field settings__num" min="0"
                       placeholder="마감 일수" type="number"/>
                <UiToggle v-model="newItem.includeWeekend" off-label="평일만" on-label="주말 포함"/>
                <UiToggle v-model="newItem.defaultDone" off-label="미완료" on-label="완료"/>
                <UiButton :disabled="!newItem.name.trim()" variant="primary" @click="addItem">
                    추가
                </UiButton>
            </div>
        </UiCard>

        <UiCard description="코드를 고치면 과거 기록의 뜻이 바뀌므로, 수정은 구 코드를 마감하고 새 코드를 추가하는 방식으로 처리됩니다."
                title="출결 코드">
            <UiTable :columns="CODE_COLUMNS" :rows="codeStore.codes">
                <template #row="{row}">
                    <td>{{ row.label }}</td>
                    <td>{{ row.reason }}</td>
                    <td>{{ row.type }}</td>
                    <td>{{ row.shortcut ?? '—' }}</td>
                    <td class="settings__muted">{{ row.phrasePattern ?? '—' }}</td>
                </template>
            </UiTable>
        </UiCard>

        <UiCard title="백업">
            <p class="settings__muted">
                데이터 파일: {{ app.status?.path }}<br/>
                파일을 열 때마다 같은 폴더에 백업본이 만들어집니다. 프로그램은 그 파일들을
                스캔하지도 지우지도 않으니 가끔 직접 정리해주세요.
            </p>
            <div class="settings__row">
                <UiButton @click="backup">데이터 파일 백업</UiButton>
                <UiButton @click="backupCsv">전체 CSV 내보내기</UiButton>
            </div>
        </UiCard>

        <UiNotice :text="message" kind="ok"/>
        <UiNotice :text="error" kind="error"/>
    </UiPage>
</template>

<style scoped>
.settings__row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
}

.settings__num {
    width: 110px;
}

.settings__grow {
    width: 100%;
}

.settings__cell-right {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
}

.settings__muted {
    color: var(--c-ink-3);
    margin: 0;
    line-height: 1.6;
}
</style>
