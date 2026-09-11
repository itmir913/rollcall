import {describe, expect, it} from 'vitest'
import {Workbook} from 'exceljs'
import * as XLSX from 'xlsx'
import {
    buildRosterWorkbook,
    rosterRowsOf,
    buildSampleWorkbook,
    bufferToBase64,
    cellText,
    decodeCsvBytes,
    extensionOf,
    findHeaderRow,
    looksLikeZip,
    mapHeaderRow,
    parseCsv,
    readRosterFile,
    readSheetRows,
    rowsToEntries,
    rowsWithExcelJs,
    rowsWithSheetJs,
    SAMPLE_ROWS,
} from './rosterFile.js'
import {matchColumn, normalizeHeader} from '../data/columnAliases.js'

/** 진짜 xlsx 바이트를 만들어 되읽는다. 목(mock)으로는 파서 폴백을 검증할 수 없다. */
async function xlsxBytes(rows) {
    const workbook = new Workbook()
    const sheet = workbook.addWorksheet('명렬표')
    rows.forEach((r) => sheet.addRow(r))
    return await workbook.xlsx.writeBuffer()
}

/**
 * SheetJS로 쓴 바이트. `bookType`으로 규격을 바꾼다.
 *
 * `ods`는 zip이지만 xlsx 규격이 아니라 exceljs가 시트를 하나도 찾지 못한다 —
 * 한셀 계열이 내보낸 파일에서 폴백이 도는 상황과 같다.
 */
function sheetJsBytes(bookType) {
    const sheet = XLSX.utils.aoa_to_sheet([
        ['학년', '반', '번호', '이름'],
        [3, 6, 1, '김철수'],
    ])
    const wb = XLSX.utils.book_new()
    XLSX.utils.book_append_sheet(wb, sheet, '명렬표')
    return XLSX.write(wb, {type: 'array', bookType})
}

function fileOf(name, bytes) {
    return new File([bytes], name)
}

function csvFile(name, text) {
    return fileOf(name, new TextEncoder().encode(text))
}

// ── 헤더 사전 ─────────────────────────────────────────────────

describe('헤더 별칭', () => {
    it('공백·괄호·대소문자를 무시하고 맞춘다', () => {
        expect(normalizeHeader(' 학 년 ')).toBe('학년')
        expect(matchColumn('학년')).toBe('grade')
        expect(matchColumn(' 반 ')).toBe('classNo')
        expect(matchColumn('GRADE')).toBe('grade')
        expect(matchColumn('성명')).toBe('name')
        expect(matchColumn('출석번호')).toBe('number')
    })

    it('모르는 열은 null이다', () => {
        expect(matchColumn('비고')).toBe(null)
        expect(matchColumn('')).toBe(null)
        expect(matchColumn(null)).toBe(null)
    })

    it('열은 위치가 아니라 이름으로 찾는다', () => {
        // 순서가 뒤집히고 중간에 모르는 열이 끼어도 같은 결과여야 한다.
        const map = mapHeaderRow(['이름', '비고', '번호', '반', '학년'])
        expect(map).toEqual({name: 0, number: 2, classNo: 3, grade: 4})
    })

    it('같은 열이 두 번 나오면 앞의 것을 쓴다', () => {
        expect(mapHeaderRow(['번호', '이름', '번호'])).toEqual({number: 0, name: 1})
    })
})

describe('헤더 줄 찾기', () => {
    it('제목 줄이 앞에 있어도 찾는다', () => {
        const rows = [
            ['2026학년도 3학년 6반 명렬표'],
            [],
            ['학년', '반', '번호', '이름'],
            ['3', '6', '1', '김철수'],
        ]
        const header = findHeaderRow(rows)
        expect(header.index).toBe(2)
        expect(header.map.name).toBe(3)
    })

    it('필수 열이 없으면 못 찾는다', () => {
        expect(findHeaderRow([['학년', '반'], ['3', '6']])).toBe(null)
    })
})

// ── CSV ───────────────────────────────────────────────────────

