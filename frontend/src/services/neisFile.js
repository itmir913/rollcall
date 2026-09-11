/**
 * 나이스 출결 파일을 읽고, 이 앱의 기록과 대조한다.
 *
 * 나이스가 내려주는 서식은 둘이고, 2026-09-11에 실제 파일로 확인했다.
 *
 * **일일출석부** — 학급담임 › 학적 › 출결관리 › 일일출결관리(담임용) › [출력] › XLS data
 * ```
 *  1행                                        (오른쪽 끝) 2026.09.11.  ← 출력일
 *  2행  일일출석부
 *  3행   ※ 3학년 6반 2026.09.01.(화)                       ← 학년 · 반 · **일자**
 *  4행  번호 │ 성명 │ 마감 │ 조회 │ 1교시 … 7교시 │ 종례 │ 비고
 *  5행  1    │ …                                          ← 결석이 없으면 뒤가 빈다
 *  6행  2    │ … │ 질병조퇴 │     │  /  …  /   │  /  │ …  ← 빠진 슬롯에 `/`
 *  …    (가운데) 1 / 2                         (오른쪽 끝) 학교명       ← 페이지 꼬리
 * ```
 * **출력물이라** 페이지마다 출력일 · 캡션 · 머리글이 통째로 반복되고 빈 행이 끼어 있다.
 *
 * **월별 출결 현황** — 학급담임 › 학적 › 출결현황및통계 › 학급별출결현황 › [엑셀다운로드]
 * ```
 *  1행  일자 │ 번호 │ 성명 │ 출결구분 │ 결시교시 │ 사유
 *  2행  2026.06.09.(화) │ 2 │ … │ 질병조퇴 │ 4교시,5교시,6교시,7교시,종례, │ …
 * ```
 * 번호 · 성명은 한 학생의 연속 행에 **세로 병합**되어 있다.
 *
 * 두 서식의 공통점이 이 모듈의 뼈대다 — **빠진 슬롯의 목록이 곧 기간이다.**
 * 일별은 열에 `/` 표시로, 월별은 `조회,1교시,…,종례,` 문자열로 준다. 그 목록을
 * `slotRuns`가 우리 표현(시작 · 끝)으로 바꾼다.
 *
 * 파서를 둘 쓰는 이유는 명렬표와 같다 — 한셀·나이스가 내보낸 xlsx는 규격에서 조금씩
 * 벗어나 exceljs가 거부하는 경우가 있다. 어느 파서가 읽었는지 화면에 알린다.
 */
import ExcelJS from 'exceljs'
import * as XLSX from 'xlsx'
import {matchNeisColumn} from '../data/columnAliases'
import {CLOSING, HOMEROOM} from './slots'
import {spanTextOf} from './phrase'

const ZIP_SIGNATURE = [0x50, 0x4b, 0x03, 0x04]

/** 이름만 xlsx인 파일을 먼저 거른다. 안 그러면 아무 텍스트나 시트로 "읽어낸다". */
export function looksLikeXlsx(bytes) {
    return ZIP_SIGNATURE.every((byte, i) => bytes[i] === byte)
}

/** `2026.09.10.` · `2026-09-10` · `2026.06.09.(화)` · `20260910`을 ISO로. 못 읽으면 null. */
export function toIso(value) {
    if (value instanceof Date) {
        const pad = (n) => String(n).padStart(2, '0')
        return `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())}`
    }
    const text = String(value ?? '').trim()
    const dotted = text.match(/(\d{4})[.\-/](\d{1,2})[.\-/](\d{1,2})/)
    if (dotted) {
        const [, y, m, d] = dotted
        return `${y}-${m.padStart(2, '0')}-${d.padStart(2, '0')}`
    }
    const packed = text.match(/^(\d{4})(\d{2})(\d{2})$/)
    if (packed) return `${packed[1]}-${packed[2]}-${packed[3]}`
    return null
}

/** `조회` · `3교시` · `종례` → 우리 토큰. 교시 자리가 아니면 null. */
export function slotTokenOf(text) {
    const s = String(text ?? '').replace(/\s/g, '')
    if (s === HOMEROOM) return HOMEROOM
    if (s === CLOSING) return CLOSING
    const m = s.match(/^(\d{1,2})교시$/)
    return m ? String(Number(m[1])) : null
}

/** `조회,1교시,2교시,종례,` → `['조회','1','2','종례']`. 꼬리 쉼표는 나이스가 늘 붙인다. */
export function parseSlotList(text) {
    return String(text ?? '')
        .split(/[,·]/)
        .map(slotTokenOf)
        .filter((token) => token !== null)
}

