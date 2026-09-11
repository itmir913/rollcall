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
import {CLOSING, HOMEROOM} from './slots'
import {spanTextOf} from './phrase'
// **서식은 데이터다.** 열 이름 · 정규식은 `data/neisFormats.json`에 있고 이 모듈은
// 그것을 읽는 통로만 부른다. 나이스가 서식을 바꾸면 JSON 한 장만 교체한다.
import {
    captionCellText,
    classInCaption,
    dateInText,
    looksLikePeriod,
    matchNeisColumn,
    neisSlotToken,
    splitSlotText as splitBySeparator,
} from './neisFormat'

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
    // 날짜 표기는 **서식이 정한다.** 나이스가 표기를 바꾸면 JSON 한 줄이 바뀐다.
    return dateInText(value)
}

/**
 * `조회` · `3교시` · `종례` → 우리 토큰. 교시 자리가 아니면 null.
 *
 * **`0교시`는 받지 않는다.** 이 앱의 하루는 `조회 · 1교시 … N교시 · 종례`라 0교시에
 * 자리가 없다. `0`으로 읽으면 순서값이 조회와 같아 그 줄이 조용히 `조회`로 바뀌는데,
 * 0교시를 운영하는 학교가 실제로 있으므로 이름을 바꾸지도 버리지도 않는다 —
 * `unsupportedSlotText`가 이유를 붙여 교사에게 알린다.
 */
export function slotTokenOf(text) {
    return neisSlotToken(text, {homeroom: HOMEROOM, closing: CLOSING})
}

/** 교시 표기이긴 한데 이 앱의 하루에 자리가 없는 것(`0교시`). 아니면 null. */
export function unsupportedSlotText(text) {
    const s = String(text ?? '').replace(/\s/g, '')
    return looksLikePeriod(s) && slotTokenOf(s) === null ? s : null
}

/** `조회,1교시,2교시,종례,` → `['조회','1','2','종례']`. 꼬리 쉼표는 나이스가 늘 붙인다. */
export function parseSlotList(text) {
    return splitSlotText(text)
        .map(slotTokenOf)
        .filter((token) => token !== null)
}

/** 그 문자열에 섞인 `0교시`를 모은다. 조용히 버리지 않고 이유로 말하기 위한 것이다. */
export function unsupportedSlotsIn(text) {
    return splitSlotText(text)
        .map(unsupportedSlotText)
        .filter((token) => token !== null)
}

function splitSlotText(text) {
    return splitBySeparator(text)
}

/**
 * 빠진 슬롯 목록을 저장할 구간들로 바꾼다. **순수 함수다.**
 *
 * 규칙은 하나다 — **연속된 것끼리 묶고, 각 묶음의 두 끝이 그 구간이다.**
 *
 * · `조회,1교시,…,7교시,종례` → 조회~종례 (하루 종일)
 * · `4교시,…,종례`           → 4교시~종례 (조퇴)
 * · `조회,1교시,2교시`        → 조회~2교시 (지각)
 * · `2교시,3교시`            → 2교시~3교시 (결과)
 *
 * **종례가 몇 번째인지는 그날 교시가 몇 개였는지에 달렸고, 파일은 빠진 슬롯만 준다.**
 * 학교 설정의 최대 교시로 계산하면 안 된다 — 실제 파일에서 그날 교시 수가 1 · 3 · 6 · 7로
 * 제각각이었고(단축수업 · 시험일), 7을 전제하면 `조회,1,2,3,종례`가 `조회~3교시`와 `종례`
 * 두 조각으로 분리된다. 그래서 자리를 이렇게 정한다.
 *
 *  1. **파일이 그날의 슬롯을 보여주면 그것을 쓴다.** 일일출석부는 머리글에 그날의
 *     `조회 │ 1교시 … │ 종례`가 전부 적혀 있어 종례의 자리가 확실하다(`daySlots`).
 *  2. 보여주지 않으면(월별 출결 현황) 그 줄에 적힌 **마지막 교시 다음으로 가정한다.**
 *     가정이지만 **조건 없이 붙인다.**
 *
 * 2를 조건 없이 붙이는 이유는 실제 파일의 모양 때문이다. `7교시,종례`는 보통 날
 * 마지막 교시에 조퇴한 가장 흔한 줄이고, 여기서 종례를 떼면 `7교시~7교시`와
 * `종례~종례` 두 건이 되어 이미 올바르게 저장된 `7교시~종례`와 어긋난다. 게다가
 * `종례~종례` 조퇴는 고르개가 표현할 수 없는 기간이다(조퇴에서 종례는 닫혀 있다).
 *
 * 대신 모호한 자리가 하나 남는다 — `조회,1교시,종례`는 교시가 하나뿐인 날의 하루
 * 종일일 수도, 1교시까지 지각하고 종례를 더 빠진 날일 수도 있다. **파일만으로는
 * 구별되지 않는다.** 흔한 쪽(그날이 거기서 끝났다)으로 읽는다. 실제 파일에서 그날
 * 교시 수가 3 · 6 · 7로 제각각이었던 것이 그 근거다.
 *
 * 출결구분이 **결석**인 줄은 여기까지 오지 않는다. 결석은 기간을 묻지 않아 언제나
 * 조회~종례 한 건이고, 그 판단은 종류를 아는 `buildRecords`가 한다.
 *
 * 묶음이 둘 이상 나오는 또 한 가지 경우는 **나이스가 하루 두 구간을 한 줄로 합쳐
 * 내보낸 것**이다. 실제 파일에 `조회,1교시,6교시,7교시,종례`(1교시까지 지각 + 6교시부터
 * 조퇴)가 있었다. 합쳐진 줄의 출결구분은 하나뿐이라 어느 쪽이 지각인지 파일만으로는
 * 알 수 없다 — 그래서 나누기만 하고 **판정하지 않는다.** 교사가 고친다.
 *
 * @param {string[]} tokens 그 줄에서 빠진 슬롯
 * @param {string[]|null} daySlots 파일이 보여주는 그날의 슬롯. 모르면 null.
 */