describe('CSV', () => {
    it('따옴표 안의 쉼표와 줄바꿈을 지킨다', () => {
        const rows = parseCsv('번호,이름\n1,"김,철수"\n2,"두\n줄"')
        expect(rows[1]).toEqual(['1', '김,철수'])
        expect(rows[2]).toEqual(['2', '두\n줄'])
    })

    it('두 겹 따옴표는 한 개로 푼다', () => {
        expect(parseCsv('a\n"그가 ""말""했다"')[1]).toEqual(['그가 "말"했다'])
    })

    it('빈 줄도 줄 번호를 위해 남긴다', () => {
        // 버리면 그 뒤 줄이 전부 한 칸씩 당겨져, 안내한 줄 번호와 교사가 파일에서
        // 여는 줄이 어긋난다. 빈 줄은 `rowsToEntries`가 조용히 넘긴다.
        const rows = parseCsv('번호,이름\n\n1,김철수\n')
        expect(rows.length).toBe(3)
        expect(rows[1]).toEqual([''])
        expect(rows[2]).toEqual(['1', '김철수'])
    })

    it('마지막 줄바꿈이 만드는 꼬리 줄은 뗀다', () => {
        // 파일에 없는 줄이라 남기면 '빈 줄 하나'가 늘 따라붙는다.
        expect(parseCsv('번호,이름\n1,김철수\n').length).toBe(2)
    })

    it('BOM을 떼고 읽는다', () => {
        const bytes = new TextEncoder().encode('﻿번호,이름')
        expect(decodeCsvBytes(bytes.buffer).text).toBe('번호,이름')
    })

    it('엑셀이 저장한 CP949 파일도 읽는다', () => {
        // UTF-8로만 읽으면 이름이 전부 깨진 채 조용히 들어간다.
        // 아래 바이트는 CP949로 쓴 "번호,이름\n1,김철수"이고, UTF-8로는 해독되지 않는다.
        const bytes = new Uint8Array([
            0xb9, 0xf8, 0xc8, 0xa3, 0x2c, 0xc0, 0xcc, 0xb8, 0xa7, 0x0a,
            0x31, 0x2c, 0xb1, 0xe8, 0xc3, 0xb6, 0xbc, 0xf6,
        ])
        expect(decodeCsvBytes(bytes.buffer).text).toBe('번호,이름\n1,김철수')
    })

    it('UTF-8 파일을 CP949로 잘못 읽지 않는다', () => {
        const bytes = new TextEncoder().encode('번호,이름\n1,김철수')
        expect(decodeCsvBytes(bytes.buffer).text).toBe('번호,이름\n1,김철수')
    })
})

// ── 셀 값 ─────────────────────────────────────────────────────

describe('cellText', () => {
    it('빈 값은 빈 문자열이다', () => {
        expect(cellText(null)).toBe('')
        expect(cellText(undefined)).toBe('')
    })

    it('서식 있는 글자를 이어 붙인다', () => {
        expect(cellText({richText: [{text: '김'}, {text: '철수'}]})).toBe('김철수')
    })

    it('수식 셀은 계산 결과를 쓴다', () => {
        expect(cellText({formula: 'A1&B1', result: '김철수'})).toBe('김철수')
    })

    it('숫자는 문자열로 바뀐다', () => {
        expect(cellText(7)).toBe('7')
    })
})

// ── 행 → 명단 ─────────────────────────────────────────────────

