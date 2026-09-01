import {describe, expect, it} from 'vitest'
import {academicYearOf} from '../stores/app.js'

describe('academicYearOf', () => {
    it('3월부터 그 해가 학년도다', () => {
        expect(academicYearOf(new Date(2026, 2, 1))).toBe(2026)  // 3월
        expect(academicYearOf(new Date(2026, 11, 31))).toBe(2026) // 12월
    })

    it('1~2월은 아직 지난해 학년도다', () => {
        // getFullYear()를 그대로 쓰면 2027년 2월에 "2027학년도"를 만들어 버린다.
        expect(academicYearOf(new Date(2027, 0, 5))).toBe(2026)  // 1월
        expect(academicYearOf(new Date(2027, 1, 28))).toBe(2026) // 2월
    })
})
