/**
 * CSV 내보내기. **문자열은 Rust가 만들고, 파일로 저장하는 것만 여기서 한다.**
 *
 * 앞 세 열이 학년 · 반 · 번호인 것도, UTF-8 BOM을 붙이는 것도 Rust가 정한다 —
 * 문자 발송 시스템이 그 세 열로 수신자를 찾고, BOM이 없으면 엑셀에서 한글이 깨진다.
 * 화면이 그 규칙을 알면 내보내기가 화면마다 달라진다.
 */
import {invoke} from '@tauri-apps/api/core'
import {save} from '@tauri-apps/plugin-dialog'

/** Rust의 write_bytes_file은 base64 문자열을 받는다. 바이트를 그대로 넘기지 않는다. */
function toBase64(text) {
    const bytes = new TextEncoder().encode(text)
    let binary = ''
    for (const byte of bytes) binary += String.fromCharCode(byte)
    return btoa(binary)
}

const COMMANDS = {
    spans: 'export_spans_csv',
    pending: 'export_pending_csv',
    quota: 'export_quota_csv',
}

/**
 * @param {'spans'|'pending'|'quota'} kind 무엇을 내보내는가
 * @param {object} args 커맨드 인자
 * @param {string} suggested 기본 파일 이름
 * @returns {Promise<string|null>} 저장한 경로. 교사가 취소하면 null.
 */
export async function exportCsv(kind, args, suggested) {
    const command = COMMANDS[kind]
    if (!command) throw new Error(`알 수 없는 내보내기입니다: ${kind}`)

    const csv = await invoke(command, args)
    const path = await save({
        defaultPath: suggested,
        filters: [{name: 'CSV', extensions: ['csv']}],
    })
    if (!path) return null

    await invoke('write_bytes_file', {path, data: toBase64(csv)})
    return path
}