/**
 * 빠진 슬롯 목록을 저장할 구간들로 바꾼다. **순수 함수다.**
 *
 * 규칙은 하나다 — **이어진 것끼리 묶고, 각 묶음의 두 끝이 그 구간이다.**
 *
 * · `조회,1교시,…,7교시,종례` → 조회~종례 (결석)
 * · `4교시,…,종례`           → 4교시~종례 (조퇴)
 * · `조회,1교시,2교시`        → 조회~2교시 (지각)
 * · `2교시,3교시`            → 2교시~3교시 (결과)
 *
 * **종례의 자리는 그 줄에 적힌 마지막 교시 다음이다.** 학교 설정의 최대 교시로
 * 계산하면 안 된다 — 실제 파일에서 그날 교시 수가 3 · 6 · 7로 제각각이었고(단축수업 ·
 * 시험일), 7을 전제하면 `조회,1,2,3,종례`가 `조회~3교시`와 `종례` 두 조각으로 갈라진다.
 *
 * 묶음이 둘 이상 나오는 것은 **나이스가 하루 두 구간을 한 줄로 합쳐 내보냈다는 뜻**이다.
 * 실제 파일에 `조회,1교시,6교시,7교시,종례`(1교시까지 지각 + 6교시부터 조퇴)가 있었다.
 * 합쳐진 줄의 출결구분은 하나뿐이라 어느 쪽이 지각인지 파일만으로는 알 수 없다 —
 * 그래서 나누기만 하고 **판정하지 않는다.** 화면이 그 줄에 표시하고 교사가 고친다.
 */
export function slotRuns(tokens) {
    if (tokens.length === 0) return []

    const periods = tokens.filter((t) => t !== HOMEROOM && t !== CLOSING).map(Number)
    const closing = (periods.length ? Math.max(...periods) : 0) + 1
    const ordinalOf = (t) => (t === HOMEROOM ? 0 : t === CLOSING ? closing : Number(t))
    const nameOf = (o) => (o === 0 ? HOMEROOM : o === closing ? CLOSING : String(o))

    const seq = [...new Set(tokens.map(ordinalOf))].sort((a, b) => a - b)
    const runs = []
    for (const o of seq) {
        const last = runs[runs.length - 1]
        if (last && o === last[1] + 1) last[1] = o
        else runs.push([o, o])
    }
    return runs.map(([a, b]) => ({startSlot: nameOf(a), endSlot: nameOf(b)}))
}

/**
 * `질병조퇴` → 구분 `질병` + 종류 `조퇴`.
 *
 * **후보는 DB에서 온다.** 구분과 종류는 데이터라 사용자가 늘릴 수 있고, 목록을
 * 여기 적어 두면 늘린 순간 조용히 틀린다. 구분하지 못하면 양쪽 다 null이고, 부르는 쪽이
 * 그 사실을 교사에게 알린다.
 *
 * 긴 것부터 맞춘다 — `출석인정`이 `인정`보다 먼저 걸려야 `출석인정결석`이 제대로 구분된다.
 */
export function splitCodeLabel(label, reasons = [], types = []) {
    const text = String(label ?? '').replace(/\s/g, '')
    const empty = {reasonLabel: null, typeLabel: null}
    if (!text) return empty

    const longestFirst = (list) =>
        [...list].map((v) => String(v?.label ?? v)).sort((a, b) => b.length - a.length)

    const typeLabel = longestFirst(types).find((t) => t && text.endsWith(t)) ?? null
    if (!typeLabel) return empty
    const head = text.slice(0, text.length - typeLabel.length)
    const reasonLabel = longestFirst(reasons).find((r) => r && head === r) ?? null
    return reasonLabel ? {reasonLabel, typeLabel} : empty
}

/** 그 종류가 교사에게 묻는 교시가 어느 쪽인지. 문구를 Rust와 맞추는 데 쓴다. */
function promptOf(typeLabel, types) {
    const hit = [...types].find((t) => t?.label === typeLabel)
    return hit?.slotPrompt ?? null
}

/**
 * 머리글 한 줄에서 열 위치를 찾는다. 머리글이 아니면 null.
 *
 * 교시 열은 별칭이 아니라 모양으로 알아본다 — 학교마다 교시 수가 다르다.
 */
export function mapNeisColumns(row) {
    const columns = {}
    const slots = []
    row.forEach((cell, index) => {
        const key = matchNeisColumn(cell)
        if (key && columns[key] === undefined) columns[key] = index
        const token = slotTokenOf(cell)
        if (token) slots.push({index, token})
    })
    const usable =
        columns.number !== undefined &&
        columns.name !== undefined &&
        (columns.code !== undefined || columns.slots !== undefined || slots.length > 0)
    return usable ? {columns, slots} : null
}

