/**
 * 명렬표 파일(xlsx · csv)을 읽어 `RosterEntry` 목록으로 바꾼다.
 *
 * 붙여넣기 방식을 쓰지 않는 이유는 구분자 때문이다. 탭인지 쉼표인지, 이름에
 * 쉼표가 들어갔는지를 텍스트만 보고는 확실히 알 수 없다. 파일에는 그 정보가
 * 들어 있다.
 *
 * 파서는 둘이다. **exceljs로 먼저 읽고, 실패하면 SheetJS로 다시 읽는다.**
 * 한셀이나 나이스가 내보낸 xlsx는 규격에서 조금씩 벗어나 exceljs가 거부하는
 * 경우가 있는데, SheetJS는 그런 파일도 대체로 읽어낸다. 반대로 SheetJS만 쓰면
 * 서식이 복잡한 정상 파일에서 값이 달라지는 경우가 있어 순서를 이렇게 둔다.
 *
 * 이 모듈은 **파일 형식을 다룰 뿐 업무 규칙을 다루지 않는다.** 명단 차분(추가·
 * 전출·개명 판정)과 저장은 전부 Rust가 한다.
 */
import {Workbook} from 'exceljs'
import * as XLSX from 'xlsx'
import {COL_LABELS, matchColumn, REQUIRED_COLS} from '../data/columnAliases'

/** 헤더 줄을 찾을 때 훑어볼 최대 행 수. 제목 줄이 앞에 붙은 파일이 흔하다. */
const HEADER_SCAN_ROWS = 10

export const SUPPORTED_EXTENSIONS = ['xlsx', 'csv']

/**
 * xlsx는 zip이라 항상 PK 네 바이트로 시작한다.
 *
 * 이 검사가 없으면 SheetJS가 아무 텍스트나 시트로 "읽어내" 깨진 글자가 담긴
 * 표를 돌려준다. 교사에게는 파일이 잘못됐다는 사실 대신 알 수 없는 머리글
 * 오류가 보인다.
 */
export function looksLikeZip(buffer) {
    const b = new Uint8Array(buffer)
    return b.length >= 4 && b[0] === 0x50 && b[1] === 0x4b && b[2] === 0x03 && b[3] === 0x04
}

export function extensionOf(fileName) {
    return String(fileName ?? '').split('.').pop().toLowerCase()
}

// ── 셀 값 ─────────────────────────────────────────────────────

/** exceljs 셀은 수식·서식 있는 글자·하이퍼링크 등 객체로 올 수 있다. */
export function cellText(value) {
    if (value === null || value === undefined) return ''
    if (value instanceof Date) return value.toISOString().slice(0, 10)
    if (typeof value === 'object') {
        if (Array.isArray(value.richText)) return value.richText.map((r) => r.text).join('')
        if (value.text !== undefined) return String(value.text)
        if (value.result !== undefined) return String(value.result) // 수식
        if (value.hyperlink !== undefined) return String(value.text ?? '')
        return ''
    }
    return String(value)
}

// ── CSV ───────────────────────────────────────────────────────

/**
 * CSV 바이트를 글자로 푼다.
 *
 * 엑셀이 저장한 한국어 CSV는 CP949(euc-kr)인 경우가 많다. UTF-8로만 읽으면
 * 이름이 전부 깨진 채 조용히 들어간다. BOM → UTF-8 → euc-kr 순서로 시도한다.
 */
export function decodeCsvBytes(buffer) {
    const bytes = new Uint8Array(buffer)
    if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
        return new TextDecoder('utf-8').decode(bytes.subarray(3))
    }
    for (const encoding of ['utf-8', 'euc-kr']) {
        try {
            return new TextDecoder(encoding, {fatal: true}).decode(bytes)
        } catch { /* 다음 인코딩으로 */ }
    }
    return new TextDecoder('utf-8').decode(bytes)
}

/**
 * RFC 4180 CSV. 따옴표 안의 줄바꿈과 쉼표를 지킨다.
 *
 * 줄 단위로 먼저 자르면 `"홍길동, 김"` 같은 값이나 셀 안 줄바꿈에서 깨진다.
 * 그래서 글자를 하나씩 본다.
 */
