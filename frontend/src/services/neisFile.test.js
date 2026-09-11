/**
 * 나이스 파일 읽기와 대조.
 *
 * 서식은 2026-09-11에 실제로 내려받은 두 파일에서 확인했다. 여기 있는 표는 그
 * 구조를 그대로 본뜬 것이고 **이름은 전부 가짜다** — 학생 이름은 개인정보라
 * 저장소에 남기지 않는다.
 *
 * 대조 규칙은 서식과 무관한 순수 함수라 아래쪽에 따로 모았다.
 */
import {describe, expect, it} from 'vitest'
import ExcelJS from 'exceljs'
import {
    buildRecords,
    captionOf,
    compareRecords,
    looksLikeXlsx,
    mapNeisColumns,
    parseSlotList,
    readNeisFile,
    sameRecord,
    slotRuns,
    slotTokenOf,
    sortPairs,
    splitCodeLabel,
    toIso,
    unsupportedSlotText,
    unsupportedSlotsIn,
} from './neisFile'

/** DB에서 오는 축. 하드코딩하지 않는다는 사실을 테스트도 따른다. */
const REASONS = [{label: '질병'}, {label: '미인정'}, {label: '기타'}, {label: '출석인정'}]
const TYPES = [
    {label: '지각', slotPrompt: 'end'},
    {label: '조퇴', slotPrompt: 'start'},
    {label: '결석', slotPrompt: 'none'},
    {label: '결과', slotPrompt: 'multi'},
]
const AXIS = {reasons: REASONS, types: TYPES}

const read = (table) => buildRecords(table, AXIS)

// ── 서식 1. 일일출석부 (XLS data) ───────────────────────────────
// 출력물이라 페이지마다 머리글이 반복되고 빈 행과 페이지 꼬리가 끼어 있다.
const DAILY_HEAD = ['번호', '성명', '마감', '조회', '1교시', '2교시', '3교시', '종례', '비고']
const DAILY = [
    ['', '', '', '', '', '', '', '', '2026.09.11.'],
    ['일일출석부'],
    [' ※ 3학년 6반 2026.09.01.(화)'],
    DAILY_HEAD,
    ['1', '학생1'],
    ['2', '학생2', '질병조퇴', '', '/', '/', '/', '/', '두통으로 1교시부터 질병조퇴'],
    ['3', '학생3', '질병결석', '/', '/', '/', '/', '/', '몸살로 질병결석'],
    ['', '', '', '', '1 / 2', '', '', '', '운양고등학교'],
    [],
    ['', '', '', '', '', '', '', '', '2026.09.11.'],
    [' ※ 3학년 6반 2026.09.01.(화)'],
    DAILY_HEAD,
    ['4', '학생4', '질병지각', '/', '', '', '', '', '늦잠으로 조회까지 질병지각'],
    ['', '', '', '', '2 / 2', '', '', '', '운양고등학교'],
    [],
]

// ── 서식 2. 월별 출결 현황 ──────────────────────────────────────
// 번호 · 성명이 한 학생의 연속 행에 세로 병합되어 있다. SheetJS는 아래칸을 빈칸으로 준다.
const MONTHLY = [
    ['일자', '번호', '성명', '출결구분', '결시교시', '사유'],
    ['2026.06.01.(월)', '2', '학생2', '출석인정결석', '조회,1교시,2교시,3교시,종례,', '경조사'],
    ['2026.06.09.(화)', '', '', '질병조퇴', '4교시,5교시,6교시,7교시,종례,', '복통으로 4교시부터'],
    ['2026.06.10.(수)', '18', '학생18', '질병지각', '조회,', '몸살로 조회까지'],
    ['2026.06.16.(화)', '23', '학생23', '질병조퇴', '조회,1교시,6교시,7교시,종례,',
        '1교시까지 질병지각, 6교시부터 질병조퇴'],
]

