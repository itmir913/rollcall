/**
 * 명렬표 파일(xlsx · csv)을 읽어 `RosterEntry` 목록으로 바꾼다.
 *
 * 붙여넣기 방식을 쓰지 않는 이유는 열을 나누는 기호 때문이다. 탭인지 쉼표인지, 이름에
 * 쉼표가 들어갔는지를 텍스트만 보고는 확실히 알 수 없다. 파일에는 그 정보가
 * 들어 있다.
 *
 * 파서는 둘이다. **exceljs로 먼저 읽고, 실패하면 SheetJS로 다시 읽는다.**
 * 한셀이나 나이스가 내보낸 xlsx는 규격에서 조금씩 벗어나 exceljs가 거부하는
 * 경우가 있는데, SheetJS는 그런 파일도 대체로 읽어낸다. 반대로 SheetJS만 쓰면
 * 서식이 복잡한 정상 파일에서 값이 달라지는 경우가 있어 순서를 이렇게 둔다.
 *
 * 이 모듈은 **파일 형식을 다룰 뿐 업무 규칙을 다루지 않는다.** 명단 차분(추가·
 * 전출·개명 판정)과 저장은 전부 Rust가 한다. 학년·반이 필수인지 아닌지도 여기서
 * 결정하지 않는다 — 이 모듈은 대상 학급을 모른다. 없는 열은 `missing`으로 알리고
 * 판단은 화면이 한다.
 *
 * **줄 번호는 파일의 줄 번호와 같아야 한다.** 세 경로(exceljs · SheetJS · CSV)가
 * 모두 빈 줄을 그대로 세어 같은 번호를 말한다. 교사가 안내받은 줄을 엑셀에서 열어
 * 멀쩡한 줄을 보는 순간 이 화면의 모든 숫자를 믿지 않게 된다. 나이스 파일을 읽는
 * `neisFile.js`가 같은 이유로 같은 방법을 쓴다.
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
 * 바이트 앞머리의 BOM이 말하는 인코딩. 없으면 null.
 *
 * **UTF-16을 반드시 본다.** 엑셀의 `유니코드 텍스트` 저장과 일부 도구의 CSV 저장이
 * UTF-16LE다. 이것을 놓치면 한글이 섞인 파일은 깨진 글자로, **ASCII만 있는 파일은
 * UTF-8 디코드가 통과해 버려** 글자 사이에 NUL이 끼어 들어간다. 그러면 번호 칸이
 * 숫자로 읽히지 않아 서른 줄이 전부 버려지는데, 교사는 파일이 잘못된 줄 안다.
 */
export function bomOf(bytes) {
    if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
        return {encoding: 'utf-8', skip: 3}
    }
    // UTF-32 BOM이 UTF-16LE BOM으로 시작하므로 먼저 걸러낸다.
    if (bytes[0] === 0xff && bytes[1] === 0xfe && bytes[2] === 0x00 && bytes[3] === 0x00) {
        return {encoding: 'utf-32le', skip: 4}
    }
    if (bytes[0] === 0xff && bytes[1] === 0xfe) return {encoding: 'utf-16le', skip: 2}
    if (bytes[0] === 0xfe && bytes[1] === 0xff) return {encoding: 'utf-16be', skip: 2}
    return null
}

/**
 * BOM 없는 UTF-16인가. **NUL 바이트의 자리로 알아본다.**
 *
 * UTF-16은 ASCII 글자마다 NUL을 한 짝씩 끼우므로, 한쪽 자리에만 NUL이 몰린다.
 * 텍스트 파일에 NUL이 들어갈 일은 그것 말고 없다 — 하나라도 있으면 UTF-8도 CP949도 아니다.
 */
export function looksLikeUtf16(bytes) {
    const limit = Math.min(bytes.length, 512)
    if (limit < 4) return null
    let even = 0
    let odd = 0
    for (let i = 0; i < limit; i++) {
        if (bytes[i] !== 0x00) continue
        if (i % 2 === 0) even += 1
        else odd += 1
    }
    if (even === 0 && odd === 0) return null
    // 한쪽으로 확실히 몰려야 한다. 섞여 있으면 UTF-16이 아니라 깨진 파일이다.
    if (odd > even * 4) return 'utf-16le'   // 낮은 바이트가 앞 → 홀수 자리가 NUL
    if (even > odd * 4) return 'utf-16be'
    return null
}