describe('rowsToEntries', () => {
    const rows = [
        ['학년', '반', '번호', '이름'],
        ['3', '6', '1', '김철수'],
        ['3', '6', '2', ' 이영희 '],
    ]
    const map = {grade: 0, classNo: 1, number: 2, name: 3}

    it('네 열을 그대로 읽는다', () => {
        const {entries, inherited} = rowsToEntries(rows, 0, map)
        expect(entries).toEqual([
            {grade: 3, classNo: 6, number: 1, name: '김철수', line: 2},
            {grade: 3, classNo: 6, number: 2, name: '이영희', line: 3},
        ])
        expect(inherited).toBe(0)
    })

    it('줄마다 파일의 줄 번호를 싣는다', () => {
        // 교과 강좌는 반이 섞여 '4번 김하늘'이 두 줄 나란히 표시될 수 있다.
        // 번호만으로는 어느 줄을 고쳐야 하는지 말하지 못한다.
        const {entries} = rowsToEntries([
            ['학년', '반', '번호', '이름'],
            ['3', '1', '4', '김하늘'],
            ['3', '6', '4', '김하늘'],
        ], 0, map)
        expect(entries.map((e) => e.line)).toEqual([2, 3])
        expect(entries.map((e) => e.classNo)).toEqual([1, 6])
    })

    it('학년·반이 없는 파일은 비워 둔다', () => {
        const {entries} = rowsToEntries(
            [['번호', '이름'], ['1', '김철수']], 0, {number: 0, name: 1},
        )
        expect(entries[0]).toEqual({grade: null, classNo: null, number: 1, name: '김철수', line: 2})
    })

    it('버린 줄을 조용히 넘기지 않고 이유를 남긴다', () => {
        // 30명 중 29명만 들어왔는데 아무 말이 없으면 교사는 알 방법이 없다.
        const {entries, skipped} = rowsToEntries([
            ['번호', '이름'],
            ['1', '김철수'],
            ['가', '이영희'],
            ['3', ''],
        ], 0, {number: 0, name: 1})
        expect(entries.length).toBe(1)
        expect(skipped.length).toBe(2)
        expect(skipped[0].reason).toContain('번호가 숫자가')
        expect(skipped[1].reason).toContain('이름이 비어')
    })

    it('완전히 빈 줄은 이유 없이 넘어간다', () => {
        const {skipped} = rowsToEntries(
            [['학년', '반', '번호', '이름'], ['', '', '', '']], 0, map,
        )
        expect(skipped.length).toBe(0)
    })

    it('학년·반만 적힌 줄은 빈 줄이 아니라 버린 줄이다', () => {
        // 번호와 이름만 보면 이 줄이 '완전히 빈 줄'로 분류되어 skipped에도 남지 않는다.
        // 담임 파일에서는 꼬리의 빈 줄이었지만 교과에서는 한 명이 조용히 사라지는 경로다.
        const {entries, skipped} = rowsToEntries([
            ['학년', '반', '번호', '이름'],
            ['3', '6', '', ''],
        ], 0, map)
        expect(entries.length).toBe(0)
        expect(skipped).toEqual([{line: 2, reason: '번호가 비어 있습니다.'}])
    })

    it('번호가 비어 있으면 그 줄의 이름을 함께 말한다', () => {
        const {skipped} = rowsToEntries([
            ['학년', '반', '번호', '이름'],
            ['3', '6', '', '김철수'],
        ], 0, map)
        expect(skipped[0].reason).toContain('김철수')
    })
})

