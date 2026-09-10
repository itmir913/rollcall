/**
 * 나이스 출결 파일을 읽고, 이 앱의 기록과 대조한다.
 *
 * **파일 서식은 아직 분석하지 않았다.** 그래서 이 모듈은 두 부분으로 나뉜다.
 *   · `readNeisFile` — 파일을 열어 표로 만드는 부분. 서식을 모르므로 열 별칭으로
 *     찾고, 못 찾으면 **정직하게 실패**한다. 조용히 빈 결과를 돌려주지 않는다.
 *   · `compareRecords` — 두 기록을 대조하는 **순수 함수**. 서식과 무관하므로 지금
 *     확정할 수 있고, 테스트도 여기에 붙는다.
 *
 * 파서를 둘 쓰는 이유는 명렬표와 같다 — 한셀·나이스가 내보낸 xlsx는 규격에서 조금씩
 * 벗어나 exceljs가 거부하는 경우가 있다. 어느 파서가 읽었는지 화면에 알린다.
 */
import ExcelJS from 'exceljs'
import * as XLSX from 'xlsx'

/** 나이스 파일에서 찾을 열 이름. 학교·버전마다 조금씩 다르다. */
export const NEIS_ALIASES = {
    number: ['번호', '학번', '출석번호'],
    name: ['성명', '이름', '학생명'],
    date: ['일자', '날짜', '출결일자', '결석일'],
    reason: ['사유', '결석사유', '구분'],
    type: ['출결', '출결상황', '종류', '유형'],
    detail: ['비고', '내용', '세부내용'],
}

const ZIP_SIGNATURE = [0x50, 0x4b, 0x03, 0x04]

/** 이름만 xlsx인 파일을 먼저 거른다. 안 그러면 아무 텍스트나 시트로 "읽어낸다". */
export function looksLikeXlsx(bytes) {
    return ZIP_SIGNATURE.every((byte, i) => bytes[i] === byte)
}

/** 머리글 한 줄에서 열 위치를 찾는다. **위치가 아니라 이름으로 찾는다.** */
export function mapColumns(header) {
    const found = {}
    header.forEach((cell, index) => {
        const text = String(cell ?? '').replace(/\s/g, '')
        for (const [key, names] of Object.entries(NEIS_ALIASES)) {
            if (found[key] === undefined && names.some((n) => text.includes(n))) found[key] = index
        }
    })
    return found
}

/** `2026.09.10.` · `2026-09-10` · `20260910`을 ISO로. 못 읽으면 null이다. */
export function toIso(value) {
    if (value instanceof Date) {
        const pad = (n) => String(n).padStart(2, '0')
        return `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())}`
    }
    const text = String(value ?? '').trim()
    const dotted = text.match(/^(\d{4})[.\-/](\d{1,2})[.\-/](\d{1,2})/)
    if (dotted) {
        const [, y, m, d] = dotted
        return `${y}-${m.padStart(2, '0')}-${d.padStart(2, '0')}`
    }
    const packed = text.match(/^(\d{4})(\d{2})(\d{2})$/)
    if (packed) return `${packed[1]}-${packed[2]}-${packed[3]}`
    return null
}

/**
 * 파일을 읽어 표로. 읽지 못하면 **던진다.**
 * @returns {{rows: Array, meta: {parser: string, from: string|null, to: string|null, skipped: Array}}}
 */
export async function readNeisFile(bytes) {
    if (!looksLikeXlsx(bytes)) {
        throw new Error('엑셀(.xlsx) 파일이 아닙니다. 나이스에서 내려받은 파일을 그대로 넣어주세요.')
    }

    let table = null
    let parser = 'exceljs'
    try {
        table = await readWithExcelJs(bytes)
    } catch {
        parser = 'SheetJS'
        table = readWithSheetJs(bytes)
    }

    const headerIndex = table.findIndex((row) => Object.keys(mapColumns(row)).length >= 3)
    if (headerIndex < 0) {
        throw new Error(
            '나이스 파일의 머리글을 찾지 못했습니다. 아직 분석하지 않은 서식입니다 — 파일을 알려주시면 맞추겠습니다.',
        )
    }

    const columns = mapColumns(table[headerIndex])
    const rows = []
    const skipped = []
    table.slice(headerIndex + 1).forEach((row, offset) => {
        const number = Number(String(row[columns.number] ?? '').trim())
        const date = toIso(row[columns.date])
        if (!Number.isInteger(number) || !date) {
            if (row.some((cell) => String(cell ?? '').trim() !== '')) {
                skipped.push({line: headerIndex + offset + 2, why: '번호나 일자를 읽지 못했습니다.'})
            }
            return
        }
        rows.push({
            number,
            name: String(row[columns.name] ?? '').trim(),
            date,
            reasonLabel: String(row[columns.reason] ?? '').trim() || null,
            typeLabel: String(row[columns.type] ?? '').trim() || null,
            detail: String(row[columns.detail] ?? '').trim() || null,
        })
    })

    const dates = rows.map((r) => r.date).sort()
    return {
        rows,
        meta: {parser, from: dates[0] ?? null, to: dates[dates.length - 1] ?? null, skipped},
    }
}

async function readWithExcelJs(bytes) {
    const book = new ExcelJS.Workbook()
    await book.xlsx.load(bytes)
    const sheet = book.worksheets[0]
    if (!sheet) throw new Error('시트가 없습니다.')
    const table = []
    sheet.eachRow((row) => {
        table.push(row.values.slice(1).map((cell) => (cell?.text ?? cell ?? '')))
    })
    return table
}

function readWithSheetJs(bytes) {
    const book = XLSX.read(bytes, {type: 'array', cellDates: true})
    const sheet = book.Sheets[book.SheetNames[0]]
    if (!sheet) throw new Error('시트가 없습니다.')
    return XLSX.utils.sheet_to_json(sheet, {header: 1, raw: false})
}

/**
 * 앱 기록과 나이스 기록을 대조한다. **순수 함수다.**
 *
 * 세 갈래로 가른다 — 서로 다름 / 나이스에만 / 앱에만.
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

/** 두 기록이 같은가. 구분·종류의 라벨만 비교한다 — 표기 차이는 별칭이 흡수한다. */
export function sameRecord(mine, theirs) {
    const norm = (v) => String(v ?? '').replace(/\s/g, '')
    return (
        norm(mine.reasonLabel) === norm(theirs.reasonLabel) &&
        norm(mine.typeLabel) === norm(theirs.typeLabel)
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