describe('일일출석부', () => {
    const out = read(DAILY)

    it('일자 · 학년 · 반은 표가 아니라 ※ 캡션에 있다', () => {
        expect(out.meta.grade).toBe(3)
        expect(out.meta.classNo).toBe(6)
        expect(out.rows.every((r) => r.date === '2026-09-01')).toBe(true)
    })

    it('출력물의 장식은 버린 줄이 아니다 — 반복 머리글 · 페이지 꼬리 · 빈 행', () => {
        expect(out.meta.skipped).toEqual([])
    })

    it('결석이 없어 뒤가 빈 줄은 버린 줄이 아니다. 그것이 정상 상태다', () => {
        expect(out.rows.map((r) => r.number)).toEqual([2, 3, 4])
    })

    it('두 번째 페이지의 학생도 읽는다 — 머리글이 다시 나와도 이어 읽는다', () => {
        expect(out.rows.at(-1)).toMatchObject({number: 4, codeLabel: '질병지각'})
    })

    it('교시 열의 `/` 표시가 곧 기간이다', () => {
        expect(out.rows[0]).toMatchObject({startSlot: '1', endSlot: '종례'})
        expect(out.rows[1]).toMatchObject({startSlot: '조회', endSlot: '종례'})
        expect(out.rows[2]).toMatchObject({startSlot: '조회', endSlot: '조회'})
    })

    it('페이지 꼬리의 `1 / 2`가 교시 열 자리에 찍혀도 결시로 읽지 않는다', () => {
        expect(out.rows).toHaveLength(3)
    })
})

describe('월별 출결 현황', () => {
    const out = read(MONTHLY)

    it('일자는 행마다 있다', () => {
        expect(out.meta.from).toBe('2026-06-01')
        expect(out.meta.to).toBe('2026-06-16')
    })

    it('세로 병합된 번호 · 성명은 위에서 이어받는다', () => {
        expect(out.rows[1]).toMatchObject({number: 2, name: '학생2', codeLabel: '질병조퇴'})
    })

    it('그날 교시가 3교시뿐이어도 하루 종일로 읽는다', () => {
        // 최대 교시(7)로 종례 자리를 계산하면 조회~3교시와 종례로 분리된다.
        expect(out.rows[0]).toMatchObject({startSlot: '조회', endSlot: '종례'})
    })

    it('조퇴는 시작 교시부터 종례까지다', () => {
        expect(out.rows[1]).toMatchObject({startSlot: '4', endSlot: '종례'})
    })

    it('결시교시가 조회 하나뿐인 지각도 있다 — 실제 파일에 있었다', () => {
        expect(out.rows[2]).toMatchObject({codeLabel: '질병지각', startSlot: '조회', endSlot: '조회'})
    })

    it('나이스가 하루 두 구간을 한 줄로 합쳐 내보내면 나누고 알린다', () => {
        const merged = out.rows.filter((r) => r.merged)
        expect(merged).toHaveLength(2)
        expect(merged[0]).toMatchObject({startSlot: '조회', endSlot: '1'})
        expect(merged[1]).toMatchObject({startSlot: '6', endSlot: '종례'})
        // 분리되어 나온 구간은 둘이지만 **합쳐진 줄은 하나다.** 교사에게는 줄 수로 말한다.
        expect(out.meta.merged).toBe(1)
    })
})

describe('slotRuns', () => {
    it('이어진 것끼리 묶는다', () => {
        expect(slotRuns(['1', '2', '3'])).toEqual([{startSlot: '1', endSlot: '3'}])
    })

    it('이어지지 않으면 나눈다 — 한 구간으로 저장하면 2교시가 조용히 포함된다', () => {
        expect(slotRuns(['1', '3', '5'])).toEqual([
            {startSlot: '1', endSlot: '1'},
            {startSlot: '3', endSlot: '3'},
            {startSlot: '5', endSlot: '5'},
        ])
    })

    it('종례의 자리는 그 줄의 마지막 교시 다음이다', () => {
        expect(slotRuns(['조회', '1', '2', '3', '종례']))
            .toEqual([{startSlot: '조회', endSlot: '종례'}])
        expect(slotRuns(['조회', '1', '2', '3', '4', '5', '6', '7', '종례']))
            .toEqual([{startSlot: '조회', endSlot: '종례'}])
    })

    it('순서가 뒤섞이거나 겹쳐도 같은 결과다', () => {
        expect(slotRuns(['3', '1', '2', '2'])).toEqual([{startSlot: '1', endSlot: '3'}])
    })

    it('빈 목록은 구간이 없다 — 기간 미정으로 남긴다', () => {
        expect(slotRuns([])).toEqual([])
    })
})