describe('세로 병합된 학년·반', () => {
    const map = {grade: 0, classNo: 1, number: 2, name: 3}

    it('빈 칸은 위에서 이어받고 몇 줄인지 센다', () => {
        // SheetJS는 병합 아래칸을 빈칸으로 준다. 폴백을 타는 파일이 정확히
        // 한셀 · 나이스 출력물이라 교과 명렬표에서 가장 먼저 만날 형태다.
        const {entries, inherited} = rowsToEntries([
            ['학년', '반', '번호', '이름'],
            ['3', '1', '4', '김철수'],
            ['', '', '7', '이영희'],
            ['3', '6', '4', '박민수'],
            ['', '', '11', '최다은'],
        ], 0, map)
        expect(entries.map((e) => [e.grade, e.classNo])).toEqual([
            [3, 1], [3, 1], [3, 6], [3, 6],
        ])
        // 조용히 값을 만들지 않는다 — 몇 줄을 이어받았는지 반드시 센다.
        expect(inherited).toBe(2)
    })

    it('번호도 이름도 없는 줄에는 이어받지 않는다', () => {
        const {entries, inherited} = rowsToEntries([
            ['학년', '반', '번호', '이름'],
            ['3', '1', '4', '김철수'],
            ['', '', '', ''],
        ], 0, map)
        expect(entries.length).toBe(1)
        expect(inherited).toBe(0)
    })

    it('머리글을 다시 만나면 이어받기를 비운다', () => {
        // 출력물은 페이지마다 머리글을 통째로 반복한다. 앞 페이지의 반을 물려주면
        // 다음 페이지 첫 줄이 남의 반으로 들어간다.
        const {entries, skipped, inherited} = rowsToEntries([
            ['학년', '반', '번호', '이름'],
            ['3', '1', '4', '김철수'],
            ['학년', '반', '번호', '이름'],
            ['', '', '7', '이영희'],
        ], 0, map)
        expect(entries[1]).toEqual({grade: null, classNo: null, number: 7, name: '이영희', line: 4})
        expect(inherited).toBe(0)
        // 반복된 머리글은 버린 줄이 아니다.
        expect(skipped).toEqual([])
    })

    it('읽지 못한 칸은 이어받을 값으로 남기지 않는다', () => {
        const {entries, inherited} = rowsToEntries([
            ['학년', '반', '번호', '이름'],
            ['3', '일', '4', '김철수'],
            ['', '', '7', '이영희'],
        ], 0, map)
        expect(entries.map((e) => e.classNo)).toEqual([null, null])
        // 학년은 이어받았으므로 두 번째 줄은 이어받은 줄이 맞다.
        expect(entries[1].grade).toBe(3)
        expect(inherited).toBe(1)
    })

    it('학년·반 열이 아예 없는 파일에서는 이어받을 것이 없다', () => {
        const {entries, inherited} = rowsToEntries([
            ['번호', '이름'], ['1', '김철수'], ['2', '이영희'],
        ], 0, {number: 0, name: 1})
        expect(entries.every((e) => e.grade === null && e.classNo === null)).toBe(true)
        expect(inherited).toBe(0)
    })
})

// ── 실제 파일 ─────────────────────────────────────────────────

