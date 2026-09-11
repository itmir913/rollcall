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
 *
 * **검사는 값의 모양이 아니라 결과로 판단한다.** `#fff`만 막고 `white`를 통과시키면
 * 규칙이 아니라 표기법을 막는 것이다. 둘은 다크에서 똑같이 깨진다. 같은 이유로
 * `font: 12px Pretendard`(단축 속성)와 `text-[0.8em]`(임의 값)도 `font-size: 12px`와
 * 같이 취급한다. 우회로를 하나 열어 두면 규칙은 그 길로만 샌다.
 *
 * 맨 아래 `검사기 자체` 묶음이 이 파일의 검사기를 표본으로 시험한다. 저장소가 깨끗할 때
 * 검사기가 고장 나면 아무도 모르기 때문이다 — 통과는 "위반이 없다"여야지
 * "검사기가 아무것도 못 찾는다"여서는 안 된다.
 */
import {describe, expect, it} from 'vitest'
import {readdirSync, readFileSync, statSync} from 'node:fs'
import {dirname, join, relative} from 'node:path'
import {fileURLToPath} from 'node:url'

const SRC = dirname(fileURLToPath(import.meta.url))

/** 줄바꿈. 역슬래시 이스케이프가 편집 중에 뭉개지는 것을 피한다. */
const NL = String.fromCharCode(10)

/** 색 리터럴이 허용된 단 하나의 파일. 여기가 토큰의 집이다. */
const TOKEN_FILE = 'style.css'

/** 본문 최소 크기. text-base = 16px. */
const MIN_PX = 16

/** 뿌리 글꼴 크기. rem·em·%를 px로 환산할 때 쓴다. 지금은 최소 크기와 같은 값이다. */
const ROOT_PX = 16

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

// ── 글자 크기 ────────────────────────────────────────────────

/**
 * 길이 단위를 px로 환산한다. px·rem·em·%만 알던 검사를 넓힌 것이다 —
 * `text-[12pt]`처럼 단위를 바꾸면 그대로 통과하던 구멍이 있었다.
 */
const UNIT_PX = {
    px: 1,
    pt: 96 / 72,
    pc: 16,
    in: 96,
    cm: 96 / 2.54,
    mm: 96 / 25.4,
    q: 96 / 101.6,
    rem: ROOT_PX,
    em: ROOT_PX,
    ex: ROOT_PX / 2,
    ch: ROOT_PX / 2,
    '%': ROOT_PX / 100,
}

/**
 * 뷰포트 단위는 창 크기에 따라 값이 변한다. 창을 줄이면 16px 아래로 내려가므로
 * "16px보다 크다"를 확인할 방법이 없다. 확인할 수 없는 값은 통과가 아니라 위반이다 —
 * 크기가 필요하면 `--t-*` 토큰을 쓴다.
 */
const VIEWPORT_UNIT = /^(vw|vh|vmin|vmax|svw|svh|lvw|lvh|dvw|dvh)$/

/** 값 안의 길이 리터럴. 대문자(`14PX`)도 잡으려고 i를 붙인다. */
const LENGTH = /(\d*\.?\d+)\s*(px|pt|pc|in|cm|mm|q|rem|em|ex|ch|vmin|vmax|vw|vh|%)(?![\w%])/gi

/**
 * 글자 크기 값에서 16px 미만인 리터럴을 뽑는다.
 *
 * 값 전체가 하나의 리터럴일 때만 보지 않고 **값 안의 리터럴을 전부** 본다.
 * `clamp(12px, 2vw, 20px)`의 아래끝이나 `font: 12px Pretendard`의 크기처럼
 * 리터럴이 값 가운데 섞여 있는 경우가 실제 우회로이기 때문이다.
 * `calc(1rem + 2px)`처럼 더해서 결국 커지는 식도 함께 잡히는데, 그것은 의도한 것이다 —
 * 읽는 사람이 계산해야 하는 크기는 애초에 토큰으로 적어야 한다.
 *
 * `var(--t-base)`·`inherit` 같은 값은 리터럴이 없으므로 그냥 통과한다.
 * 토큰을 쓰라는 것이 규칙이므로 토큰 사용을 막으면 안 된다.
 */
function smallSizes(value) {
    const found = []
    const v = value.trim()
    // `font-size: 0`은 단위가 없어도 0px다.
    if (v === '0') found.push('0')
    for (const m of v.matchAll(LENGTH)) {
        const unit = m[2].toLowerCase()
        if (VIEWPORT_UNIT.test(unit)) {
            found.push(`${m[0]} (창 크기에 따라 달라져 16px을 보장할 수 없다)`)
        } else if (Number(m[1]) * UNIT_PX[unit] < MIN_PX) {
            found.push(m[0])
        }
    }
    return found
}

