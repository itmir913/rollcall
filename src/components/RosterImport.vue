<script setup>
/**
 * 명렬표 파일 가져오기. 끌어다 놓거나 골라서 연다.
 *
 * 붙여넣기 방식을 쓰지 않는다. 탭인지 쉼표인지, 이름에 쉼표가 들어갔는지를
 * 텍스트만으로는 확실히 알 수 없다. 파일에는 그 정보가 들어 있다.
 *
 * 여기서 하는 일은 **파일을 읽어 목록을 만드는 것까지**다. 그 목록을 어떻게
 * 반영할지(추가·전출·개명)는 미리보기 화면과 Rust가 정한다.
 */
import {ref} from 'vue'
import {save} from '@tauri-apps/plugin-dialog'
import {invoke} from '@tauri-apps/api/core'
import {
    bufferToBase64,
    buildSampleWorkbook,
    readRosterFile,
    SUPPORTED_EXTENSIONS,
} from '../services/rosterFile'
import {COL_LABELS} from '../data/columnAliases'
import {UiButton, UiNotice} from './ui'

const emit = defineEmits(['loaded'])

const dragging = ref(false)
const busy = ref(false)
const fileName = ref('')
const error = ref('')
const notice = ref('')
const skipped = ref([])
const fileInput = ref(null)

const ACCEPT = SUPPORTED_EXTENSIONS.map((e) => `.${e}`).join(',')

async function handleFile(file) {
    if (!file) return
    busy.value = true
    error.value = ''
    notice.value = ''
    skipped.value = []
    fileName.value = file.name

    try {
        const result = await readRosterFile(file)
        skipped.value = result.skipped

        const parts = [`${result.entries.length}명을 읽었습니다.`]
        if (result.missing.length) {
            const labels = result.missing.map((c) => COL_LABELS[c]).join(' · ')
            parts.push(`${labels} 열이 없어 학급은 아래에서 정해주세요.`)
        }
        // 어느 파서가 읽었는지 알린다. 폴백이 쓰였다면 파일이 표준에서 벗어났다는 뜻이고,
        // 값이 이상할 때 어디를 의심할지 알려주는 단서가 된다.
        if (result.parser === 'sheetjs') {
            parts.push('(표준에서 벗어난 파일이라 보조 파서로 읽었습니다)')
        }
        notice.value = parts.join(' ')

        emit('loaded', result)
    } catch (e) {
        error.value = e.message ?? String(e)
    } finally {
        busy.value = false
    }
}

function onDrop(event) {
    dragging.value = false
    handleFile(event.dataTransfer?.files?.[0])
}

function onPick(event) {
    handleFile(event.target.files?.[0])
    // 같은 파일을 고쳐서 다시 고를 수 있어야 한다. 값을 비우지 않으면 change가 안 뜬다.
    event.target.value = ''
}

async function downloadSample() {
    error.value = ''
    try {
        const path = await save({
            title: '명렬표 양식 저장',
            defaultPath: '명렬표_양식.xlsx',
            filters: [{name: '엑셀 파일', extensions: ['xlsx']}],
        })
        if (!path) return
        const buffer = await buildSampleWorkbook()
        await invoke('write_bytes_file', {path, data: bufferToBase64(buffer)})
        notice.value = `양식을 저장했습니다: ${path}`
    } catch (e) {
        error.value = `양식을 저장하지 못했습니다: ${e}`
    }
}
</script>

<template>
    <div class="import">
        <div :class="['drop', dragging ? 'is-over' : '', busy ? 'is-busy' : '']"
             @click="fileInput?.click()"
             @dragleave.prevent="dragging = false"
             @dragover.prevent="dragging = true"
             @drop.prevent="onDrop">
            <p class="drop__title">
                {{ busy ? '읽는 중…' : '명렬표 파일을 여기에 끌어다 놓으세요' }}
            </p>
            <p class="drop__sub">
                엑셀(.xlsx) · CSV — 또는 눌러서 고르기
            </p>
            <p v-if="fileName" class="drop__file">{{ fileName }}</p>
            <input ref="fileInput" :accept="ACCEPT" class="drop__input" type="file"
                   @change="onPick"/>
        </div>

        <div class="import__row">
            <UiButton @click.stop="downloadSample">양식 내려받기</UiButton>
            <span class="import__hint">
                머리글은 <b>학년 · 반 · 번호 · 이름</b>입니다. 열 순서는 상관없고,
                모르는 열은 무시합니다.
            </span>
        </div>

        <UiNotice :text="notice" kind="ok"/>
        <UiNotice :text="error" kind="error"/>

        <div v-if="skipped.length" class="skipped">
            <p class="skipped__title">건너뛴 줄 {{ skipped.length }}개</p>
            <ul class="skipped__list">
                <li v-for="s in skipped" :key="s.line">{{ s.line }}번째 줄 — {{ s.reason }}</li>
            </ul>
        </div>
    </div>
</template>

<style scoped>
.import {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.drop {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 32px 20px;
    border: 2px dashed var(--c-line);
    border-radius: 12px;
    background: var(--c-raised);
    cursor: pointer;
    text-align: center;
    transition: border-color 120ms ease, background 120ms ease;
}

.drop:hover,
.drop.is-over {
    border-color: var(--c-accent);
}

.drop.is-over {
    background: color-mix(in srgb, var(--c-accent) 8%, var(--c-raised));
}

.drop.is-busy {
    opacity: 0.6;
    cursor: progress;
}

.drop__title {
    margin: 0;
    font-weight: 600;
}

.drop__sub {
    margin: 0;
    color: var(--c-ink-3);
}

.drop__file {
    margin: 4px 0 0;
    color: var(--c-accent);
}

.drop__input {
    display: none;
}

.import__row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
}

.import__hint {
    color: var(--c-ink-3);
}

.skipped {
    padding: 12px 14px;
    border-radius: 8px;
    border: 1px solid var(--c-warn);
    color: var(--c-warn);
}

.skipped__title {
    margin: 0 0 6px;
    font-weight: 600;
}

.skipped__list {
    margin: 0;
    padding-left: 18px;
    line-height: 1.6;
}
</style>