describe('readRosterFile', () => {
    it('xlsx 네 열 명렬표를 읽는다', async () => {
        const bytes = await xlsxBytes([
            ['학년', '반', '번호', '이름'],
            [3, 6, 1, '김철수'],
            [3, 6, 2, '이영희'],
        ])
        const result = await readRosterFile(fileOf('명렬표.xlsx', bytes))
        expect(result.parser).toBe('exceljs')
        expect(result.entries).toEqual([
            {grade: 3, classNo: 6, number: 1, name: '김철수', line: 2},
            {grade: 3, classNo: 6, number: 2, name: '이영희', line: 3},
        ])
        expect(result.missing).toEqual([])
        expect(result.inherited).toBe(0)
    })

    it('열 순서가 달라도 이름으로 찾는다', async () => {
        const bytes = await xlsxBytes([
            ['이름', '번호', '반', '학년'],
            ['김철수', 1, 6, 3],
        ])
        const {entries} = await readRosterFile(fileOf('뒤집힌.xlsx', bytes))
        expect(entries[0]).toEqual({grade: 3, classNo: 6, number: 1, name: '김철수', line: 2})
    })

    it('이어받은 줄 수를 화면에 알린다', async () => {
        // 조용히 값을 만들지 않는다. 파일에 없던 반이 어디서 왔는지 교사가 알아야 한다.
        const file = csvFile(
            '교과명렬표.csv',
            '학년,반,번호,이름\n3,1,4,김철수\n,,7,이영희\n3,6,4,박민수\n',
        )
        const result = await readRosterFile(file)
        expect(result.inherited).toBe(1)
        expect(result.entries[1]).toEqual(
            {grade: 3, classNo: 1, number: 7, name: '이영희', line: 3},
        )
    })

    it('중간의 빈 줄이 뒤 줄의 번호를 당기지 않는다', async () => {
        // 교사가 안내받은 줄을 파일에서 열었을 때 다른 줄이 보이면 이 화면의
        // 모든 숫자를 믿지 않게 된다.
        const file = csvFile('빈줄.csv', '학년,반,번호,이름\n3,1,4,김철수\n\n3,6,7,이영희\n')
        const {entries, skipped} = await readRosterFile(file)
        expect(entries.map((e) => e.line)).toEqual([2, 4])
        expect(skipped).toEqual([])
    })

    it('학년·반이 없으면 무엇이 없는지 알려준다', async () => {
        const bytes = await xlsxBytes([['번호', '이름'], [1, '김철수']])
        const result = await readRosterFile(fileOf('두열.xlsx', bytes))
        expect(result.missing).toEqual(['grade', 'classNo'])
    })

    it('csv도 같은 결과가 나온다', async () => {
        const file = csvFile('명렬표.csv', '학년,반,번호,이름\n3,6,1,김철수\n')
        const result = await readRosterFile(file)
        expect(result.parser).toBe('csv')
        expect(result.entries[0].name).toBe('김철수')
    })

    it('머리글을 못 찾으면 무엇을 읽었는지 보여준다', async () => {
        const bytes = await xlsxBytes([['성', '이름만'], ['김', '철수']])
        await expect(readRosterFile(fileOf('이상.xlsx', bytes)))
            .rejects.toThrow(/머리글/)
    })

    it('지원하지 않는 확장자는 이유를 말한다', async () => {
        await expect(readRosterFile(fileOf('명단.xls', new Uint8Array([1, 2]))))
            .rejects.toThrow(/\.xlsx/)
        expect(extensionOf('a/b/명단.XLSX')).toBe('xlsx')
    })

    it('이름만 .xlsx인 파일을 깨진 글자로 읽어들이지 않는다', async () => {
        // SheetJS는 아무 텍스트나 시트로 "읽어낸다". 그대로 두면 교사에게는
        // 파일이 잘못됐다는 사실 대신 알 수 없는 머리글 오류가 보인다.
        const junk = new TextEncoder().encode('이건 엑셀이 아니다')
        await expect(readRosterFile(fileOf('가짜.xlsx', junk)))
            .rejects.toThrow(/엑셀 파일이 아닙니다/)
    })

    it('zip 서명 검사는 진짜 xlsx를 막지 않는다', async () => {
        const bytes = await xlsxBytes([['번호', '이름'], [1, '김철수']])
        expect(looksLikeZip(bytes)).toBe(true)
        expect(looksLikeZip(new TextEncoder().encode('아님').buffer)).toBe(false)
    })
})

describe('SheetJS 폴백', () => {
    it('SheetJS가 만든 xlsx는 exceljs가 그대로 읽는다 — 아직 폴백이 아니다', async () => {
        // 다른 라이브러리로 썼다는 것만으로는 exceljs가 거부하지 않는다.
        // 이것을 폴백 테스트로 두면 exceljs 경로를 돌면서 통과해 버린다.
        const {rows, parser} = await readSheetRows(sheetJsBytes('xlsx'))
        expect(parser).toBe('exceljs')
        expect(rows[0]).toEqual(['학년', '반', '번호', '이름'])
        expect(rows[1]).toEqual(['3', '6', '1', '김철수'])
    })

    it('exceljs가 거부하는 파일을 SheetJS가 받는다', async () => {
        // 한셀 계열이 내보낸 표는 zip이기는 하지만 xlsx 규격이 아니다.
        // exceljs는 시트를 하나도 찾지 못하고, SheetJS는 읽어낸다.
        const bytes = sheetJsBytes('ods')
        await expect(rowsWithExcelJs(bytes)).rejects.toThrow()
        expect(rowsWithSheetJs(bytes)[1]).toEqual(['3', '6', '1', '김철수'])

        const {rows, parser} = await readSheetRows(bytes)
        expect(parser).toBe('sheetjs')
        expect(rows[1]).toEqual(['3', '6', '1', '김철수'])
    })

    it('폴백으로 읽은 파일도 명단까지 나온다 — 어느 파서였는지 함께 알린다', async () => {
        // 값이 이상할 때 어디를 의심할지 알려주는 단서라 화면에 그대로 올린다.
        const result = await readRosterFile(fileOf('한셀명렬표.xlsx', sheetJsBytes('ods')))
        expect(result.parser).toBe('sheetjs')
        expect(result.entries).toEqual([
            {grade: 3, classNo: 6, number: 1, name: '김철수', line: 2},
        ])
    })

    it('두 파서가 모두 실패하면 양쪽 이유를 함께 말한다', async () => {
        // 한쪽 이유만 보이면 교사가 보내온 파일을 두고 어디부터 볼지 알 수 없다.
        const broken = new Uint8Array(sheetJsBytes('ods')).slice(0, 300)
        expect(looksLikeZip(broken)).toBe(true) // 앞 네 바이트는 멀쩡한 zip이다
        await expect(readSheetRows(broken)).rejects.toThrow(/exceljs:.*SheetJS:/s)
    })
})