/**
 * 글자 크기를 설정하는 선언. `font-size` 외에 세 가지를 더 본다.
 *   - `font:` 단축 속성 — 크기가 그 안에 들어 있어 `font-size`만 훑으면 통째로 빠져나간다.
 *   - `fontSize` — Vue의 `:style` 객체는 camelCase로 적는다.
 *   - `--t-*` — 토큰 파일이 애초에 16px보다 작은 크기 토큰을 만들지 못하게 막는다.
 * 긴 이름을 먼저 적어야 `font`가 `font-size`를 가로채지 않는다.
 */
const SIZE_DECL = /(?<![-\w])(font-size|fontSize|font|--t-[\w-]+)\s*:\s*([^;}\n]*)/gi

/** Tailwind 임의 값 유틸리티. `text-[12pt]`처럼 괄호 안으로 크기·색이 숨는다. */
const TW_ARBITRARY = /\b(bg|text|border|ring|outline|fill|stroke|from|via|to|decoration|divide|placeholder|caret|accent|shadow)-\[([^\]]*)]/g

/** 파일 하나에서 16px 미만 글자 크기를 찾는다. */
function sizeOffenders(path, text) {
    const found = []
    for (const m of text.matchAll(SIZE_DECL)) {
        for (const bad of smallSizes(m[2])) {
            found.push(`${path}:${lineOf(text, m.index)} — ${m[1]}: ${m[2].trim()} (${bad})`)
        }
    }
    for (const m of text.matchAll(/\btext-(xs|sm)\b/g)) {
        found.push(`${path}:${lineOf(text, m.index)} — text-${m[1]}`)
    }
    for (const m of text.matchAll(TW_ARBITRARY)) {
        if (m[1] !== 'text') continue
        // `text-[length:0.8em]`처럼 앞에 자료형이 붙는 표기도 있다.
        for (const bad of smallSizes(m[2].replace(/^[a-z-]+:/i, '').replace(/_/g, ' '))) {
            found.push(`${path}:${lineOf(text, m.index)} — ${m[0]} (${bad})`)
        }
    }
    return found
}

// ── 색 ────────────────────────────────────────────────

/**
 * 이름으로 적는 색. `color: white`는 `#fff`와 똑같이 다크에서 흰 바탕에 흰 글자가 된다.
 * 표기가 다를 뿐 같은 위반이라 같이 막는다.
 *
 * `transparent`·`currentColor`·`none`·`inherit`는 일부러 넣지 않았다. 이 넷은 고정된 색이
 * 아니라 "색 없음" 또는 "물려받은 색"이라, 테마가 바뀌면 함께 바뀐다. 규칙이 막으려는 것은
 * 테마를 따라가지 않는 값이다.
 */
const NAMED_COLORS = [
    'aliceblue', 'antiquewhite', 'aqua', 'aquamarine', 'azure', 'beige', 'bisque', 'black',
    'blanchedalmond', 'blue', 'blueviolet', 'brown', 'burlywood', 'cadetblue', 'chartreuse',
    'chocolate', 'coral', 'cornflowerblue', 'cornsilk', 'crimson', 'cyan', 'darkblue',
    'darkcyan', 'darkgoldenrod', 'darkgray', 'darkgreen', 'darkgrey', 'darkkhaki',
    'darkmagenta', 'darkolivegreen', 'darkorange', 'darkorchid', 'darkred', 'darksalmon',
    'darkseagreen', 'darkslateblue', 'darkslategray', 'darkslategrey', 'darkturquoise',
    'darkviolet', 'deeppink', 'deepskyblue', 'dimgray', 'dimgrey', 'dodgerblue', 'firebrick',
    'floralwhite', 'forestgreen', 'fuchsia', 'gainsboro', 'ghostwhite', 'gold', 'goldenrod',
    'gray', 'green', 'greenyellow', 'grey', 'honeydew', 'hotpink', 'indianred', 'indigo',
    'ivory', 'khaki', 'lavender', 'lavenderblush', 'lawngreen', 'lemonchiffon', 'lightblue',
    'lightcoral', 'lightcyan', 'lightgoldenrodyellow', 'lightgray', 'lightgreen', 'lightgrey',
    'lightpink', 'lightsalmon', 'lightseagreen', 'lightskyblue', 'lightslategray',
    'lightslategrey', 'lightsteelblue', 'lightyellow', 'lime', 'limegreen', 'linen', 'magenta',
    'maroon', 'mediumaquamarine', 'mediumblue', 'mediumorchid', 'mediumpurple',
    'mediumseagreen', 'mediumslateblue', 'mediumspringgreen', 'mediumturquoise',
    'mediumvioletred', 'midnightblue', 'mintcream', 'mistyrose', 'moccasin', 'navajowhite',
    'navy', 'oldlace', 'olive', 'olivedrab', 'orange', 'orangered', 'orchid', 'palegoldenrod',
    'palegreen', 'paleturquoise', 'palevioletred', 'papayawhip', 'peachpuff', 'peru', 'pink',
    'plum', 'powderblue', 'purple', 'rebeccapurple', 'red', 'rosybrown', 'royalblue',
    'saddlebrown', 'salmon', 'sandybrown', 'seagreen', 'seashell', 'sienna', 'silver',
    'skyblue', 'slateblue', 'slategray', 'slategrey', 'snow', 'springgreen', 'steelblue',
    'tan', 'teal', 'thistle', 'tomato', 'turquoise', 'violet', 'wheat', 'white', 'whitesmoke',
    'yellow', 'yellowgreen',
]

