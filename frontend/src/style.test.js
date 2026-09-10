/**
 * 화면 규칙을 기계가 지킨다.
 *
 * 사람이 눈으로 지키는 규칙은 반드시 샌다. 특히 두 가지가 그렇다.
 *
 *   1. **글자가 작아지는 것.** 한 곳에서 0.85rem을 쓰면 다음 화면이 그걸 보고 따라 한다.
 *      교사가 쓰는 프로그램이고, 30행짜리 격자를 하루에 몇 번씩 훑는다. 최소는 16px다.
 *   2. **색이 컴포넌트로 새는 것.** `#fff` 하나가 라이트에서는 멀쩡히 보이므로 리뷰를 통과하고,
 *      다크에서만 흰 바탕에 흰 글자가 된다. 색은 style.css의 토큰에만 있어야 한다.
 *
 * 규칙의 근거는 CLAUDE.md의 UI 규칙에 적혀 있다. 여기서는 그것을 강제만 한다.
 */
import {describe, expect, it} from 'vitest'
import {readdirSync, readFileSync, statSync} from 'node:fs'
import {dirname, join, relative} from 'node:path'
import {fileURLToPath} from 'node:url'

const SRC = dirname(fileURLToPath(import.meta.url))

/** 색 리터럴이 허용된 단 하나의 파일. 여기가 토큰의 집이다. */
const TOKEN_FILE = 'style.css'

/** 본문 최소 크기. text-base = 16px. */
const MIN_PX = 16

function sourceFiles(dir = SRC, found = []) {
    for (const entry of readdirSync(dir)) {
        if (entry === 'node_modules') continue
        const full = join(dir, entry)
        if (statSync(full).isDirectory()) {
            sourceFiles(full, found)
        } else if (/\.(vue|css|js)$/.test(entry) && !/\.test\.js$/.test(entry)) {
            found.push(full)
        }
    }
    return found
}

/** 파일을 {경로, 본문, 줄} 로 읽는다. 경로는 frontend/src 기준 상대경로. */
const FILES = sourceFiles().map((path) => ({
    path: relative(SRC, path).replace(/\\/g, '/'),
    text: readFileSync(path, 'utf-8'),
}))

/** 몇 번째 줄인지. 실패 메시지에 줄 번호가 없으면 찾느라 시간을 쓴다. */
function lineOf(text, index) {
    return text.slice(0, index).split('\n').length
}

/**
 * font-size 값이 16px 미만인가. 판단할 수 없는 값(var, inherit, calc)은 통과시킨다 —
 * 토큰을 쓰라는 것이 규칙이므로 토큰 사용을 막으면 안 된다.
 */
function tooSmall(value) {
    const v = value.trim()
    const match = /^(\d*\.?\d+)(px|rem|em|%)$/.exec(v)
    if (!match) return false
    const size = Number(match[1])
    switch (match[2]) {
        case 'px':
            return size < MIN_PX
        case '%':
            return size < 100
        default: // rem, em — 뿌리 글꼴이 16px이다
            return size < 1
    }
}

describe('글자 크기', () => {
    it('font-size가 16px보다 작은 곳이 없다', () => {
        const offenders = []
        for (const {path, text} of FILES) {
            for (const m of text.matchAll(/font-size\s*:\s*([^;}\n]+)/g)) {
                if (tooSmall(m[1])) {
                    offenders.push(`${path}:${lineOf(text, m.index)} — font-size: ${m[1].trim()}`)
                }
            }
        }
        expect(
            offenders,
            `본문보다 작은 글자는 쓰지 않는다. 작아 보여야 하는 자리는 크기 대신 색(--c-ink-3)과 ` +
            `자간으로 누른다. 밀도가 필요하면 행 높이와 여백을 줄인다.\n${offenders.join('\n')}`,
        ).toEqual([])
    })

    it('text-sm · text-xs 같은 작은 유틸리티 클래스를 쓰지 않는다', () => {
        const offenders = []
        for (const {path, text} of FILES) {
            for (const m of text.matchAll(/\btext-(xs|sm)\b/g)) {
                offenders.push(`${path}:${lineOf(text, m.index)} — text-${m[1]}`)
            }
            // text-[13px] 처럼 임의 값으로 우회하는 경우도 같이 막는다.
            for (const m of text.matchAll(/\btext-\[(\d*\.?\d+)(px|rem)]/g)) {
                if (tooSmall(m[1] + m[2])) {
                    offenders.push(`${path}:${lineOf(text, m.index)} — text-[${m[1]}${m[2]}]`)
                }
            }
        }
        expect(offenders, `text-base가 최소 크기다.\n${offenders.join('\n')}`).toEqual([])
    })
})