describe('줄 번호', () => {
    /**
     * 빈 줄과 빈 열을 끼운 xlsx. 1행과 A열이 비어 있고 4행도 비어 있다.
     * 실제로 시트가 B2에서 시작하는 파일이 있고, 출력물에는 빈 행이 끼어 있다.
     */
    async function gappedXlsxBytes() {
        const workbook = new Workbook()
        const sheet = workbook.addWorksheet('명렬표')
        const put = (address, value) => {
            sheet.getCell(address).value = value
        }
        ;['학년', '반', '번호', '이름'].forEach((v, i) => put(`${'BCDE'[i]}2`, v))
        ;[3, 1, 4, '김철수'].forEach((v, i) => put(`${'BCDE'[i]}3`, v))
        ;[3, 6, 7, '이영희'].forEach((v, i) => put(`${'BCDE'[i]}5`, v))
        return await workbook.xlsx.writeBuffer()
    }

    it('두 파서가 같은 줄을 같은 번호로 말한다', async () => {
        // `eachRow`는 빈 행을 건너뛰어 배열을 압축한다. 그대로 두면 같은 파일의
        // 같은 줄이 exceljs와 SheetJS에서 다른 번호로 보고된다.
        const bytes = await gappedXlsxBytes()
        const byExcelJs = await rowsWithExcelJs(bytes)
        const bySheetJs = rowsWithSheetJs(bytes)
        expect(bySheetJs).toEqual(byExcelJs)
        expect(byExcelJs.length).toBe(5)
        expect(byExcelJs[4]).toContain('이영희')
    })

    it('건너뛴 빈 줄만큼 학생 줄이 당겨지지 않는다', async () => {
        const result = await readRosterFile(fileOf('빈줄.xlsx', await gappedXlsxBytes()))
        expect(result.headerLine).toBe(2)
        expect(result.entries.map((e) => e.line)).toEqual([3, 5])
        expect(result.skipped).toEqual([])
    })

    it('exceljs는 병합된 칸에 주인 값을 준다 — 이어받을 것이 없다', async () => {
        // 두 파서가 같은 파일을 다르게 주는 자리다. 이어받기는 SheetJS 쪽을 위한 것이고,
        // exceljs 경로에서 그 수가 늘어나면 없는 병합을 지어낸 것이다.
        const workbook = new Workbook()
        const sheet = workbook.addWorksheet('명렬표')
        sheet.addRow(['학년', '반', '번호', '이름'])
        sheet.addRow([3, 1, 4, '김철수'])
        sheet.addRow([null, null, 7, '이영희'])
        sheet.mergeCells('A2:A3')
        sheet.mergeCells('B2:B3')
        const result = await readRosterFile(fileOf('병합.xlsx', await workbook.xlsx.writeBuffer()))
        expect(result.parser).toBe('exceljs')
        expect(result.entries.map((e) => [e.grade, e.classNo])).toEqual([[3, 1], [3, 1]])
        expect(result.inherited).toBe(0)
    })
})