describe('교시 표기', () => {
    it('교시 열은 별칭이 아니라 모양으로 알아본다', () => {
        expect(slotTokenOf('조회')).toBe('조회')
        expect(slotTokenOf('3교시')).toBe('3')
        expect(slotTokenOf('종례')).toBe('종례')
        expect(slotTokenOf('비고')).toBeNull()
    })

    it('꼬리 쉼표는 나이스가 늘 붙인다', () => {
        expect(parseSlotList('조회,1교시,종례,')).toEqual(['조회', '1', '종례'])
        expect(parseSlotList('')).toEqual([])
    })
})

describe('splitCodeLabel', () => {
    it('구분과 종류로 나눈다', () => {
        expect(splitCodeLabel('질병조퇴', REASONS, TYPES))
            .toEqual({reasonLabel: '질병', typeLabel: '조퇴'})
    })

    it('긴 것부터 맞춘다 — 출석인정이 인정보다 먼저다', () => {
        expect(splitCodeLabel('출석인정결석', REASONS, TYPES))
            .toEqual({reasonLabel: '출석인정', typeLabel: '결석'})
    })

    it('모르는 표기는 지어내지 않고 비운다', () => {
        expect(splitCodeLabel('공결', REASONS, TYPES))
            .toEqual({reasonLabel: null, typeLabel: null})
    })

    it('후보는 DB에서 온다 — 목록이 비면 아무것도 구분하지 못한다', () => {
        expect(splitCodeLabel('질병조퇴', [], [])).toEqual({reasonLabel: null, typeLabel: null})
    })
})

describe('읽지 못한 것을 알린다', () => {
    it('모르는 출결 표기를 모아 알린다', () => {
        const out = read([
            ['일자', '번호', '성명', '출결구분', '결시교시'],
            ['2026.06.01.(월)', '1', '학생1', '공결', '조회,1교시,종례,'],
        ])
        expect(out.meta.unknownCodes).toEqual(['공결'])
        expect(out.rows[0]).toMatchObject({codeLabel: '공결', reasonLabel: null, typeLabel: null})
    })

    it('일자를 읽지 못한 줄은 조용히 넘기지 않는다', () => {
        const out = read([
            ['일자', '번호', '성명', '출결구분', '결시교시'],
            ['지난주', '1', '학생1', '질병결석', '조회,1교시,종례,'],
        ])
        expect(out.rows).toEqual([])
        expect(out.meta.skipped).toEqual([{line: 2, why: '일자를 읽지 못했습니다.'}])
    })

    it('머리글을 못 찾으면 빈 결과가 아니라 오류다', () => {
        expect(() => read([['무엇인가'], ['다른 것']])).toThrow(/머리글/)
    })
})

describe('mapNeisColumns', () => {
    it('열은 이름으로 찾는다. 위치로 찾지 않는다', () => {
        const {columns} = mapNeisColumns(['일자', '번호', '성명', '출결구분', '결시교시', '사유'])
        expect(columns).toMatchObject({date: 0, number: 1, name: 2, code: 3, slots: 4, detail: 5})
    })

    it('일일출석부의 마감 열이 출결 코드 열이다', () => {
        const {columns, slots} = mapNeisColumns(DAILY_HEAD)
        expect(columns.code).toBe(2)
        expect(slots.map((s) => s.token)).toEqual(['조회', '1', '2', '3', '종례'])
    })

    it('머리글이 아니면 null이다', () => {
        expect(mapNeisColumns(['1', '학생1', '질병결석'])).toBeNull()
    })
})

describe('captionOf', () => {
    it('학년 · 반 · 일자를 한 줄에서 읽는다', () => {
        expect(captionOf(['※ 3학년 6반 2026.09.01.(화)']))
            .toEqual({grade: 3, classNo: 6, date: '2026-09-01'})
    })

    it('캡션이 없으면 null이다', () => {
        expect(captionOf(['번호', '성명'])).toBeNull()
    })
})