/**
 * CSV 바이트를 글자로 푼다. **어느 인코딩으로 읽었는지 함께 돌려준다.**
 *
 * 엑셀이 저장한 한국어 CSV는 CP949(euc-kr)인 경우가 많다. UTF-8로만 읽으면 이름이
 * 전부 깨진 채 조용히 들어간다. 순서는 BOM → UTF-16 짐작 → UTF-8 → CP949다.
 *
 * **어느 것으로 읽었는지 화면에 알린다.** 파서 이름을 알리는 것과 같은 이유다 —
 * 이름이 이상할 때 어디를 의심할지 알려주는 단서가 된다.
 */
export function decodeCsvBytes(buffer) {
    const bytes = new Uint8Array(buffer)

    const bom = bomOf(bytes)
    if (bom) {
        try {
            return {
                text: new TextDecoder(bom.encoding).decode(bytes.subarray(bom.skip)),
                encoding: bom.encoding,
            }
        } catch {
            // utf-32le처럼 브라우저가 모르는 인코딩이면 아래로 내려간다.
        }
    }

    const guessed = looksLikeUtf16(bytes)
    if (guessed) {
        try {
            return {text: new TextDecoder(guessed).decode(bytes), encoding: guessed}
        } catch { /* 아래로 */ }
    }

    for (const encoding of ['utf-8', 'euc-kr']) {
        try {
            return {
                text: new TextDecoder(encoding, {fatal: true}).decode(bytes),
                encoding,
            }
        } catch { /* 다음 인코딩으로 */ }
    }
    // 어느 것으로도 깨끗하게 읽히지 않았다. 읽히는 만큼 읽되 **그 사실을 숨기지 않는다.**
    return {text: new TextDecoder('utf-8').decode(bytes), encoding: '알 수 없음'}
}

/**
 * RFC 4180 CSV. 따옴표 안의 줄바꿈과 쉼표를 지킨다.
 *
 * 줄 단위로 먼저 자르면 `"홍길동, 김"` 같은 값이나 셀 안 줄바꿈에서 깨진다.
 * 그래서 글자를 하나씩 본다.
 *
 * **빈 줄을 버리지 않는다.** 버리면 그 뒤 줄이 전부 한 칸씩 당겨져, 안내한 줄 번호와
 * 교사가 파일에서 여는 줄이 어긋난다. 빈 줄은 `rowsToEntries`가 조용히 넘긴다.
 * 다만 마지막 줄바꿈이 만드는 꼬리 한 줄은 파일에 없는 줄이라 뗀다.
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

    const last = rows[rows.length - 1]
    if (rows.length > 1 && last.length === 1 && last[0] === '') rows.pop()
    return rows
}

// ── 시트 → 행 ─────────────────────────────────────────────────

/**
 * 두 파서는 각각 내보낸다. 화면은 `readSheetRows`만 부르지만, 테스트가 폴백을
 * 확인하려면 **어느 파서가 무엇을 거부하는지**를 따로 물어볼 수 있어야 한다.
 * `readSheetRows`만 열어 두면 앞 파서가 읽어 버린 파일로도 테스트가 통과한다.
 */
export async function rowsWithExcelJs(buffer) {
    const workbook = new Workbook()
    await workbook.xlsx.load(buffer)
    const sheet = workbook.worksheets[0]
    if (!sheet) throw new Error('시트가 없습니다.')
    // **`eachRow`를 쓰지 않는다.** 그것은 빈 행을 건너뛰어 배열을 압축하므로 같은
    // 파일의 같은 줄이 SheetJS와 다른 번호로 보고된다. 행 번호로 훑어 자리를 지킨다.
    const rows = []
    for (let r = 1; r <= sheet.rowCount; r++) {
        const row = sheet.getRow(r)
        const cells = []
        for (let c = 1; c <= sheet.columnCount; c++) cells.push(cellText(row.getCell(c).value))
        rows.push(cells)
    }
    return rows
}