/**
 * ` ※ 3학년 6반 2026.09.01.(화)` — 일일출석부는 여기에만 일자가 있다.
 *
 * **학급도 일자도 없으면 캡션이 아니다.** `※`는 한국어 메모에서 흔한 기호라
 * (`※ 진단서 제출`), 그것만 보고 캡션으로 삼으면 그 줄의 출결이 통째로 사라지고
 * 일일출석부에서는 캡션의 일자까지 날아가 뒤따르는 학생이 전부 실패한다.
 */
export function captionOf(row) {
    for (const cell of row) {
        const text = String(cell ?? '').trim()
        if (!text.startsWith('※')) continue
        const cls = text.match(/(\d+)\s*학년\s*(\d+)\s*반/)
        const date = toIso(text)
        if (!cls && !date) continue
        return {
            grade: cls ? Number(cls[1]) : null,
            classNo: cls ? Number(cls[2]) : null,
            date,
        }
    }
    return null
}

/**
 * 교시 열의 결시 표시인가. 일일출석부는 `/`를 쓴다.
 *
 * 길이로 거르는 이유는 페이지 꼬리(`1 / 2`)가 하필 교시 열 자리에 찍히기 때문이다.
 * 표시는 한두 글자이고 꼬리는 그보다 길다. 꼬리 줄은 결국 출결 내용이 비어 있어
 * 데이터가 아닌 줄로 걸러지므로, 꼬리를 따로 알아보는 규칙은 두지 않는다 —
 * 두면 사유가 `9 / 1`인 줄까지 조용히 버린다.
 */
function isMark(cell) {
    const text = String(cell ?? '').trim()
    return text !== '' && text.length <= 2
}

/**
 * 파일을 읽어 기록 목록으로. 읽지 못하면 **던진다.**
 *
 * @param {Uint8Array} bytes
 * @param {{reasons?: Array, types?: Array}} axis DB의 구분 · 종류. 코드 라벨을 구분하는 데 쓴다.
 */
export async function readNeisFile(bytes, axis = {}) {
    if (!looksLikeXlsx(bytes)) {
        throw new Error('엑셀(.xlsx) 파일이 아닙니다. 나이스에서 내려받은 파일을 그대로 넣어주세요.')
    }

    // exceljs가 **읽기는 했으나 쓸 수 없는** 경우까지 폴백에 넣는다. 머리글을 못 찾은
    // 것도 실패이므로 `buildRecords`까지 넣고 던지는지로 판단한다.
    try {
        return buildRecords(await readWithExcelJs(bytes), {parser: 'exceljs', ...axis})
    } catch (first) {
        try {
            return buildRecords(readWithSheetJs(bytes), {parser: 'SheetJS', ...axis})
        } catch (second) {
            // 두 이유를 모두 보여준다. 뒤엣것만 남기면 어느 단계에서 막혔는지 알 수 없다.
            throw new Error(
                `나이스 파일을 읽지 못했습니다. (exceljs: ${first.message ?? first}` +
                ` / SheetJS: ${second.message ?? second})`,
            )
        }
    }
}

/**
 * 표(2차원 문자열)를 기록 목록으로. **파일과 무관한 순수 함수라 테스트가 여기 붙는다.**
 *
 * 버린 줄은 조용히 넘기지 않는다. 다만 출력물의 장식(제목 · 출력일 · 캡션 · 페이지 꼬리 ·
 * 빈 행)은 버린 줄이 아니다 — 그것까지 세면 정작 읽지 못한 학생 한 명이 묻힌다.
 * 결석이 없어 뒤가 빈 줄도 마찬가지다. 그것이 일일출석부의 정상 상태다.
 */