describe('색', () => {
    it('색 리터럴은 style.css에만 있다', () => {
        const offenders = []
        for (const {path, text} of FILES) {
            if (path === TOKEN_FILE) continue
            const patterns = [/#[0-9a-fA-F]{3,8}\b/g, /\brgba?\s*\(/g, /\bhsla?\s*\(/g]
            for (const pattern of patterns) {
                for (const m of text.matchAll(pattern)) {
                    offenders.push(`${path}:${lineOf(text, m.index)} — ${m[0]}`)
                }
            }
        }
        expect(
            offenders,
            `색은 style.css의 토큰에만 둔다. 컴포넌트에 적힌 색은 한쪽 테마에서만 읽힌다. ` +
            `필요한 색이 토큰에 없으면 토큰을 추가하라.\n${offenders.join('\n')}`,
        ).toEqual([])
    })

    it('인라인 style 속성으로 색·글자 크기를 정하지 않는다', () => {
        const offenders = []
        for (const {path, text} of FILES) {
            // style="..." 와 :style="{...}" 둘 다 본다. 폭·정렬 같은 배치 값은 허용한다.
            for (const m of text.matchAll(/:?style\s*=\s*"([^"]*)"/g)) {
                if (/color|background|font-?[Ss]ize/.test(m[1])) {
                    offenders.push(`${path}:${lineOf(text, m.index)} — ${m[0].slice(0, 60)}`)
                }
            }
        }
        expect(
            offenders,
            `인라인 style에 색이나 글자 크기를 적으면 테마 전환에서 그 부분만 남는다. ` +
            `클래스와 토큰으로 옮겨라.\n${offenders.join('\n')}`,
        ).toEqual([])
    })
})

describe('토큰 파일', () => {
    it('style.css는 prefers-color-scheme 미디어 쿼리를 쓰지 않는다', () => {
        const tokens = FILES.find((f) => f.path === TOKEN_FILE)
        expect(tokens, 'style.css를 찾지 못했다').toBeDefined()
        expect(
            /@media[^{]*prefers-color-scheme/.test(tokens.text),
            '테마 상태는 data-theme 한 곳에만 있어야 한다. 미디어 쿼리를 쓰면 토글과 시스템 설정이 어긋난다.',
        ).toBe(false)
    })

    it('다크 테마가 라이트에서 정의한 토큰을 모두 다시 정의한다', () => {
        const tokens = FILES.find((f) => f.path === TOKEN_FILE)
        const block = (selector) => {
            const start = tokens.text.indexOf(selector)
            const open = tokens.text.indexOf('{', start)
            const close = tokens.text.indexOf('}', open)
            return [...tokens.text.slice(open, close).matchAll(/(--[\w-]+)\s*:/g)].map((m) => m[1])
        }
        const light = block(':root {')
        const dark = block('html[data-theme="dark"]')
        const missing = light.filter((name) => !dark.includes(name))
        expect(
            missing,
            `다크에서 값이 빠진 토큰은 라이트 값을 그대로 쓴다. 어두운 바탕에 밝은 바탕용 색이 남는다.\n${missing.join(', ')}`,
        ).toEqual([])
    })
})