export function parseCsv(text) {
    const rows = []
    let row = []
    let field = ''
    let inQuotes = false
    const source = text.replace(/\r\n/g, '\n').replace(/\r/g, '\n')

    for (let i = 0; i < source.length; i++) {
        const ch = source[i]
        if (inQuotes) {
            if (ch === '"' && source[i + 1] === '"') {
                field += '"'
                i++
            } else if (ch === '"') {
                inQuotes = false
            } else {
                field += ch
            }
        } else if (ch === '"') {
            inQuotes = true
        } else if (ch === ',') {
            row.push(field)
            field = ''
        } else if (ch === '\n') {
            row.push(field)
            rows.push(row)
            row = []
            field = ''
        } else {
            field += ch
        }
    }
    row.push(field)
    rows.push(row)

    return rows.filter((r) => r.some((c) => String(c).trim() !== ''))
}

// ── 시트 → 행 ─────────────────────────────────────────────────

async function rowsWithExcelJs(buffer) {
    const workbook = new Workbook()
    await workbook.xlsx.load(buffer)
    const sheet = workbook.worksheets[0]
    if (!sheet) throw new Error('시트가 없습니다.')
    const rows = []
    sheet.eachRow((row) => {
        // row.values는 1-based라 첫 칸이 비어 있다.
        rows.push(row.values.slice(1).map(cellText))
    })
    return rows
}

function rowsWithSheetJs(buffer) {
    const workbook = XLSX.read(buffer, {type: 'array'})
    const name = workbook.SheetNames[0]
    if (!name) throw new Error('시트가 없습니다.')
    const raw = XLSX.utils.sheet_to_json(workbook.Sheets[name], {header: 1, defval: ''})
    return raw.map((row) => row.map(cellText))
}

/**
 * 엑셀 파일을 행 배열로. exceljs → SheetJS 순서로 시도한다.
 * 어느 파서가 읽었는지 함께 돌려준다 — 폴백이 쓰였다는 사실은 화면에 알린다.
 */
export async function readSheetRows(buffer) {
    try {
        const rows = await rowsWithExcelJs(buffer)
        if (rows.length) return {rows, parser: 'exceljs'}
        // 빈 결과는 실패로 본다. 정상 파일이라면 헤더 한 줄은 나온다.
        throw new Error('읽어낸 행이 없습니다.')
    } catch (first) {
        try {
            return {rows: rowsWithSheetJs(buffer), parser: 'sheetjs'}
        } catch (second) {
            throw new Error(
                `엑셀 파일을 읽지 못했습니다. (exceljs: ${first.message} / SheetJS: ${second.message})`,
            )
        }
    }
}

// ── 헤더 → 열 ─────────────────────────────────────────────────

/** 헤더 한 줄을 열 이름 → 위치로 바꾼다. 모르는 열은 버린다. */
export function mapHeaderRow(headerRow) {
    const map = {}
    headerRow.forEach((cell, index) => {
        const col = matchColumn(cell)
        // 같은 열이 두 번 나오면 앞의 것을 쓴다. 뒤엣것은 대개 비고란이다.
        if (col && map[col] === undefined) map[col] = index
    })
    return map
}

/**
 * 헤더가 있는 줄을 찾는다. 제목 줄이 앞에 붙은 파일이 흔해서 첫 줄만 보지 않는다.
 * 필수 열(번호·이름)이 모두 잡히는 첫 줄이 헤더다.
 */
export function findHeaderRow(rows) {
    const limit = Math.min(rows.length, HEADER_SCAN_ROWS)
    for (let i = 0; i < limit; i++) {
        const map = mapHeaderRow(rows[i])
        if (REQUIRED_COLS.every((c) => map[c] !== undefined)) {
            return {index: i, map}
        }
    }
    return null
}

function toNumber(text) {
    const cleaned = String(text ?? '').trim()
    if (!/^\d+$/.test(cleaned)) return null
    const n = Number(cleaned)
    return Number.isInteger(n) && n >= 1 ? n : null
}

/**
 * 행들을 명단 항목으로 바꾼다.
 *
 * 번호나 이름이 없는 줄은 **조용히 버리지 않고** 어떤 줄이 왜 빠졌는지 남긴다.
 * 30명 중 29명만 들어왔는데 아무 말이 없으면 교사는 알 방법이 없다.
 */