export function buildRecords(table, {parser = 'exceljs', reasons = [], types = []} = {}) {
    const rows = []
    const skipped = []
    const unknownCodes = new Set()

    let columns = null
    let slotColumns = []
    let caption = null
    let lastNumber = null
    let lastName = ''

    table.forEach((source, index) => {
        const line = index + 1
        const row = source.map((cell) => String(cell ?? '').trim())
        if (row.every((cell) => cell === '')) return

        const header = mapNeisColumns(row)
        if (header) {
            columns = header.columns
            slotColumns = header.slots
            // 페이지가 바뀌었다. 앞 페이지의 마지막 학생을 이어받지 않는다.
            lastNumber = null
            lastName = ''
            return
        }

        const cell = (key) =>
            !columns || columns[key] === undefined ? '' : (row[columns[key]] ?? '')
        const codeText = cell('code')
        const slotTokens = !columns
            ? []
            : columns.slots === undefined
                ? slotColumns.filter(({index: at}) => isMark(row[at])).map(({token}) => token)
                : parseSlotList(cell('slots'))
        const hasContent = codeText !== '' || slotTokens.length > 0

        // **캡션은 출결 내용이 없는 줄에서만 읽는다.** 사유 칸의 `※ …`가 그 줄을
        // 삼키면 출결 한 건이 조용히 사라진다.
        if (!hasContent) {
            const found = captionOf(row)
            if (found) caption = found
            // 제목 · 출력일 · 페이지 꼬리, 그리고 그날 결석이 없는 학생.
            // 일일출석부는 전교생을 한 줄씩 싣는다 — 버린 줄이 아니다.
            return
        }
        if (!columns) return

        // 번호 · 성명은 한 학생의 연속 행에 세로 병합되어 있다. SheetJS는 병합 아래칸을
        // 빈칸으로 주므로 이어받는다. **빈칸일 때만 이어받는다** — 읽을 수 없는 번호까지
        // 이어받으면 그 출결이 앞 학생에게 붙고, 아무 데도 남지 않는다.
        const numberText = cell('number')
        let number = null
        let name = cell('name')

        if (/^\d+$/.test(numberText)) {
            number = Number(numberText)
            lastNumber = number
            if (name) lastName = name
        } else if (numberText === '' && lastNumber !== null) {
            number = lastNumber
            name = name || lastName
        } else {
            skipped.push({
                line,
                why: numberText === ''
                    ? '번호를 읽지 못했습니다.'
                    : `번호를 읽지 못했습니다: ${numberText}`,
            })
            return
        }

        const date = columns.date === undefined ? caption?.date : toIso(cell('date'))
        if (!date) {
            skipped.push({line, why: '일자를 읽지 못했습니다.'})
            return
        }

        const {reasonLabel, typeLabel} = splitCodeLabel(codeText, reasons, types)
        if (codeText && !typeLabel) unknownCodes.add(codeText)
        const slotPrompt = promptOf(typeLabel, types)

        const runs = slotRuns(slotTokens)
        const spans = runs.length ? runs : [{startSlot: null, endSlot: null}]
        for (const run of spans) {
            rows.push({
                line,
                number,
                name,
                date,
                codeLabel: codeText || null,
                reasonLabel,
                typeLabel,
                slotPrompt,
                startSlot: run.startSlot,
                endSlot: run.endSlot,
                spanText: spanTextOf(slotPrompt, run.startSlot, run.endSlot),
                detail: cell('detail') || null,
                // 나이스가 하루 두 구간을 한 줄로 합쳐 내보낸 것이다. 판정하지 않고 알린다.
                merged: spans.length > 1,
            })
        }
    })

    if (!columns) {
        throw new Error(
            '나이스 파일의 머리글을 찾지 못했습니다. 일일출석부(XLS data)나 월별 출결 현황을 그대로 넣어주세요.',
        )
    }

    const dates = rows.map((r) => r.date).sort()
    return {
        rows,
        meta: {
            parser,
            from: dates[0] ?? null,
            to: dates[dates.length - 1] ?? null,
            grade: caption?.grade ?? null,
            classNo: caption?.classNo ?? null,
            skipped,
            unknownCodes: [...unknownCodes],
            // 합쳐진 **줄** 수다. 갈라져 나온 구간 수를 세면 한 줄을 두 건이라 말한다.
            merged: new Set(rows.filter((r) => r.merged).map((r) => r.line)).size,
        },
    }
}

/** exceljs는 빈 행을 건너뛰므로 행 번호로 읽는다. 버린 줄을 말할 때 줄 번호가 맞아야 한다. */
async function readWithExcelJs(bytes) {
    const book = new ExcelJS.Workbook()
    await book.xlsx.load(bytes)
    const sheet = book.worksheets[0]
    if (!sheet) throw new Error('시트가 없습니다.')
    const table = []
    for (let r = 1; r <= sheet.rowCount; r += 1) {
        const row = sheet.getRow(r)
        const cells = []
        for (let c = 1; c <= sheet.columnCount; c += 1) cells.push(textOf(row.getCell(c).value))
        table.push(cells)
    }
    return table
}