/**
 * 앞뒤가 하이픈·글자면 색 이름이 아니다. `white-space: nowrap`의 `white`,
 * `--c-white` 같은 토큰 이름을 색으로 읽지 않게 한다.
 */
const NAMED_RE = new RegExp(`(?<![-\\w])(${NAMED_COLORS.join('|')})(?![-\\w])`, 'gi')

/**
 * 색을 받는 속성. 이름색 검사는 **이 속성의 값 안에서만** 한다 —
 * `white`·`red` 같은 낱말은 흔한 영어 단어라 파일 전체를 훑으면 오탐이 쏟아진다.
 * Vue의 `:style` 객체를 위해 camelCase 표기도 함께 만든다.
 */
const COLOR_PROPS = [
    'color', 'background', 'background-color', 'background-image',
    'border', 'border-color', 'border-top', 'border-right', 'border-bottom', 'border-left',
    'border-top-color', 'border-right-color', 'border-bottom-color', 'border-left-color',
    'outline', 'outline-color', 'fill', 'stroke', 'stop-color', 'flood-color',
    'box-shadow', 'text-shadow', 'caret-color', 'accent-color', 'text-decoration-color',
    'column-rule', 'column-rule-color', 'scrollbar-color',
]

/** 긴 이름이 먼저 와야 `border`가 `border-color`를 가로채지 않는다. */
const PROP_ALT = COLOR_PROPS
    .flatMap((p) => (p.includes('-') ? [p, p.replace(/-(\w)/g, (_, c) => c.toUpperCase())] : [p]))
    .sort((a, b) => b.length - a.length)
    .join('|')

/** `color: white`(CSS)와 `fill="white"`(SVG 속성)를 함께 본다. */
const COLOR_DECL = new RegExp(`(?<![-\\w])(${PROP_ALT})\\s*[:=]\\s*([^;}\\n]*)`, 'gi')

/**
 * 값 자체가 색 리터럴인 모양. 오탐이 없으므로 파일 전체에서 찾는다.
 * `oklch()`·`lab()`·`lch()`·`color()`는 채널 값을 직접 적는 함수라 언제나 리터럴이다.
 */