describe('실제 xlsx를 열어 본다', () => {
    async function bookOf(table) {
        const book = new ExcelJS.Workbook()
        const sheet = book.addWorksheet('sheet1')
        table.forEach((row, i) => sheet.getRow(i + 1).values = row.length ? row : [null])
        return new Uint8Array(await book.xlsx.writeBuffer())
    }

    it('exceljs로 읽고 어느 파서인지 알린다', async () => {
        const out = await readNeisFile(await bookOf(MONTHLY), AXIS)
        expect(out.meta.parser).toBe('exceljs')
        expect(out.rows).toHaveLength(5)
    })

    it('빈 행이 있어도 줄 번호가 밀리지 않는다', async () => {
        const out = await readNeisFile(await bookOf(DAILY), AXIS)
        expect(out.rows.map((r) => r.line)).toEqual([6, 7, 13])
    })

    it('이름만 xlsx인 파일은 zip 서명으로 먼저 거른다', async () => {
        await expect(readNeisFile(new Uint8Array([0x3c, 0x3f, 0x78, 0x6d]), AXIS))
            .rejects.toThrow(/엑셀/)
    })
})

// ── 대조 ────────────────────────────────────────────────────────

const mine = (over = {}) => ({
    number: 5, name: '학생5', date: '2026-09-10',
    reasonLabel: '질병', typeLabel: '결석', spanText: '하루 종일',
    startSlot: '조회', endSlot: '종례', ...over,
})
const theirs = (over = {}) => ({
    number: 5, name: '학생5', date: '2026-09-10',
    reasonLabel: '질병', typeLabel: '결석', detail: null,
    startSlot: '조회', endSlot: '종례', ...over,
})

describe('compareRecords', () => {
    it('같은 것은 일치로 센다', () => {
        const out = compareRecords([mine()], [theirs()])
        expect(out.same).toHaveLength(1)
        expect(out.diff).toHaveLength(0)
    })

    it('같은 날 같은 학생인데 내용이 다르면 다름이다', () => {
        const out = compareRecords([mine()], [theirs({reasonLabel: '미인정'})])
        expect(out.diff).toHaveLength(1)
        expect(out.diff[0].mine.reasonLabel).toBe('질병')
        expect(out.diff[0].theirs.reasonLabel).toBe('미인정')
    })

    it('축이 같아도 기간이 다르면 다름이다', () => {
        const out = compareRecords(
            [mine({typeLabel: '조퇴', startSlot: '1', endSlot: '종례'})],
            [theirs({typeLabel: '조퇴', startSlot: '5', endSlot: '종례'})],
        )
        expect(out.diff).toHaveLength(1)
    })

    it('앱에만 있으면 나이스 쪽이 비어 있다 — 칸을 없애지 않는다', () => {
        const out = compareRecords([mine()], [])
        expect(out.onlyApp).toHaveLength(1)
        expect(out.onlyApp[0].theirs).toBeNull()
    })

    it('나이스에만 있으면 앱 쪽이 비어 있다', () => {
        const out = compareRecords([], [theirs()])
        expect(out.onlyNeis).toHaveLength(1)
        expect(out.onlyNeis[0].mine).toBeNull()
    })

    it('하루 2구간에서 하나만 어긋나면 나머지는 일치로 남는다', () => {
        const out = compareRecords(
            [mine({typeLabel: '지각', endSlot: '2'}), mine({typeLabel: '조퇴', startSlot: '5'})],
            [theirs({typeLabel: '지각', endSlot: '2'}), theirs({typeLabel: '결과', startSlot: '5'})],
        )
        expect(out.same).toHaveLength(1)
        expect(out.diff).toHaveLength(1)
        expect(out.onlyApp).toHaveLength(0)
    })

    it('다른 날짜는 서로 짝이 되지 않는다', () => {
        const out = compareRecords([mine()], [theirs({date: '2026-09-11'})])
        expect(out.diff).toHaveLength(0)
        expect(out.onlyApp).toHaveLength(1)
        expect(out.onlyNeis).toHaveLength(1)
    })

    it('표기의 띄어쓰기 차이는 흡수한다 — 학교마다 다르게 적는다', () => {
        expect(sameRecord({reasonLabel: '질병', typeLabel: '조퇴'},
            {reasonLabel: '질병', typeLabel: '조 퇴'})).toBe(true)
    })
})