describe('샘플 양식', () => {
    it('base64로 옮겨도 바이트가 유지된다', async () => {
        // Rust의 write_bytes_file이 이 문자열을 그대로 디스크에 쓴다.
        // 여기서 한 바이트라도 어긋나면 내려받은 양식이 열리지 않는다.
        const buffer = await buildSampleWorkbook()
        const back = Uint8Array.from(atob(bufferToBase64(buffer)), (c) => c.charCodeAt(0))
        const result = await readRosterFile(fileOf('양식.xlsx', back))
        expect(result.entries.length).toBe(SAMPLE_ROWS.length)
        expect(result.entries[0].name).toBe('김철수')
    })

    it('만든 파일을 그대로 다시 읽을 수 있다', async () => {
        // 샘플이 우리 파서를 통과하지 못하면 배포할 이유가 없다.
        const bytes = await buildSampleWorkbook()
        const result = await readRosterFile(fileOf('샘플.xlsx', bytes))
        expect(result.entries.length).toBe(SAMPLE_ROWS.length)
        expect(result.missing).toEqual([])
        expect(result.entries[0]).toEqual({grade: 3, classNo: 1, number: 4, name: '김철수', line: 2})
    })

    it('반이 섞여 있고 같은 번호가 두 반에 있다', async () => {
        // 교과 강좌는 선택과목이라 여러 반이 한 명단에 모이고, 번호 하나로는 학생을
        // 구별할 수 없다. 한 반으로만 된 양식을 내려받으면 학년 · 반 열을 비워 두어도
        // 되는 줄 알고, 그 파일은 교과 강좌에 들어가지 못한다.
        const seats = SAMPLE_ROWS.map(([grade, classNo]) => `${grade}-${classNo}`)
        expect(new Set(seats).size).toBeGreaterThan(1)
        const numbers = SAMPLE_ROWS.map(([, , number]) => number)
        expect(new Set(numbers).size).toBeLessThan(numbers.length)
    })
})

describe('자체 양식 — 내보낸 것을 그대로 다시 읽는다', () => {
    /**
     * **이 양식이 바닥이라는 주장의 증거다.** 나이스 엑셀 파일 열기는 그 위에 얹는
     * 어댑터이고, 나이스가 서식을 바꿔도 이 왕복은 그대로여야 한다. 내보낸 파일을
     * 자기가 못 읽으면 교사의 명단이 이 앱 안에 갇힌다.
     */
    const STUDENTS = [
        {grade: 3, classNo: 1, number: 4, name: '김하늘'},
        {grade: 3, classNo: 6, number: 4, name: '박서연'},
        {grade: 3, classNo: 6, number: 11, name: '이도윤'},
    ]

    it('학적 자리와 이름이 한 글자도 달라지지 않는다', async () => {
        const bytes = await buildRosterWorkbook(rosterRowsOf(STUDENTS))
        const {entries, skipped, missing} = await readRosterFile(fileOf('내보낸.xlsx', bytes))

        expect(skipped).toEqual([])
        // 학년 · 반이 모두 실려 있어야 교과 강좌에도 그대로 들어간다.
        expect(missing).toEqual([])
        expect(entries.map(({grade, classNo, number, name}) => ({grade, classNo, number, name})))
            .toEqual(STUDENTS)
    })

    it('같은 번호가 두 반에 있어도 섞이지 않는다', async () => {
        const bytes = await buildRosterWorkbook(rosterRowsOf(STUDENTS))
        const {entries} = await readRosterFile(fileOf('내보낸.xlsx', bytes))

        const 사번 = entries.filter((e) => e.number === 4)
        expect(사번).toHaveLength(2)
        expect(사번.map((e) => e.classNo)).toEqual([1, 6])
    })

    it('양식 내려받기와 내보내기가 같은 머리글을 쓴다', async () => {
        // 달라지면 내보낸 파일을 자기가 못 읽는다.
        const sample = await readRosterFile(fileOf('양식.xlsx', await buildSampleWorkbook()))
        const mine = await readRosterFile(fileOf('내보낸.xlsx',
            await buildRosterWorkbook(rosterRowsOf(STUDENTS))))
        expect(mine.columns).toEqual(sample.columns)
        expect(mine.headerLine).toBe(sample.headerLine)
    })
})