const COLOR_LITERALS = [
    /#[0-9a-fA-F]{3,8}\b/g,
    /\brgba?\s*\(/gi,
    /\bhsla?\s*\(/gi,
    /\b(oklch|oklab|lch|lab)\s*\(/gi,
    /(?<![-\w])color\s*\(/gi,
]

/**
 * Tailwind v4가 설치돼 있으므로 `class="bg-red-600"`은 실제로 동작하는 색이다.
 * 토큰을 거치지 않아 다크에서 그대로 남는다 — `#fff`와 같은 위반이다.
 */
const TW_PALETTE = 'slate|gray|grey|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black'
const TW_COLOR = new RegExp(`\\b(bg|text|border|ring|outline|fill|stroke|from|via|to|decoration|divide|placeholder|caret|accent|shadow)-(${TW_PALETTE})\\b(-\\d{1,3})?`, 'g')

/**
 * 값에서 토큰 참조와 파일 경로를 지운다.
 *
 * `var(--c-accent)`는 토큰이라 검사 대상이 아니고, `url(red-icon.svg)`의 `red`는 색이 아니다.
 * 지우고 나서 남는 것만 본다. 그래서 토큰만으로 만든 `color-mix(in srgb, var(--c-accent) 8%,
 * var(--c-raised))`는 통과한다 — 토큰에서 계산한 색은 테마를 그대로 따라가므로 규칙이
 * 막으려는 대상이 아니다. 반대로 `color-mix(in srgb, white 8%, ...)`는 `white`가 남아 잡힌다.
 */
function stripRefs(value) {
    return value.replace(/\bvar\s*\([^)]*\)/gi, ' ').replace(/\burl\s*\([^)]*\)/gi, ' ')
}

/** 파일 하나에서 색 리터럴을 찾는다. */
function colorOffenders(path, text) {
    const found = []
    for (const pattern of COLOR_LITERALS) {
        for (const m of text.matchAll(pattern)) {
            found.push(`${path}:${lineOf(text, m.index)} — ${m[0]}`)
        }
    }
    for (const m of text.matchAll(COLOR_DECL)) {
        for (const named of stripRefs(m[2]).matchAll(NAMED_RE)) {
            found.push(`${path}:${lineOf(text, m.index)} — ${m[1]}: ${named[0]}`)
        }
    }
    for (const m of text.matchAll(TW_COLOR)) {
        found.push(`${path}:${lineOf(text, m.index)} — ${m[0]}`)
    }
    for (const m of text.matchAll(TW_ARBITRARY)) {
        for (const named of stripRefs(m[2].replace(/_/g, ' ')).matchAll(NAMED_RE)) {
            found.push(`${path}:${lineOf(text, m.index)} — ${m[0]} (${named[0]})`)
        }
    }
    return found
}

/**
 * 인라인 style에 적으면 안 되는 속성. 값이 무엇이든 속성 자체를 막는다 —
 * 폭·정렬 같은 배치 값만 허용한다는 것이 규칙이라, 여기 색이 있으면 테마를 바꿔도
 * 그 부분만 남는다.
 */
const INLINE_BANNED = new RegExp(`(?<![-\\w])(${PROP_ALT}|font-size|fontSize|font)\\s*:`, 'i')

/**
 * 인라인 style 속성을 찾는다. 따옴표는 쌍·홑 둘 다 본다 —
 * 쌍따옴표만 보던 탓에 `style='color: white'`가 이 검사와 색 검사를 함께 빠져나갔다.
 */
const INLINE_STYLE = /:?style\s*=\s*(["'])([\s\S]*?)\1/g

/** 파일 하나에서 색·글자 크기를 적은 인라인 style을 찾는다. */
function inlineStyleOffenders(path, text) {
    const found = []
    for (const m of text.matchAll(INLINE_STYLE)) {
        if (INLINE_BANNED.test(m[2])) {
            found.push(`${path}:${lineOf(text, m.index)} — ${m[0].slice(0, 60)}`)
        }
    }
    return found
}

// ── 토큰 파일 읽기 ────────────────────────────────────────────────

/**
 * CSS 주석을 공백으로 바꾼다. 길이를 유지해 줄 번호가 어긋나지 않는다.
 *
 * style.css의 머리 주석에 `html[data-theme="dark"]`라는 글자가 그대로 적혀 있어서,
 * 주석을 지우지 않고 선택자를 찾으면 주석을 먼저 만난다. 그러면 다크 블록 대신
 * 바로 뒤의 `:root` 블록을 읽어 라이트를 라이트와 비교하게 되고, 검사는
 * 아무것도 확인하지 않은 채 통과한다.
 */
function stripComments(css) {
    return css.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, ' '))
}

/**
 * 선택자에 해당하는 규칙 블록의 본문을 전부 모은다.
 * 중괄호 짝을 세어 끝을 찾는다 — 첫 `}`에서 멈추면 중첩된 규칙이 들어오는 순간 잘린다.
 */
function ruleBlocks(css, selector) {
    const blocks = []
    for (const m of css.matchAll(selector)) {
        const open = css.indexOf('{', m.index + m[0].length)
        if (open === -1) continue
        let depth = 0
        for (let i = open; i < css.length; i++) {
            if (css[i] === '{') depth += 1
            else if (css[i] === '}' && (depth -= 1) === 0) {
                blocks.push(css.slice(open + 1, i))
                break
            }
        }
    }
    return blocks
}

/** 바로 뒤에 `{`가 오는 `:root`만. `:root[data-theme='dark']`를 라이트로 읽으면 안 된다. */
const LIGHT_SELECTOR = /(?<![-\w]):root\s*(?=\{)/g

/**
 * 다크 토큰 블록. 따옴표 표기(`"dark"` · `'dark'` · `dark`)와 앞의 `html` 유무를 모두 받는다.
 * 선택자를 조금 고쳤다고 검사가 조용히 풀리면 안 된다.
 * 뒤에 `{`를 요구하므로 `html[data-theme="dark"] .field {` 같은 컴포넌트 규칙은 걸리지 않는다.
 */
const DARK_SELECTOR = /(?:html|:root)?\[data-theme\s*=\s*["']?dark["']?\s*]\s*(?=\{)/g

/**
 * 바로 뒤에 `{`가 오는 `html`만. `html[data-theme="dark"]`는 다크 토큰 블록이지
 * 문서 전체에 걸리는 규칙이 아니라, 여기로 읽으면 스크롤바 검사가 엉뚱한 블록을 본다.
 */
const HTML_SELECTOR = /(?<![-\w])html\s*(?=\{)/g

/** 블록들에서 색 토큰 이름을 모은다. 색 토큰은 `--c-` 접두사를 쓴다. */
function colorTokens(blocks) {
    return [...new Set(blocks.flatMap((b) => [...b.matchAll(/(--c-[\w-]+)\s*:/g)].map((m) => m[1])))]
}

// ── 검사 ────────────────────────────────────────────────

describe('글자 크기', () => {
    it('글자 크기 선언에 16px보다 작은 값이 없다', () => {
        const offenders = FILES.flatMap(({path, text}) => sizeOffenders(path, text))
        expect(
            offenders,
            `본문보다 작은 글자는 쓰지 않는다. 작아 보여야 하는 자리는 크기 대신 색(--c-ink-3)과 ` +
            `자간으로 누른다. 밀도가 필요하면 행 높이와 여백을 줄인다.\n${offenders.join('\n')}`,
        ).toEqual([])
    })
})

describe('색', () => {
    it('색 리터럴은 style.css에만 있다', () => {
        const offenders = FILES
            .filter(({path}) => path !== TOKEN_FILE)
            .flatMap(({path, text}) => colorOffenders(path, text))
        expect(
            offenders,
            `색은 style.css의 토큰에만 둔다. 컴포넌트에 적힌 색은 한쪽 테마에서만 읽힌다. ` +
            `#fff든 white든 bg-red-600이든 결과는 같다. ` +
            `필요한 색이 토큰에 없으면 토큰을 추가하라.\n${offenders.join('\n')}`,
        ).toEqual([])
    })

    it('인라인 style 속성으로 색·글자 크기를 설정하지 않는다', () => {
        const offenders = FILES.flatMap(({path, text}) => inlineStyleOffenders(path, text))
        expect(
            offenders,
            `인라인 style에 색이나 글자 크기를 적으면 테마 전환에서 그 부분만 남는다. ` +
            `클래스와 토큰으로 옮겨라.\n${offenders.join('\n')}`,
        ).toEqual([])
    })
})

describe('토큰 파일', () => {
    const tokens = FILES.find((f) => f.path === TOKEN_FILE)
    const css = tokens ? stripComments(tokens.text) : ''

    it('style.css를 찾았다', () => {
        expect(tokens, 'style.css를 찾지 못했다. 파일을 옮겼다면 TOKEN_FILE을 고쳐라.').toBeDefined()
    })

    it('style.css는 prefers-color-scheme 미디어 쿼리를 쓰지 않는다', () => {
        expect(
            /@media[^{]*prefers-color-scheme/.test(css),
            '테마 상태는 data-theme 한 곳에만 있어야 한다. 미디어 쿼리를 쓰면 토글과 시스템 설정이 어긋난다.',
        ).toBe(false)
    })

    it('다크 테마가 라이트에서 정의한 색 토큰을 모두 다시 정의한다', () => {
        const lightBlocks = ruleBlocks(css, LIGHT_SELECTOR)
        const darkBlocks = ruleBlocks(css, DARK_SELECTOR)

        // 찾지 못한 검사는 통과가 아니라 실패다. 선택자를 고쳤다면 이 검사도 함께 고쳐야 한다.
        expect(
            lightBlocks.length,
            'style.css에서 `:root` 블록을 찾지 못했다. 검사할 대상을 못 찾은 것이므로 실패로 둔다.',
        ).toBeGreaterThan(0)
        expect(
            darkBlocks.length,
            'style.css에서 다크 토큰 블록을 찾지 못했다. 선택자를 바꿨다면 DARK_SELECTOR도 함께 고쳐라.',
        ).toBeGreaterThan(0)

        const light = colorTokens(lightBlocks)
        expect(light.length, '`:root`에서 색 토큰을 하나도 읽지 못했다.').toBeGreaterThan(0)

        // 크기 토큰(--t-*)은 비교하지 않는다. 테마에 따라 달라지는 값이 아니라 양쪽이 같다.
        const dark = colorTokens(darkBlocks)
        const missing = light.filter((name) => !dark.includes(name))
        expect(
            missing,
            `다크에서 값이 빠진 토큰은 라이트 값을 그대로 쓴다. 어두운 바탕에 밝은 바탕용 색이 남는다.\n${missing.join(', ')}`,
        ).toEqual([])
    })
})

/**
 * 스크롤바 — **화면이 흔들리지 않는다.**
 *
 * 내용이 한 줄 늘어 스크롤바가 나타나는 순간 본문 폭이 그만큼 줄어 화면 전체가
 * 좌우로 움직인다. 목록에서 한 건을 지웠을 때도 반대로 움직인다. 교사가 30행짜리
 * 격자를 훑으며 누르는 프로그램이라, 누르려던 자리가 그 사이에 옮겨 간다.
 *
 * 규칙이 셋이고 셋이 함께 있어야 뜻이 있다.
 *   1. `html`의 `scrollbar-gutter: stable` — 스크롤바 자리를 미리 잡아 둔다.
 *   2. `::-webkit-scrollbar` 모양 — 창이 WebView2(Chromium)라 실제로 그려진다.
 *   3. `scrollbar-width` · `scrollbar-color`를 쓰지 않는다 — 최신 Chromium은 그 둘 중
 *      하나라도 있으면 표준 스크롤바로 넘어가면서 2번을 통째로 무시한다. 색만 바꾸려고
 *      한 줄 더한 것이 모양을 전부 되돌리는데, 브라우저는 아무 경고도 하지 않는다.
 */
describe('스크롤바', () => {
    const tokens = FILES.find((f) => f.path === TOKEN_FILE)
    const css = tokens ? stripComments(tokens.text) : ''

    it('html이 스크롤바 자리를 늘 비워 둔다', () => {
        const blocks = ruleBlocks(css, HTML_SELECTOR)
        expect(
            blocks.length,
            'style.css에서 `html` 규칙을 찾지 못했다. 선택자를 바꿨다면 HTML_SELECTOR도 함께 고쳐라.',
        ).toBeGreaterThan(0)
        expect(
            blocks.some((b) => /scrollbar-gutter\s*:\s*stable/.test(b)),
            '`html`에 scrollbar-gutter: stable이 없다. 스크롤바가 나타나고 사라질 때마다 ' +
            '본문 폭이 바뀌어 화면 전체가 좌우로 움직인다.',
        ).toBe(true)
    })

    it('::-webkit-scrollbar 모양을 그린다', () => {
        expect(
            /::-webkit-scrollbar(-thumb)?\s*\{/.test(css),
            '스크롤바 모양이 없다. 창이 WebView2라 이 선택자가 실제로 그려지는 자리다.',
        ).toBe(true)
    })

    it('scrollbar-width · scrollbar-color를 쓰지 않는다', () => {
        // 주석에는 이 두 이름이 "쓰지 않는다"는 근거로 적혀 있다. 주석을 지운 본문만 본다.
        const offenders = ['scrollbar-width', 'scrollbar-color']
            .filter((prop) => new RegExp(`${prop}\\s*:`).test(css))
        expect(
            offenders,
            '이 둘 중 하나라도 있으면 최신 Chromium이 표준 스크롤바로 넘어가면서 ' +
            `::-webkit-scrollbar 모양을 통째로 무시한다. 한쪽만 고른다.\n${offenders.join(', ')}`,
        ).toEqual([])
    })
})

/**
 * CLAUDE.md가 ❌로 못박은 순수 한국어 동사. 일상에서 거의 쓰이지 않아 읽는 사람이
 * 한 번 멈춘다 — 같은 뜻의 `한자어 + 하다`가 훨씬 자연스럽다.
 *
 * **지금은 `style.css`만 본다.** 이 파일이 토큰의 집이고 규칙이 실제로 새었던 자리이며,
 * 검사기가 이미 이 파일을 읽고 있기 때문이다. 나머지 화면 · 시험 파일에도 같은 규칙이
 * 적용되지만 아직 남은 것이 있어, 그쪽을 정리할 때 이 검사의 범위를 `FILES` 전체로
 * 넓히면 된다 — 범위만 바꾸면 되도록 검사와 목록을 분리해 두었다.
 */
/**
 * 금칙어 목록을 **`terms.md`에서 읽는다.**
 *
 * 목록을 여기에 적어 두면 `terms.md` · 이 파일 · `src-tauri/src/tests/terms_tests.rs`
 * 세 곳이 곧 갈라진다. 셋이 다른 말을 하면 어느 것이 규칙인지 아무도 모른다.
 * 고칠 곳은 `terms.md`의 표 하나다.
 *
 * 표의 모양은 `| \`찾을 문자열\` | 대신 쓸 말 |`이고 **백틱 안의 앞뒤 빈칸도 문자열의
 * 일부다** — `정하다`류는 앞 빈칸이 있어야 `판정하지` · `결정한다`가 걸리지 않는다.
 */
function bannedWords() {
    const path = join(SRC, '..', '..', 'terms.md')
    const text = readFileSync(path, 'utf-8')
    const section = text.split('## 4. 검사기가 읽는 목록')[1]
    expect(section, 'terms.md에 `## 4. 검사기가 읽는 목록` 절이 없다.').toBeDefined()

    const rules = []
    for (const line of section.split(NL)) {
        const row = line.trim()
        if (!row.startsWith('| `')) continue
        const cells = row.replace(/^\|/, '').replace(/\|$/, '').split('|')
        // 백틱 안쪽이 곧 찾을 문자열이다. trim으로 빈칸을 없애면 안 된다.
        const word = cells[0].trim().replace(/^`/, '').replace(/`$/, '')
        rules.push([word, (cells[1] ?? '').trim()])
    }
    expect(rules.length, 'terms.md의 금칙어 표를 읽지 못했다.').toBeGreaterThan(10)
    return rules
}

/**
 * 검사에서 빼는 파일. **금칙어 목록과 그 목록을 확인하는 시험은 그 낱말을 쓸 수밖에 없다.**
 * 그 밖의 `*.test.js`도 뺀다 — "이 낱말이 없어야 한다"고 단언하려면 낱말을 적어야 한다.
 */
const TERM_SKIP = /(^|\/)style\.test\.js$/

describe('문구', () => {
    const rules = bannedWords()

    it('terms.md가 금칙어 표를 들고 있다', () => {
        expect(rules.map(([w]) => w)).toContain('맡은 것')
    })

    it('화면과 소스에 교사가 쓰지 않는 말이 없다', () => {
        // **주석도 검사한다.** 주석의 낱말이 다음 화면 문구의 낱말이 되기 때문이다.
        const offenders = []
        for (const file of FILES) {
            if (TERM_SKIP.test(file.path)) continue
            file.text.split(NL).forEach((line, i) => {
                for (const [word, better] of rules) {
                    if (line.includes(word)) {
                        offenders.push(`${file.path}:${i + 1} [${word.trim()}] → ${better}`)
                    }
                }
            })
        }
        expect(
            offenders,
            ['화면에 쓰지 않는 말이 남아 있다. 낱말만 바꾸고 뜻은 그대로 둘 것.', ...offenders].join(NL),
        ).toEqual([])
    })
})

describe('검사기 자체', () => {
    // 저장소가 깨끗하면 위 검사는 전부 초록이다. 검사기가 고장 나도 초록이다.
    // 그 둘을 구분하려고 표본을 넣어 본다.
    const size = (text) => sizeOffenders('표본', text)
    const color = (text) => colorOffenders('표본', text)

    it('글자 크기 우회로를 잡는다', () => {
        expect(size('font-size: 14px')).toHaveLength(1)
        expect(size('font-size: 14PX'), '대문자 단위').toHaveLength(1)
        expect(size('font: 12px Pretendard'), '단축 속성').toHaveLength(1)
        expect(size('font-size: 11pt'), 'px 아닌 단위').toHaveLength(1)
        expect(size('font-size: clamp(12px, 2vw, 20px)'), 'clamp의 아래끝').not.toHaveLength(0)
        expect(size('font-size: 2vw'), '창 크기에 따라 변하는 값').toHaveLength(1)
        expect(size('font-size: 90%')).toHaveLength(1)
        expect(size('class="text-[0.8em]"'), 'Tailwind 임의 값').toHaveLength(1)
        expect(size('class="text-sm"')).toHaveLength(1)
        expect(size(':style="{ fontSize: \'13px\' }"'), 'camelCase').toHaveLength(1)
        expect(size('--t-tiny: 0.75rem'), '작은 크기 토큰').toHaveLength(1)
    })

    it('토큰으로 적은 크기는 통과시킨다', () => {
        expect(size('font-size: var(--t-base)')).toEqual([])
        expect(size('font-size: inherit')).toEqual([])
        expect(size('font-size: 1rem')).toEqual([])
        expect(size('font-size: 12pt'), '12pt = 16px라 최소를 만족한다').toEqual([])
        expect(size('font-family: Pretendard'), 'font-size가 아니다').toEqual([])
        expect(size('font-weight: 500')).toEqual([])
        expect(size('class="text-base"')).toEqual([])
    })

    it('이름으로 적은 색과 유틸리티 클래스를 잡는다', () => {
        expect(color('color: white'), '#fff와 같은 위반이다').toHaveLength(1)
        expect(color('border-color: black')).toHaveLength(1)
        expect(color('background: linear-gradient(white, gray)')).toHaveLength(2)
        expect(color('fill="red"'), 'SVG 속성').toHaveLength(1)
        expect(color("style='color: white'"), '홑따옴표').toHaveLength(1)
        expect(color('background: color-mix(in srgb, white 8%, var(--c-raised))')).toHaveLength(1)
        expect(color('color: oklch(70% 0.1 250)')).toHaveLength(1)
        expect(color('color: lab(50% 40 59)')).toHaveLength(1)
        expect(color('color: color(display-p3 1 0 0)')).toHaveLength(1)
        expect(color('class="bg-red-600"'), 'Tailwind가 실제로 설치돼 있다').toHaveLength(1)
        expect(color('class="hover:text-white"')).toHaveLength(1)
        expect(color('class="bg-[white]"')).toHaveLength(1)
        expect(color('color: #fff')).toHaveLength(1)
    })

    it('테마를 따라가는 값은 통과시킨다', () => {
        expect(color('stroke="currentColor"')).toEqual([])
        expect(color('background: transparent')).toEqual([])
        expect(color('fill="none"')).toEqual([])
        expect(color('color: var(--c-ink)')).toEqual([])
        expect(
            color('background: color-mix(in srgb, var(--c-accent) 8%, var(--c-raised))'),
            '토큰에서 계산한 색은 테마를 그대로 따라간다',
        ).toEqual([])
        expect(color('white-space: nowrap'), '속성 이름이지 색이 아니다').toEqual([])
        expect(color('background-image: url(red-icon.svg)'), '파일 이름이지 색이 아니다').toEqual([])
        expect(color('class="border-b"'), 'Tailwind 색 클래스가 아니다').toEqual([])
    })

    it('인라인 style은 따옴표 종류를 가리지 않는다', () => {
        expect(inlineStyleOffenders('표본', 'style="color: red"')).toHaveLength(1)
        expect(inlineStyleOffenders('표본', "style='color: red'")).toHaveLength(1)
        expect(inlineStyleOffenders('표본', 'style="background-color: red"')).toHaveLength(1)
        expect(inlineStyleOffenders('표본', ':style="{ fontSize: s }"')).toHaveLength(1)
        expect(inlineStyleOffenders('표본', 'style="width: 40%"'), '배치 값은 허용한다').toEqual([])
    })

    it('토큰 블록을 잘못 찾으면 빈 배열을 돌려준다', () => {
        // 주석 안의 선택자를 규칙으로 읽으면 엉뚱한 블록을 비교하게 된다.
        const sample = stripComments('/* html[data-theme="dark"] 로 켠다 */\n:root { --c-ink: #000; }')
        expect(ruleBlocks(sample, DARK_SELECTOR), '주석은 규칙이 아니다').toEqual([])
        expect(ruleBlocks(sample, LIGHT_SELECTOR)).toHaveLength(1)
        expect(ruleBlocks(":root[data-theme='dark'] { --c-ink: #fff; }", DARK_SELECTOR)).toHaveLength(1)
    })

    it('html 규칙만 html로 읽는다 — 다크 선택자를 여기로 읽으면 스크롤바 검사가 헛돈다', () => {
        expect(ruleBlocks('html { scrollbar-gutter: stable; }', HTML_SELECTOR)).toHaveLength(1)
        expect(ruleBlocks('html[data-theme="dark"] { --c-ink: #fff; }', HTML_SELECTOR)).toEqual([])
    })
})