describe('sortPairs', () => {
    const pairs = [
        {mine: mine({number: 12, date: '2026-09-01'}), theirs: null},
        {mine: null, theirs: theirs({number: 3, date: '2026-09-20'})},
    ]

    it('번호순은 번호가 먼저다', () => {
        expect(sortPairs(pairs, 'number').map((p) => (p.mine ?? p.theirs).number)).toEqual([3, 12])
    })

    it('날짜순은 날짜가 먼저다', () => {
        expect(sortPairs(pairs, 'date').map((p) => (p.mine ?? p.theirs).date))
            .toEqual(['2026-09-01', '2026-09-20'])
    })
})

describe('파일 읽기 도구', () => {
    it('이름만 xlsx인 파일을 zip 서명으로 거른다', () => {
        expect(looksLikeXlsx(new Uint8Array([0x50, 0x4b, 0x03, 0x04, 0x00]))).toBe(true)
        expect(looksLikeXlsx(new Uint8Array([0x3c, 0x3f, 0x78, 0x6d]))).toBe(false)
    })

    it('날짜 표기가 제각각이어도 ISO로 모은다', () => {
        expect(toIso('2026.09.10.')).toBe('2026-09-10')
        expect(toIso('2026-9-3')).toBe('2026-09-03')
        expect(toIso('20260910')).toBe('2026-09-10')
        expect(toIso('2026.06.09.(화)')).toBe('2026-06-09')
        expect(toIso(new Date(2026, 8, 10))).toBe('2026-09-10')
    })

    it('읽을 수 없는 날짜는 조용히 넘기지 않고 null로 알린다', () => {
        expect(toIso('지난주')).toBeNull()
        expect(toIso('')).toBeNull()
    })
})

// ── 감사가 찾은 결함들 ──────────────────────────────────────────

describe('삼켜서는 안 되는 줄', () => {
    it('사유에 ※를 쓴 줄을 캡션으로 오인하지 않는다', () => {
        // `※ 진단서 제출`은 한국어 메모에서 흔하다. 캡션으로 삼으면 그 출결이 사라진다.
        const out = read([
            ['일자', '번호', '성명', '출결구분', '결시교시', '사유'],
            ['2026.06.01.(월)', '2', '학생2', '질병결석', '조회,1교시,종례,', '※ 진단서 제출'],
        ])
        expect(out.rows).toHaveLength(1)
        expect(out.rows[0]).toMatchObject({number: 2, detail: '※ 진단서 제출'})
        expect(out.meta.skipped).toEqual([])
    })

    it('일일출석부에서도 ※ 메모가 캡션의 일자를 지우지 않는다', () => {
        // 지우면 그 페이지의 뒤따르는 학생이 전부 "일자를 읽지 못했습니다"가 된다.
        const out = read([
            [' ※ 3학년 6반 2026.09.01.(화)'],
            DAILY_HEAD,
            ['1', '학생1', '질병결석', '/', '/', '/', '/', '/', '※ 진단서 받기로 함'],
            ['2', '학생2', '질병결석', '/', '/', '/', '/', '/', '몸살'],
        ])
        expect(out.rows.map((r) => r.date)).toEqual(['2026-09-01', '2026-09-01'])
        expect(out.meta.skipped).toEqual([])
    })

    it('사유가 `9 / 1`이어도 페이지 꼬리로 보지 않는다', () => {
        const out = read([
            ['일자', '번호', '성명', '출결구분', '결시교시', '사유'],
            ['2026.06.01.(월)', '2', '학생2', '질병결석', '조회,1교시,종례,', '9 / 1'],
        ])
        expect(out.rows).toHaveLength(1)
    })
})