describe('CSV 인코딩 — 자동으로 알아본다', () => {
    /**
     * **이 앱이 늘 넘어지던 자리다.** 엑셀이 저장한 한국어 CSV는 CP949인 경우가 많고,
     * `유니코드 텍스트` 저장은 UTF-16LE다. 하나라도 놓치면 이름이 전부 깨진 채 조용히
     * 들어가거나, 번호 칸이 숫자로 읽히지 않아 서른 줄이 통째로 버려진다.
     */
    const NL = String.fromCharCode(10)
    const ROWS = ['학년,반,번호,이름', '3,1,4,김하늘', ''].join(NL)

    const utf8 = (text) => new TextEncoder().encode(text)

    /** UTF-16 바이트를 손으로 짠다. JS에는 UTF-16 인코더가 없다. */
    function utf16(text, {little = true, bom = false} = {}) {
        const units = []
        if (bom) units.push(0xfeff)
        for (const ch of text) units.push(ch.charCodeAt(0))
        const bytes = new Uint8Array(units.length * 2)
        units.forEach((u, i) => {
            bytes[i * 2 + (little ? 0 : 1)] = u & 0xff
            bytes[i * 2 + (little ? 1 : 0)] = u >> 8
        })
        return bytes
    }

    /** `학년,반,번호,이름
3,1,4,김하늘
`의 CP949 바이트. */
    const CP949 = new Uint8Array([
        199, 208, 179, 226, 44, 185, 221, 44, 185, 248, 200, 163, 44, 192, 204, 184, 167, 10,
        51, 44, 49, 44, 52, 44, 177, 232, 199, 207, 180, 195, 10,
    ])

    const 경우 = {
        'UTF-8': {bytes: utf8(ROWS), encoding: 'utf-8'},
        'UTF-8 (BOM)': {bytes: new Uint8Array([0xef, 0xbb, 0xbf, ...utf8(ROWS)]), encoding: 'utf-8'},
        'CP949 (엑셀 한국어 저장)': {bytes: CP949, encoding: 'euc-kr'},
        'UTF-16LE (BOM)': {bytes: utf16(ROWS, {bom: true}), encoding: 'utf-16le'},
        'UTF-16LE (BOM 없음)': {bytes: utf16(ROWS), encoding: 'utf-16le'},
        'UTF-16BE (BOM)': {bytes: utf16(ROWS, {little: false, bom: true}), encoding: 'utf-16be'},
        'UTF-16BE (BOM 없음)': {bytes: utf16(ROWS, {little: false}), encoding: 'utf-16be'},
    }

    for (const [이름, {bytes, encoding}] of Object.entries(경우)) {
        it(`${이름} — 글자가 한 자도 깨지지 않는다`, () => {
            const decoded = decodeCsvBytes(bytes.buffer ?? bytes)
            expect(decoded.encoding).toBe(encoding)
            expect(decoded.text).toContain('김하늘')
            // BOM은 글자가 아니다. 남으면 첫 머리글 앞에 보이지 않는 글자가 붙어 열을 못 찾는다.
            expect(decoded.text.startsWith('학년')).toBe(true)
        })

        it(`${이름} — 파일로 읽어도 학생이 그대로 들어온다`, async () => {
            const result = await readRosterFile(fileOf('명렬표.csv', bytes))
            expect(result.encoding).toBe(encoding)
            expect(result.skipped).toEqual([])
            expect(result.entries).toHaveLength(1)
            expect(result.entries[0]).toMatchObject({grade: 3, classNo: 1, number: 4, name: '김하늘'})
        })
    }

    it('어느 것으로도 깨끗하게 읽히지 않으면 숨기지 않는다', () => {
        // UTF-8도 CP949도 아닌 바이트. 읽히는 만큼 읽되 그 사실을 말한다.
        const 이상한것 = new Uint8Array([0xc0, 0xc0, 0xc0, 0x80, 0x80])
        expect(decodeCsvBytes(이상한것.buffer).encoding).toBe('알 수 없음')
    })
})