/**
 * SheetJS로 읽는다. **exceljs와 같은 모양을 내놓아야 한다.**
 *
 *  · 범위를 A1부터로 넓힌다 — 시트가 B2에서 시작하면 줄 번호와 열 번호가 통째로 밀린다.
 *  · `blankrows`를 켠다 — 빈 행을 빼면 그 뒤 줄 번호가 전부 당겨진다.
 */
export function rowsWithSheetJs(buffer) {
    const workbook = XLSX.read(buffer, {type: 'array'})
    const name = workbook.SheetNames[0]
    if (!name) throw new Error('시트가 없습니다.')
    const sheet = workbook.Sheets[name]
    const range = XLSX.utils.decode_range(sheet['!ref'] ?? 'A1')
    range.s.r = 0
    range.s.c = 0
    const raw = XLSX.utils.sheet_to_json(sheet, {
        header: 1, raw: true, blankrows: true, defval: '', range,
    })
    return raw.map((row) => (Array.isArray(row) ? row.map(cellText) : []))
}

/**
 * 엑셀 파일을 행 배열로. exceljs → SheetJS 순서로 시도한다.
 * 어느 파서가 읽었는지 함께 돌려준다 — 폴백이 쓰였다는 사실은 화면에 알린다.
 */
export async function readSheetRows(buffer) {
    try {
        const rows = await rowsWithExcelJs(buffer)
        // **행 수가 아니라 내용으로 판단한다.** 서식만 남은 빈 행도 행으로 세어지므로
        // 길이만 보면 아무것도 읽지 못한 파일에서 폴백이 돌지 않는다.
        if (hasContent(rows)) return {rows, parser: 'exceljs'}
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

/** 값이 한 칸이라도 있는가. 빈 표를 '읽었다'고 말하지 않기 위한 것이다. */
function hasContent(rows) {
    return rows.some((row) => row.some((cell) => String(cell ?? '').trim() !== ''))
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

/** 그 줄에 읽을 것이 있는가. 빈 줄은 줄 번호를 위해 남아 있을 뿐이다. */
function rowHasContent(row) {
    return (row ?? []).some((cell) => String(cell ?? '').trim() !== '')
}

/**
 * 헤더가 있는 줄을 찾는다. 제목 줄이 앞에 붙은 파일이 흔해서 첫 줄만 보지 않는다.
 * 필수 열(번호·이름)이 모두 잡히는 첫 줄이 헤더다.
 *
 * **세는 것은 내용이 있는 줄이다.** 줄 번호를 지키려고 빈 줄을 남기게 되면서, 물리적인
 * 열 줄을 세면 제목과 빈 줄이 앞에 붙은 파일에서 머리글이 범위 밖으로 밀린다 —
 * 나이스 출력물이 정확히 그런 모양이다. 돌려주는 `index`는 그대로 물리적 자리다.
 */
export function findHeaderRow(rows) {
    let looked = 0
    for (let i = 0; i < rows.length && looked < HEADER_SCAN_ROWS; i++) {
        if (!rowHasContent(rows[i])) continue
        looked += 1
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

/** 그 줄이 머리글인가. 출력물은 페이지마다 머리글을 통째로 반복한다. */
function isHeaderRow(row) {
    const map = mapHeaderRow(row)
    return REQUIRED_COLS.every((c) => map[c] !== undefined)
}

/**
 * 행들을 명단 항목으로 바꾼다.
 *
 * 번호나 이름이 없는 줄은 **조용히 버리지 않고** 어떤 줄이 왜 버려졌는지 남긴다.
 * 30명 중 29명만 들어왔는데 아무 말이 없으면 교사는 알 방법이 없다. 그래서 빈 줄인지는
 * **네 칸을 모두 보고** 판단한다 — 번호와 이름만 보면 학년 · 반만 적힌 줄이 '완전히 빈 줄'로
 * 분류되어 `skipped`에도 남지 않는다. 담임 파일에서는 꼬리의 빈 줄이었지만, 반이 섞이는
 * 교과 명렬표에서는 진짜 한 명이 아무 말 없이 사라지는 경로다.
 *
 * 학년 · 반이 **세로 병합된 파일**은 그 값을 위에서 이어받는다. exceljs는 병합 아래칸에
 * 주인 값을 주지만 SheetJS는 빈칸으로 주는데, 폴백(SheetJS)을 타는 파일이 정확히
 * 한셀 · 나이스 출력물이라 교과 명렬표에서 가장 먼저 만날 형태다. 조건은 셋이다 —
 * 그 줄에 번호나 이름이 있을 때만, 그 칸이 비어 있을 때만, 그리고 **몇 줄을 이어받았는지
 * 반드시 센다**(`inherited`). 조용히 값을 만들지 않는다.
 *
 * @returns {{entries: Array, skipped: Array, inherited: number}}
 */
export function rowsToEntries(rows, headerIndex, map) {
    const entries = []
    const skipped = []
    let inherited = 0
    // 위에서 이어받을 학년 · 반. 머리글을 다시 만나면(다음 페이지) 비운다.
    let lastGrade = null
    let lastClassNo = null

    for (let i = headerIndex + 1; i < rows.length; i++) {
        const row = rows[i] ?? []
        const line = i + 1
        const at = (col) => (map[col] === undefined ? '' : String(row[map[col]] ?? '').trim())

        // 반복된 머리글은 버린 줄이 아니다. 앞 페이지의 학년 · 반을 넘겨서도 안 된다.
        if (isHeaderRow(row)) {
            lastGrade = null
            lastClassNo = null
            continue
        }

        const gradeText = at('grade')
        const classText = at('classNo')
        const numberText = at('number')
        const rawName = at('name')

        if (!gradeText && !classText && !numberText && !rawName) continue // 완전히 빈 줄

        // 읽지 못한 칸은 이어받을 값으로 두지 않는다. 남겨 두면 다음 줄이 그 값을 물려받는다.
        if (gradeText) lastGrade = toNumber(gradeText)
        if (classText) lastClassNo = toNumber(classText)

        const number = toNumber(numberText)
        if (number === null) {
            skipped.push({
                line,
                reason: numberText === ''
                    ? `번호가 비어 있습니다.${rawName ? ` (이름 "${rawName}")` : ''}`
                    : `번호가 숫자가 아닙니다: "${numberText}"`,
            })
            continue
        }
        if (!rawName) {
            skipped.push({line, reason: `${number}번의 이름이 비어 있습니다.`})
            continue
        }

        const borrowed = (!gradeText && lastGrade !== null) || (!classText && lastClassNo !== null)
        if (borrowed) inherited += 1

        entries.push({
            grade: gradeText ? toNumber(gradeText) : lastGrade,
            classNo: classText ? toNumber(classText) : lastClassNo,
            number,
            name: rawName,
            // 버린 줄이 자기를 가리키기 위한 값이다 — 교과에서 '4번 김하늘'이 두 줄
            // 나란히 표시되면 번호만으로는 어느 줄을 고칠지 말하지 못한다.
            line,
        })
    }

    return {entries, skipped, inherited}
}

// ── 진입점 ────────────────────────────────────────────────────

/**
 * 파일 하나를 읽어 명단으로. 화면은 이 함수만 부른다.
 *
 * @param {File} file
 * @returns {Promise<{entries, skipped, parser, encoding, headerLine, columns, missing, inherited}>}
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
    // 어느 인코딩으로 읽었는지. 엑셀 파일은 zip 안이 언제나 UTF-8이라 물을 것이 없다.
    let encoding = ''
    if (ext === 'csv') {
        const decoded = decodeCsvBytes(buffer)
        encoding = decoded.encoding
        rows = parseCsv(decoded.text)
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

    // 빈 줄도 줄 번호를 위해 남아 있으므로 행 수가 아니라 내용으로 판단한다.
    if (!hasContent(rows)) throw new Error('파일이 비어 있습니다.')

    const header = findHeaderRow(rows)
    if (!header) {
        // 빈 줄을 남기게 되었으므로 `rows[0]`이 빈 행일 수 있다. 그것을 보여주면
        // 교사에게 '(비어 있음)'만 돌려주게 된다 — 읽은 것을 말하지 못한다.
        const firstRow = rows.find(rowHasContent) ?? []
        const first = firstRow.map((c) => String(c).trim()).filter(Boolean).join(' · ')
        const missing = REQUIRED_COLS.map((c) => COL_LABELS[c]).join(' · ')
        throw new Error(
            `머리글에서 ${missing} 열을 찾지 못했습니다.\n` +
            `읽은 머리글: ${first || '(비어 있음)'}\n` +
            '첫 줄에 "학년, 반, 번호, 이름"을 넣어 주세요. 열 순서는 상관없습니다.',
        )
    }

    const {entries, skipped, inherited} = rowsToEntries(rows, header.index, header.map)
    if (!entries.length) throw new Error('머리글은 찾았지만 학생 줄이 없습니다.')

    return {
        entries,
        skipped,
        parser,
        // CSV를 어느 인코딩으로 읽었는지. 엑셀 파일은 zip 안이 언제나 UTF-8이라 빈 문자열이다.
        encoding,
        headerLine: header.index + 1,
        columns: Object.keys(header.map),
        // 없는 열을 알리기만 한다. 학년 · 반이 필수인지는 대상 학급을 아는 화면이 판단한다.
        missing: ['grade', 'classNo'].filter((c) => header.map[c] === undefined),
        // 학년 · 반이 비어 위에서 이어받은 줄 수.
        inherited,
    }
}

// ── 샘플 양식 ─────────────────────────────────────────────────

export const SAMPLE_HEADERS = ['학년', '반', '번호', '이름']

/**
 * 반을 섞어 둔다. 교과 강좌는 선택과목이라 여러 반이 한 명단에 모이고, **번호 하나로는
 * 학생을 구별할 수 없다** — 3학년 1반 4번과 3학년 6반 4번이 같은 강좌에 있다. 한 반으로만
 * 된 예시를 저장하면 학년 · 반 열을 비워 두어도 되는 줄 알고, 그 파일은 교과 강좌에
 * 들어가지 못한다. 담임 명렬표로 쓸 때는 학년 · 반을 읽지 않으므로 섞여 있어도 무방하다.
 */
export const SAMPLE_ROWS = [
    [3, 1, 4, '김철수'],
    [3, 6, 4, '이영희'],
    [3, 6, 11, '박민수'],
]

/**
 * 명단을 **자체 양식** xlsx 바이트로 만든다. 저장은 호출한 쪽이 한다.
 *
 * **이 양식이 바닥이다.** 나이스 엑셀 파일 열기는 그 위에 얹는 어댑터이고, 이 앱이
 * 언제나 읽고 쓸 수 있는 것은 이 양식 하나다. 그래서 내보낸 파일은 `readRosterFile`이
 * 그대로 다시 읽는다 — 학교를 옮기든 컴퓨터를 바꾸든 명단은 이 파일로 따라간다.
 *
 * 머리글을 `SAMPLE_HEADERS` 하나에서 읽는 이유가 그것이다. 파일 저장과 예시
 * 예시 저장이 각각 머리글을 적으면 한쪽만 고쳐져 **내보낸 파일을 자기가 못 읽는다.**
 */
export async function buildRosterWorkbook(rows) {
    const workbook = new Workbook()
    const sheet = workbook.addWorksheet('명렬표')
    sheet.addRow(SAMPLE_HEADERS)
    rows.forEach((row) => sheet.addRow(row))
    sheet.getRow(1).font = {bold: true}
    sheet.columns = SAMPLE_HEADERS.map(() => ({width: 12}))
    return await workbook.xlsx.writeBuffer()
}

/** 학생 목록을 자체 양식의 줄로. `StudentItem`이 들고 오는 학적 자리를 그대로 적는다. */
export function rosterRowsOf(students) {
    return (students ?? []).map((s) => [s.grade, s.classNo, s.number, s.name])
}

/** 샘플 명렬표를 xlsx 바이트로 만든다. 파일 저장과 **같은 양식**이다. */
export async function buildSampleWorkbook() {
    return await buildRosterWorkbook(SAMPLE_ROWS)
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
