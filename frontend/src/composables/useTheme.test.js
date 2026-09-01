import {describe, expect, it, vi} from 'vitest'
import {resolveTheme} from './useTheme.js'

function stubMatchMedia(dark) {
    vi.stubGlobal('window', {
        matchMedia: () => ({matches: dark, addEventListener: () => {}}),
    })
}

describe('resolveTheme', () => {
    it('기본값과 light는 시스템 설정과 무관하게 라이트다', () => {
        stubMatchMedia(true)
        expect(resolveTheme('light')).toBe('light')
        expect(resolveTheme(undefined)).toBe('light')
        expect(resolveTheme('아무거나')).toBe('light')
    })

    it('dark는 항상 다크다', () => {
        stubMatchMedia(false)
        expect(resolveTheme('dark')).toBe('dark')
    })

    it('system일 때만 OS 설정을 따른다', () => {
        stubMatchMedia(true)
        expect(resolveTheme('system')).toBe('dark')
        stubMatchMedia(false)
        expect(resolveTheme('system')).toBe('light')
    })
})