describe('번호 이어받기', () => {
    it('읽을 수 없는 번호를 앞 학생에게 붙이지 않는다', () => {
        // 이어받기는 **세로 병합**을 위한 것이다. 빈칸이 아닌데 못 읽었다면 그것은
        // 오류이고, 앞 학생에게 붙이면 엉뚱한 학생이 결석 처리된다.
        const out = read([
            ['일자', '번호', '성명', '출결구분', '결시교시', '사유'],
            ['2026.06.01.(월)', '2', '학생2', '질병결석', '조회,1교시,종례,', ''],
            ['2026.06.03.(수)', '３', '학생3', '질병지각', '조회,', ''],
        ])
        expect(out.rows).toHaveLength(1)
        expect(out.rows[0].number).toBe(2)
        expect(out.meta.skipped).toEqual([{line: 3, why: '번호를 읽지 못했습니다: ３'}])
    })

    it('빈칸은 세로 병합이므로 그대로 이어받는다', () => {
        const out = read(MONTHLY)
        expect(out.rows[1]).toMatchObject({number: 2, name: '학생2'})
    })

    it('페이지가 바뀌면 앞 페이지의 마지막 학생을 이어받지 않는다', () => {
        const out = read([
            [' ※ 3학년 6반 2026.09.01.(화)'],
            DAILY_HEAD,
            ['1', '학생1', '질병결석', '/', '/', '/', '/', '/', ''],
            [' ※ 3학년 6반 2026.09.01.(화)'],
            DAILY_HEAD,
            ['', '', '질병결석', '/', '/', '/', '/', '/', ''],
        ])
        expect(out.rows.map((r) => r.number)).toEqual([1])
        expect(out.meta.skipped).toEqual([{line: 6, why: '번호를 읽지 못했습니다.'}])
    })
})