export function slotRuns(tokens, daySlots = null) {
    if (tokens.length === 0) return []

    const periodsOf = (list) => list.filter((t) => t !== HOMEROOM && t !== CLOSING).map(Number)
    const known = daySlots ? periodsOf(daySlots) : []
    const closing = Math.max(0, ...periodsOf(tokens), ...known) + 1
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
 *
 * **띄어쓰기는 양쪽에서 지운다.** 파일의 표기만 정규화하면 `출석 인정`으로 저장한 학교에서
 * 파일의 `출석인정결석`이 영영 맞지 않아 모르는 표기로 쌓인다. 다만 **돌려주는 것은
 * DB에 적힌 그대로**다 — 이 문자열로 Rust가 축을 다시 찾기 때문에, 정규화한 쪽을 넘기면
 * 그 줄이 읽지 못한 줄이 된다.
 */
export function splitCodeLabel(label, reasons = [], types = []) {
    const text = String(label ?? '').replace(/\s/g, '')
    const empty = {reasonLabel: null, typeLabel: null}
    if (!text) return empty

    const longestFirst = (list) =>
        [...list]
            .map((v) => String(v?.label ?? v))
            .map((label) => ({label, key: label.replace(/\s/g, '')}))
            .sort((a, b) => b.key.length - a.key.length)

    const typeHit = longestFirst(types).find(({key}) => key && text.endsWith(key)) ?? null
    if (!typeHit) return empty
    const head = text.slice(0, text.length - typeHit.key.length)
    const reasonHit = longestFirst(reasons).find(({key}) => key && head === key) ?? null
    return reasonHit ? {reasonLabel: reasonHit.label, typeLabel: typeHit.label} : empty
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
 * **그래서 이 목록이 곧 그날의 슬롯이다.** `slotRuns`가 종례의 자리를 여기서 안다.
 *
 * 다루지 못하는 교시 열(`0교시`)은 `unsupported`로 따로 돌려준다. 모른 척하면 그 칸의
 * 결시 표시가 통째로 사라져, 0교시에 빠진 학생이 아무 말 없이 출석으로 남는다.
 */
export function mapNeisColumns(row) {
    const columns = {}
    const slots = []
    const unsupported = []
    row.forEach((cell, index) => {
        const key = matchNeisColumn(cell)
        if (key && columns[key] === undefined) columns[key] = index
        const token = slotTokenOf(cell)
        if (token) slots.push({index, token})
        const odd = token ? null : unsupportedSlotText(cell)
        if (odd) unsupported.push({index, text: odd})
    })
    const usable =
        columns.number !== undefined &&
        columns.name !== undefined &&
        (columns.code !== undefined || columns.slots !== undefined || slots.length > 0)
    return usable ? {columns, slots, unsupported} : null
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
        const text = captionCellText(cell)
        if (text === null) continue
        const cls = classInCaption(text)
        const date = toIso(text)
        if (!cls && !date) continue
        return {
            grade: cls ? cls.grade : null,
            classNo: cls ? cls.classNo : null,
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
 *
 * **여기에는 가정이 하나 들어 있다 — 칸이 비면 출석, 한두 글자가 있으면 결시.**
 * 확인한 일일출석부가 하나뿐이라(2026-09-11) 그 파일이 쓰는 `/` 말고 다른 표기를
 * 본 적이 없어, 표기를 목록으로 두지 않고 "무엇이든 적혀 있으면 빠진 것"으로 읽는다.
 * 나이스가 **출석을 표시하는** 서식(`○` 같은 것)을 내보내면 이 가정이 뒤집혀 반 전체가
 * 결석이 된다. 교시 열이 통째로 이상할 때 가장 먼저 확인할 곳이 여기다.
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
    let oddColumns = []
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
            oddColumns = header.unsupported
            // 페이지가 바뀌었다. 앞 페이지의 마지막 학생을 이어받지 않는다.
            lastNumber = null
            lastName = ''
            return
        }

        const cell = (key) =>
            !columns || columns[key] === undefined ? '' : (row[columns[key]] ?? '')
        const codeText = cell('code')
        // 교시 열로 주는 서식(일일출석부)인가, 결시교시 문자열로 주는 서식(월별)인가.
        const byColumn = Boolean(columns) && columns.slots === undefined
        const slotTokens = !columns
            ? []
            : byColumn
                ? slotColumns.filter(({index: at}) => isMark(row[at])).map(({token}) => token)
                : parseSlotList(cell('slots'))
        // 이 앱이 다루지 못하는 교시(`0교시`). 빼고 읽으면 기간이 조용히 밀린다.
        const oddSlots = !columns
            ? []
            : byColumn
                ? oddColumns.filter(({index: at}) => isMark(row[at])).map(({text}) => text)
                : unsupportedSlotsIn(cell('slots'))
        const hasContent = codeText !== '' || slotTokens.length > 0 || oddSlots.length > 0

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

        // **`0교시`는 이 앱의 하루에 자리가 없다.** 빼고 읽으면 기간이 한 칸 밀린 채
        // 저장되므로 그 줄은 넘기고 무엇이 걸렸는지 말한다. 그 한 건은 교사가 직접 넣는다.
        if (oddSlots.length) {
            skipped.push({line, why: `이 앱이 다루지 못하는 교시입니다: ${oddSlots.join(', ')}`})
            return
        }

        const {reasonLabel, typeLabel} = splitCodeLabel(codeText, reasons, types)
        if (codeText && !typeLabel) unknownCodes.add(codeText)
        const slotPrompt = promptOf(typeLabel, types)

        // 일일출석부는 머리글에 그날의 슬롯이 전부 적혀 있다. 종례의 자리를 가정하지 않는다.
        const daySlots = byColumn && slotColumns.length ? slotColumns.map(({token}) => token) : null
        // **결석은 기간을 묻지 않는다.** 앱도 Rust도 결석을 조회~종례 한 건으로 저장하므로
        // (`ranges_for` · `day_slots`), 결시교시가 어떻게 적혀 있든 한 건이다. 여기서
        // 나누면 같은 하루가 두 건이 되고 가져오기에서 둘 다 조회~종례로 들어간다.
        // 결시교시 칸이 빈 결석 줄이 '기간 미정'으로 남아 내 기록과 어긋나던 것도 함께 막는다.
        const runs = slotPrompt === 'none'
            ? [{startSlot: HOMEROOM, endSlot: CLOSING}]
            : slotRuns(slotTokens, daySlots)
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
                // 한 줄이 두 구간으로 나뉘었다 — 나이스가 합쳐 내보냈거나, 종례를 앞
                // 교시에 붙일 근거가 파일에 없거나. 어느 쪽이든 판정하지 않고 알린다.
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
            // 합쳐진 **줄** 수다. 분리되어 나온 구간 수를 세면 한 줄을 두 건이라 말한다.
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
 *
 * **구분하지 못한 표기는 일치라고 말하지 않는다.** 출결 표기가 적혀 있는데 두 축
 * 어느 쪽도 구분하지 못한 줄은 양쪽이 null인데, 그것을 축이 미정인 내 기록과 비교하면
 * 둘 다 비어 있다는 이유로 일치가 된다. 같은 것이 아니라 같은지를 모르는 것이다.
 * Rust의 `resolve`도 같은 줄을 읽지 못한 줄로 돌려보낸다 — 두 경로가 다른 말을 하면
 * 검증 화면은 일치라 하고 가져오기 화면은 못 읽었다고 한다.
 */
export function sameRecord(mine, theirs) {
    if (unreadableCode(mine) || unreadableCode(theirs)) return false
    const norm = (v) => String(v ?? '').replace(/\s/g, '')
    return (
        norm(mine.reasonLabel) === norm(theirs.reasonLabel) &&
        norm(mine.typeLabel) === norm(theirs.typeLabel) &&
        norm(mine.startSlot) === norm(theirs.startSlot) &&
        norm(mine.endSlot) === norm(theirs.endSlot)
    )
}

/**
 * 출결 표기는 있는데 두 축 어느 쪽도 구분하지 못한 기록인가.
 *
 * 내 기록의 `codeLabel`은 두 축이 다 정해졌을 때만 채워지므로(Rust `SpanItem`) 이
 * 조건에 걸리는 것은 나이스 줄뿐이다. 그래도 양쪽에 같이 물어 두 경로를 맞춘다.
 */
function unreadableCode(row) {
    return Boolean(row?.codeLabel) && !row?.reasonLabel && !row?.typeLabel
}

/** 정렬 기준. 번호순과 날짜순 둘뿐이다. */
export function sortPairs(pairs, by) {
    const at = (pair) => pair.mine ?? pair.theirs
    return [...pairs].sort((a, b) => {
        if (by === 'date') return at(a).date.localeCompare(at(b).date) || at(a).number - at(b).number
        return at(a).number - at(b).number || at(a).date.localeCompare(at(b).date)
    })
}
