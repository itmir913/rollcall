/**
 * 학년도와 달력 연도의 변환.
 *
 * 학년도는 3월에 열린다. 그래서 학년도와 달만 가지고는 달력 연도가 정해지지
 * 않는다 — 2026학년도 1월은 2027년이다. 월 필터가 고른 달을 날짜로 바꿀 때
 * 이 차이를 놓치면 1·2월 기록이 한 해 전 날짜로 조회된다.
 *
 * 날짜가 속한 학년도를 되돌리는 계산은 여기에 두지 않는다. 그것은 Rust의
 * `due::academic_year_of`가 하고, 화면은 이미 학년도를 알고 있는 상태로 들어온다.
 */

/** 학년도와 달로 실제 달력 연도를 만든다. 1·2월은 이듬해다. */
export function calendarYearOf(academicYear, month) {
    return month >= 3 ? academicYear : academicYear + 1
}

/** 월 필터의 차례. 학년도가 3월에 열리므로 1월·2월이 뒤에 온다. */
export const MONTHS = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 1, 2]

/**
 * 오늘이 속한 학년도. **3월에 열린다.**
 *
 * 1월과 2월은 지난해에 열린 학년도의 끝자락이라 한 해를 뺀다 — 2027년 2월은
 * 2026학년도다. 첫 실행에서 교사에게 되묻지 않고 이 값으로 채운다.
 */
export function academicYearOf(iso) {
    const [year, month] = String(iso).split('-').map(Number)
    return month >= 3 ? year : year - 1
}

/** 그 학년도가 열리고 닫히는 날. 3월 1일부터 이듬해 2월 말일까지다. */
export function yearSpanOf(academicYear) {
    // 2월의 마지막 날은 해마다 다르다. 3월 1일에서 하루를 빼면 윤년을 따로 세지 않는다.
    const pad = (n) => String(n).padStart(2, '0')
    const end = new Date(Date.UTC(academicYear + 1, 2, 1))
    end.setUTCDate(end.getUTCDate() - 1)
    return {
        startsOn: `${academicYear}-03-01`,
        endsOn: `${end.getUTCFullYear()}-${pad(end.getUTCMonth() + 1)}-${pad(end.getUTCDate())}`,
    }
}