/**
 * 두 파서가 **같은 모양**을 내놓아야 한다. 그러지 않으면 어느 것이 읽었느냐에 따라
 * 결과가 달라지고, 정작 이 경로를 타는 것은 exceljs가 거부한 실제 파일이다.
 *
 *  · 칸 값은 `textOf`를 거친다 — 그러지 않으면 날짜 칸이 `6/1/26`으로 와서 `toIso`가 거부한다.
 *  · 범위를 A1부터로 넓힌다 — 시트가 B2에서 시작하면 줄 번호와 열 번호가 통째로 밀린다.
 */
function readWithSheetJs(bytes) {
    const book = XLSX.read(bytes, {type: 'array', cellDates: true})
    const sheet = book.Sheets[book.SheetNames[0]]
    if (!sheet) throw new Error('시트가 없습니다.')
    const range = XLSX.utils.decode_range(sheet['!ref'] ?? 'A1')
    range.s.r = 0
    range.s.c = 0
    const table = XLSX.utils.sheet_to_json(sheet, {
        header: 1, raw: true, blankrows: true, defval: '', range,
    })
    return table.map((row) => (Array.isArray(row) ? row.map(textOf) : []))
}

/** exceljs의 칸 값은 문자열이 아닐 수 있다 — 서식 있는 글, 수식, 날짜. */
function textOf(value) {
    if (value === null || value === undefined) return ''
    if (value instanceof Date) return toIso(value) ?? ''
    if (typeof value === 'object') {
        if (Array.isArray(value.richText)) return value.richText.map((part) => part.text).join('')
        if (value.text !== undefined) return String(value.text)
        if (value.result !== undefined) return String(value.result)
        return ''
    }
    return String(value)
}

/**
 * 앱 기록과 나이스 기록을 대조한다. **순수 함수다.**
 *
 * 세 갈래로 구분한다 — 서로 다름 / 나이스에만 / 앱에만.
 * 같은 것은 돌려주되 화면이 접어 둔다.
 *
 * 짝은 (번호, 날짜)로 맞춘다. 하루에 두 구간인 경우가 있으므로 **여럿을 여럿과**
 * 비교한다. 같은 값이 있으면 짝을 지우고, 남은 것끼리 다름으로 본다.
 */
export function compareRecords(appSpans, neisRows) {
    const key = (r) => `${r.number}|${r.date}`
    const groups = new Map()

    for (const span of appSpans) {
        const bucket = groups.get(key(span)) ?? {mine: [], theirs: []}
        bucket.mine.push(span)
        groups.set(key(span), bucket)
    }
    for (const row of neisRows) {
        const bucket = groups.get(key(row)) ?? {mine: [], theirs: []}
        bucket.theirs.push(row)
        groups.set(key(row), bucket)
    }

    const same = []
    const diff = []
    const onlyNeis = []
    const onlyApp = []

    for (const bucket of groups.values()) {
        const theirs = [...bucket.theirs]
        const leftovers = []

        for (const mine of bucket.mine) {
            const at = theirs.findIndex((row) => sameRecord(mine, row))
            if (at >= 0) {
                same.push({mine, theirs: theirs[at]})
                theirs.splice(at, 1)
            } else {
                leftovers.push(mine)
            }
        }

        while (leftovers.length && theirs.length) {
            diff.push({mine: leftovers.shift(), theirs: theirs.shift()})
        }
        leftovers.forEach((mine) => onlyApp.push({mine, theirs: null}))
        theirs.forEach((row) => onlyNeis.push({mine: null, theirs: row}))
    }

    return {same, diff, onlyNeis, onlyApp}
}

/**
 * 두 기록이 같은가. 두 축과 **기간**을 본다.
 *
 * 기간까지 보는 이유는, 같은 날 같은 학생의 `1교시부터 조퇴`와 `5교시부터 조퇴`가
 * 축만으로는 구별되지 않기 때문이다. 나이스 파일이 결시교시를 주므로 비교할 수 있다.
 */
export function sameRecord(mine, theirs) {
    const norm = (v) => String(v ?? '').replace(/\s/g, '')
    return (
        norm(mine.reasonLabel) === norm(theirs.reasonLabel) &&
        norm(mine.typeLabel) === norm(theirs.typeLabel) &&
        norm(mine.startSlot) === norm(theirs.startSlot) &&
        norm(mine.endSlot) === norm(theirs.endSlot)
    )
}

/** 정렬 기준. 번호순과 날짜순 둘뿐이다. */
export function sortPairs(pairs, by) {
    const at = (pair) => pair.mine ?? pair.theirs
    return [...pairs].sort((a, b) => {
        if (by === 'date') return at(a).date.localeCompare(at(b).date) || at(a).number - at(b).number
        return at(a).number - at(b).number || at(a).date.localeCompare(at(b).date)
    })
}