describe('종례의 자리', () => {
    it('마지막 교시부터의 조퇴를 쪼개지 않는다 — 월별 파일에서 가장 흔한 줄이다', () => {
        // 분리하면 `7교시~7교시`와 `종례~종례` 두 건이 되어 이미 올바르게 저장된
        // `7교시~종례`와 어긋나고, `종례~종례` 조퇴는 고르개가 표현할 수도 없다.
        expect(slotRuns(['7', '종례'])).toEqual([{startSlot: '7', endSlot: '종례'}])
        expect(slotRuns(['2', '종례'])).toEqual([{startSlot: '2', endSlot: '종례'}])
        expect(slotRuns(['6', '7', '종례'])).toEqual([{startSlot: '6', endSlot: '종례'}])
    })

    it('그날의 슬롯을 모르면 흔한 쪽으로 읽는다 — 그날이 거기서 끝났다', () => {
        // `조회,1교시,종례`는 교시가 하나뿐인 날의 하루 종일일 수도, 1교시까지 지각하고
        // 종례를 더 빠진 날일 수도 있다. 파일만으로는 구별되지 않아 흔한 쪽으로 읽는다.
        // 실제 파일에서 그날 교시 수가 3 · 6 · 7로 제각각이었던 것이 그 근거다.
        expect(slotRuns(['조회', '1', '종례'])).toEqual([{startSlot: '조회', endSlot: '종례'}])
        expect(slotRuns(['조회', '1', '2', '3', '종례']))
            .toEqual([{startSlot: '조회', endSlot: '종례'}])
        expect(slotRuns(['조회', '종례'])).toEqual([{startSlot: '조회', endSlot: '종례'}])
    })

    it('파일이 그날의 슬롯을 보여주면 가정하지 않는다 — 일일출석부 머리글', () => {
        const day = ['조회', '1', '2', '3', '4', '5', '6', '7', '종례']
        // 7교시까지 있는 날이라 종례는 1교시 옆이 아니다.
        expect(slotRuns(['조회', '1', '종례'], day)).toEqual([
            {startSlot: '조회', endSlot: '1'},
            {startSlot: '종례', endSlot: '종례'},
        ])
        // 마지막 교시부터 나간 조퇴는 한 구간이다.
        expect(slotRuns(['7', '종례'], day)).toEqual([{startSlot: '7', endSlot: '종례'}])
        // 그날 교시가 하나뿐이면 조회~종례가 맞다.
        expect(slotRuns(['조회', '1', '종례'], ['조회', '1', '종례']))
            .toEqual([{startSlot: '조회', endSlot: '종례'}])
    })

    it('월별 파일의 마지막 교시 조퇴는 한 건으로 들어온다', () => {
        const out = read([
            ['일자', '번호', '성명', '출결구분', '결시교시', '사유'],
            ['2026.06.01.(월)', '2', '학생2', '질병조퇴', '7교시,종례,', '7교시부터 조퇴'],
        ])
        expect(out.rows.map((r) => [r.startSlot, r.endSlot])).toEqual([['7', '종례']])
        expect(out.rows[0].merged).toBe(false)
        expect(out.meta.merged).toBe(0)
    })

    it('합쳐진 줄로 세는 것은 실제로 떨어져 있을 때뿐이다', () => {
        // 나이스가 하루 두 구간을 한 줄로 내보낸 실제 모양. 사이가 비어 있다.
        const out = read([
            ['일자', '번호', '성명', '출결구분', '결시교시', '사유'],
            ['2026.06.01.(월)', '2', '학생2', '질병조퇴', '조회,1교시,6교시,7교시,종례,', ''],
        ])
        expect(out.rows.map((r) => [r.startSlot, r.endSlot]))
            .toEqual([['조회', '1'], ['6', '종례']])
        expect(out.meta.merged).toBe(1)
    })

    it('일일출석부는 머리글이 그날의 교시를 보여주므로 나눌 곳을 안다', () => {
        const head = ['번호', '성명', '마감', '조회', '1교시', '2교시', '3교시', '4교시', '종례']
        const out = read([
            [' ※ 3학년 6반 2026.09.01.(화)'],
            head,
            ['1', '학생1', '질병지각', '/', '/', '', '', '', '/'],
        ])
        expect(out.rows.map((r) => [r.startSlot, r.endSlot]))
            .toEqual([['조회', '1'], ['종례', '종례']])
    })

    it('결석은 기간을 묻지 않는다 — 결시교시가 어떻게 적혀 있든 한 건이다', () => {
        // 앱도 Rust도 결석을 조회~종례로 저장한다(`ranges_for` · `day_slots`).
        // 여기서 나누면 같은 하루가 두 건이 되고, 빈 칸이면 기간 미정으로 어긋난다.
        const out = read([
            ['일자', '번호', '성명', '출결구분', '결시교시', '사유'],
            ['2026.06.01.(월)', '2', '학생2', '질병결석', '조회,1교시,종례,', ''],
            ['2026.06.02.(화)', '3', '학생3', '질병결석', '', ''],
        ])
        expect(out.rows.map((r) => [r.startSlot, r.endSlot]))
            .toEqual([['조회', '종례'], ['조회', '종례']])
        expect(out.rows.every((r) => r.spanText === '하루 종일')).toBe(true)
        expect(out.meta.merged).toBe(0)
    })
})

describe('0교시', () => {
    it('0교시를 조회로 고쳐 읽지 않는다', () => {
        // 순서값이 조회와 같아 이름이 조회로 바뀐다. 0교시를 쓰는 학교가 실제로 있다.
        expect(slotTokenOf('0교시')).toBeNull()
        expect(unsupportedSlotText('0교시')).toBe('0교시')
        expect(unsupportedSlotText('1교시')).toBeNull()
        expect(unsupportedSlotsIn('0교시,1교시,')).toEqual(['0교시'])
    })

    it('0교시가 섞인 줄은 빼고 읽지 않고 이유를 남긴다', () => {
        // 빼고 읽으면 기간이 한 칸 밀린 채 저장된다. 교사가 직접 넣도록 알린다.
        const out = read([
            ['일자', '번호', '성명', '출결구분', '결시교시', '사유'],
            ['2026.06.01.(월)', '2', '학생2', '질병지각', '0교시,1교시,', ''],
        ])
        expect(out.rows).toEqual([])
        expect(out.meta.skipped)
            .toEqual([{line: 2, why: '이 앱이 다루지 못하는 교시입니다: 0교시'}])
    })

    it('일일출석부의 0교시 열에 찍힌 표시도 조용히 사라지지 않는다', () => {
        const head = ['번호', '성명', '마감', '0교시', '조회', '1교시', '2교시', '종례', '비고']
        const out = read([
            [' ※ 3학년 6반 2026.09.01.(화)'],
            head,
            ['1', '학생1', '질병지각', '/', '/', '', '', '', ''],
        ])
        expect(out.rows).toEqual([])
        expect(out.meta.skipped)
            .toEqual([{line: 3, why: '이 앱이 다루지 못하는 교시입니다: 0교시'}])
    })
})

