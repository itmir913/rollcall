import {describe, expect, it} from 'vitest'
import {Workbook} from 'exceljs'
import * as XLSX from 'xlsx'
import {
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
} from './rosterFile.js'
import {matchColumn, normalizeHeader} from '../data/columnAliases.js'

/** 진짜 xlsx 바이트를 만들어 되읽는다. 목(mock)으로는 파서 폴백을 검증할 수 없다. */
async function xlsxBytes(rows) {
    const workbook = new Workbook()
    const sheet = workbook.addWorksheet('명렬표')
    rows.forEach((r) => sheet.addRow(r))
    return await workbook.xlsx.writeBuffer()
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

    it('빈 줄은 버린다', () => {
        expect(parseCsv('번호,이름\n\n1,김철수\n').length).toBe(2)
    })

    it('BOM을 떼고 읽는다', () => {
        const bytes = new TextEncoder().encode('﻿번호,이름')
        expect(decodeCsvBytes(bytes.buffer)).toBe('번호,이름')
    })

    it('엑셀이 저장한 CP949 파일도 읽는다', () => {
        // UTF-8로만 읽으면 이름이 전부 깨진 채 조용히 들어간다.
        // 아래 바이트는 CP949로 쓴 "번호,이름\n1,김철수"이고, UTF-8로는 해독되지 않는다.
        const bytes = new Uint8Array([
            0xb9, 0xf8, 0xc8, 0xa3, 0x2c, 0xc0, 0xcc, 0xb8, 0xa7, 0x0a,
            0x31, 0x2c, 0xb1, 0xe8, 0xc3, 0xb6, 0xbc, 0xf6,
        ])
        expect(decodeCsvBytes(bytes.buffer)).toBe('번호,이름\n1,김철수')
    })

    it('UTF-8 파일을 CP949로 잘못 읽지 않는다', () => {
        const bytes = new TextEncoder().encode('번호,이름\n1,김철수')
        expect(decodeCsvBytes(bytes.buffer)).toBe('번호,이름\n1,김철수')
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
        const {entries} = rowsToEntries(rows, 0, map)
        expect(entries).toEqual([
            {grade: 3, classNo: 6, number: 1, name: '김철수'},
            {grade: 3, classNo: 6, number: 2, name: '이영희'},
        ])
    })

    it('학년·반이 없는 파일은 비워 둔다', () => {
        const {entries} = rowsToEntries(
            [['번호', '이름'], ['1', '김철수']], 0, {number: 0, name: 1},
        )
        expect(entries[0]).toEqual({grade: null, classNo: null, number: 1, name: '김철수'})
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
        const {skipped} = rowsToEntries([['번호', '이름'], ['', '']], 0, {number: 0, name: 1})
        expect(skipped.length).toBe(0)
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
            {grade: 3, classNo: 6, number: 1, name: '김철수'},
            {grade: 3, classNo: 6, number: 2, name: '이영희'},
        ])
        expect(result.missing).toEqual([])
    })

    it('열 순서가 달라도 이름으로 찾는다', async () => {
        const bytes = await xlsxBytes([
            ['이름', '번호', '반', '학년'],
            ['김철수', 1, 6, 3],
        ])
        const {entries} = await readRosterFile(fileOf('뒤집힌.xlsx', bytes))
        expect(entries[0]).toEqual({grade: 3, classNo: 6, number: 1, name: '김철수'})
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
    it('SheetJS가 만든 파일도 같은 행으로 읽힌다', async () => {
        // 비표준 xlsx를 흉내내기 위해 다른 라이브러리로 쓴 파일을 넣는다.
        const sheet = XLSX.utils.aoa_to_sheet([
            ['학년', '반', '번호', '이름'],
            [3, 6, 1, '김철수'],
        ])
        const wb = XLSX.utils.book_new()
        XLSX.utils.book_append_sheet(wb, sheet, '명렬표')
        const bytes = XLSX.write(wb, {type: 'array', bookType: 'xlsx'})

        const {rows} = await readSheetRows(bytes)
        expect(rows[0]).toEqual(['학년', '반', '번호', '이름'])
        expect(rows[1]).toEqual(['3', '6', '1', '김철수'])
    })
})

describe('샘플 양식', () => {
    it('base64로 옮겨도 바이트가 유지된다', async () => {
        // Rust의 write_bytes_file이 이 문자열을 그대로 디스크에 쓴다.
        // 여기서 한 바이트라도 어긋나면 내려받은 양식이 열리지 않는다.
        const buffer = await buildSampleWorkbook()
        const back = Uint8Array.from(atob(bufferToBase64(buffer)), (c) => c.charCodeAt(0))
        const result = await readRosterFile(fileOf('양식.xlsx', back))
        expect(result.entries.length).toBe(3)
        expect(result.entries[0].name).toBe('김철수')
    })

    it('만든 파일을 그대로 다시 읽을 수 있다', async () => {
        // 샘플이 우리 파서를 통과하지 못하면 배포할 이유가 없다.
        const bytes = await buildSampleWorkbook()
        const result = await readRosterFile(fileOf('샘플.xlsx', bytes))
        expect(result.entries.length).toBe(3)
        expect(result.missing).toEqual([])
        expect(result.entries[0]).toEqual({grade: 3, classNo: 6, number: 1, name: '김철수'})
    })
})