export function rowsToEntries(rows, headerIndex, map) {
    const entries = []
    const skipped = []

    for (let i = headerIndex + 1; i < rows.length; i++) {
        const row = rows[i]
        const at = (col) => (map[col] === undefined ? '' : row[map[col]])
        const rawName = String(at('name') ?? '').trim()
        const number = toNumber(at('number'))

        if (!rawName && number === null) continue // 완전히 빈 줄

        if (number === null) {
            skipped.push({line: i + 1, reason: `번호가 숫자가 아닙니다: "${at('number')}"`})
            continue
        }
        if (!rawName) {
            skipped.push({line: i + 1, reason: `${number}번의 이름이 비어 있습니다.`})
            continue
        }

        entries.push({
            grade: toNumber(at('grade')),
            classNo: toNumber(at('classNo')),
            number,
            name: rawName,
        })
    }

    return {entries, skipped}
}

// ── 진입점 ────────────────────────────────────────────────────

/**
 * 파일 하나를 읽어 명단으로. 화면은 이 함수만 부른다.
 *
 * @param {File} file
 * @returns {Promise<{entries, skipped, parser, headerLine, columns, missing}>}
 */
export async function readRosterFile(file) {
    const ext = extensionOf(file.name)
    if (!SUPPORTED_EXTENSIONS.includes(ext)) {
        throw new Error(
            `지원하지 않는 형식입니다: .${ext}\n엑셀(.xlsx)이나 CSV(.csv)로 저장해 주세요.` +
            (ext === 'xls' ? '\n(.xls는 .xlsx로 다시 저장해야 합니다)' : ''),
        )
    }

    const buffer = await file.arrayBuffer()
    let rows
    let parser
    if (ext === 'csv') {
        rows = parseCsv(decodeCsvBytes(buffer))
        parser = 'csv'
    } else {
        if (!looksLikeZip(buffer)) {
            throw new Error(
                `엑셀 파일이 아닙니다. 이름만 .xlsx이고 내용은 다른 형식입니다.
엑셀에서 열어 "Excel 통합 문서(.xlsx)"로 다시 저장해 주세요.`,
            )
        }
        ;({rows, parser} = await readSheetRows(buffer))
    }

    if (!rows.length) throw new Error('파일이 비어 있습니다.')

    const header = findHeaderRow(rows)
    if (!header) {
        const first = rows[0].map((c) => String(c).trim()).filter(Boolean).join(' · ')
        const missing = REQUIRED_COLS.map((c) => COL_LABELS[c]).join(' · ')
        throw new Error(
            `머리글에서 ${missing} 열을 찾지 못했습니다.\n` +
            `읽은 머리글: ${first || '(비어 있음)'}\n` +
            '첫 줄에 "학년, 반, 번호, 이름"을 넣어 주세요. 열 순서는 상관없습니다.',
        )
    }

    const {entries, skipped} = rowsToEntries(rows, header.index, header.map)
    if (!entries.length) throw new Error('머리글은 찾았지만 학생 줄이 없습니다.')

    return {
        entries,
        skipped,
        parser,
        headerLine: header.index + 1,
        columns: Object.keys(header.map),
        missing: ['grade', 'classNo'].filter((c) => header.map[c] === undefined),
    }
}

// ── 샘플 양식 ─────────────────────────────────────────────────

export const SAMPLE_HEADERS = ['학년', '반', '번호', '이름']

export const SAMPLE_ROWS = [
    [3, 6, 1, '김철수'],
    [3, 6, 2, '이영희'],
    [3, 6, 3, '박민수'],
]

/** 샘플 명렬표를 xlsx 바이트로 만든다. 저장은 호출한 쪽이 한다. */
export async function buildSampleWorkbook() {
    const workbook = new Workbook()
    const sheet = workbook.addWorksheet('명렬표')
    sheet.addRow(SAMPLE_HEADERS)
    SAMPLE_ROWS.forEach((row) => sheet.addRow(row))
    sheet.getRow(1).font = {bold: true}
    sheet.columns = SAMPLE_HEADERS.map(() => ({width: 12}))
    return await workbook.xlsx.writeBuffer()
}

/** Rust의 `write_bytes_file`이 base64를 받는다. 큰 파일에서도 스택이 터지지 않게 끊어 넘긴다. */
export function bufferToBase64(buffer) {
    const bytes = new Uint8Array(buffer)
    let binary = ''
    const CHUNK = 8192
    for (let i = 0; i < bytes.length; i += CHUNK) {
        binary += String.fromCharCode(...bytes.subarray(i, i + CHUNK))
    }
    return btoa(binary)
}
