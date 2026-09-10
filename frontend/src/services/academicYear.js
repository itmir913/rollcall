/**
 * 학년도 계산. Rust의 `due::academic_year_of` · `semester_of`와 같은 규칙이다.
 *
 * 학년도는 3월에 열린다. `getFullYear()`를 그대로 쓰면 2027년 2월에 "2027학년도"를
 * 만들어 버리는데, 그날은 아직 2026학년도다.
 */

/** 그 날짜가 속한 학년도. 1·2월은 전년도다. */
export function academicYearOf(date) {
    return date.getMonth() + 1 >= 3 ? date.getFullYear() : date.getFullYear() - 1
}

/** 학년도와 달로 실제 달력 연도를 만든다. 1·2월은 이듬해다. */
export function calendarYearOf(academicYear, month) {
    return month >= 3 ? academicYear : academicYear + 1
}

/** 3~8월이 1학기, 9~2월이 2학기다. */
export function semesterOf(month) {
    return month >= 3 && month <= 8 ? 1 : 2
}

/** 월 필터의 차례. 학년도가 3월에 열리므로 1월·2월이 뒤에 온다. */
export const MONTHS = [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 1, 2]