describe('표기를 다듬는 자리', () => {
    it('띄어쓰기는 파일과 DB 양쪽에서 지운다 — `출석 인정`으로 저장한 학교가 있다', () => {
        const reasons = [{label: '질병'}, {label: '출석 인정'}]
        const types = [{label: '결석', slotPrompt: 'none'}, {label: '조 퇴', slotPrompt: 'start'}]
        // 돌려주는 것은 **DB에 적힌 그대로**다. 정규화한 쪽을 넘기면 Rust가 축을 못 찾는다.
        expect(splitCodeLabel('출석인정결석', reasons, types))
            .toEqual({reasonLabel: '출석 인정', typeLabel: '결석'})
        expect(splitCodeLabel('질병조퇴', reasons, types))
            .toEqual({reasonLabel: '질병', typeLabel: '조 퇴'})
        // 파일이 통째로 "모르는 출결 표기"로 쌓이던 자리다.
        const out = buildRecords([
            ['일자', '번호', '성명', '출결구분', '결시교시'],
            ['2026.06.01.(월)', '1', '학생1', '출석인정결석', '조회,1교시,종례,'],
        ], {reasons, types})
        expect(out.meta.unknownCodes).toEqual([])
        expect(out.rows[0]).toMatchObject({reasonLabel: '출석 인정', typeLabel: '결석'})
    })

    it('교시 열의 한두 글자는 무엇이든 결시로 읽는다 — 본 서식이 하나라는 가정이다', () => {
        // 확인한 일일출석부는 `/`를 쓴다. 표기를 목록으로 두지 않으므로 다른 글자도
        // 결시다. 나이스가 출석을 표시하는 서식을 내보내면 여기가 먼저 틀린다.
        const out = read([
            [' ※ 3학년 6반 2026.09.01.(화)'],
            DAILY_HEAD,
            ['1', '학생1', '질병지각', '결', '', '', '', '', ''],
        ])
        expect(out.rows[0]).toMatchObject({startSlot: '조회', endSlot: '조회'})
    })
})

describe('구분하지 못한 표기', () => {
    it('모르는 출결 표기를 미정 기록과 일치라고 말하지 않는다', () => {
        // Rust의 `resolve`는 같은 줄을 "모르는 출결 표기"로 돌려보낸다. 여기서만
        // 일치라고 하면 검증 화면과 가져오기 화면이 한 줄을 두고 다른 말을 한다.
        const unread = theirs({codeLabel: '공결', reasonLabel: null, typeLabel: null})
        const undecided = mine({reasonLabel: null, typeLabel: null})
        expect(sameRecord(undecided, unread)).toBe(false)

        const out = compareRecords([undecided], [unread])
        expect(out.same).toHaveLength(0)
        expect(out.diff).toHaveLength(1)
    })

    it('출결 표기가 아예 없는 줄은 그대로 비교한다 — 모르는 것이 아니라 없는 것이다', () => {
        expect(sameRecord(mine({reasonLabel: null, typeLabel: null}),
            theirs({codeLabel: null, reasonLabel: null, typeLabel: null}))).toBe(true)
    })
})

describe('파서 둘', () => {
    it('둘 다 실패하면 두 이유를 모두 말한다', async () => {
        // zip 서명은 맞지만 엑셀이 아닌 파일.
        const bytes = new Uint8Array(64)
        bytes.set([0x50, 0x4b, 0x03, 0x04])
        await expect(readNeisFile(bytes, AXIS)).rejects.toThrow(/exceljs:.*SheetJS:/s)
    })
})
