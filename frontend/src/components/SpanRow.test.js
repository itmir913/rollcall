/**
 * 출결 한 줄 — 네 화면이 같은 줄을 쓴다.
 *
 * 지키는 것:
 *   · 사유는 접혀 있다가 눌러야 열린다(칩을 늘어놓으면 번호·이름이 오른쪽으로 밀린다).
 *   · 태그는 붙어 있을 때만 배지로 보인다.
 *   · 겹침은 막지 않고 표시만 한다.
 */
import {describe, expect, it} from 'vitest'
import {mount} from '@vue/test-utils'
import SpanRow from './SpanRow.vue'

const SPAN = {
    id: 1,
    number: 5,
    name: '김하늘',
    date: '2026-09-10',
    dateLabel: '2026.09.10.(목)',
    reasonLabel: '질병',
    typeLabel: '결석',
    slotPrompt: 'none',
    spanText: '하루 종일',
    memo: '',
    tagId: null,
    tagName: null,
    complete: true,
    overlapping: false,
    docDone: false,
    neisDone: false,
}

function build(span = {}, props = {}) {
    return mount(SpanRow, {
        props: {
            span: {...SPAN, ...span},
            tags: [{id: 7, name: '체험학습'}],
            memos: ['감기', '장염'],
            expanded: false,
            ...props,
        },
    })
}

describe('출결 줄', () => {
    it('사유가 비면 무엇을 해야 하는지 적어 둔다', () => {
        expect(build().find('.row__reason button').text()).toContain('사유를 입력하세요')
    })

    it('사유를 누르면 열라고 알린다 — 그 줄만 커진다', async () => {
        const wrapper = build()
        await wrapper.find('.row__reason button').trigger('click')
        expect(wrapper.emitted('toggleExpand')).toHaveLength(1)
    })

    it('접혀 있으면 편집 영역이 없다', () => {
        expect(build().find('.edit').exists()).toBe(false)
    })

    it('열면 사유 칸과 후보 · 태그 칩이 함께 나온다', () => {
        const wrapper = build({memo: '감기'}, {expanded: true})
        expect(wrapper.find('.edit__area').element.value).toBe('감기')
        expect(wrapper.findAll('.chip').length).toBe(3) // 후보 둘 + 태그 하나
    })

    it('태그는 붙어 있을 때만 배지로 보인다', () => {
        expect(build().find('.tagmark').exists()).toBe(false)
        expect(build({tagId: 7, tagName: '체험학습'}).find('.tagmark').text()).toBe('체험학습')
    })

    it('구분과 기간을 누르면 수정 모달을 열라고 알린다', async () => {
        const wrapper = build()
        await wrapper.find('.row__what button').trigger('click')
        await wrapper.find('.row__when button').trigger('click')
        expect(wrapper.emitted('fix')).toHaveLength(2)
    })

    it('겹치는 구간은 막지 않고 표시만 한다', () => {
        const wrapper = build({overlapping: true})
        expect(wrapper.find('.row__clash').text()).toBe('겹침')
        expect(wrapper.find('.row').classes()).toContain('is-warn')
    })

    it('미완성 기록은 주황으로 남는다 — 채워야 할 것이 보여야 한다', () => {
        expect(build({complete: false}).find('.row').classes()).toContain('is-warn')
    })

    it('결석은 초록, 나머지는 붉은 띠다', () => {
        expect(build().find('.row').classes()).toContain('is-ok')
        expect(build({slotPrompt: 'end'}).find('.row').classes()).toContain('is-bad')
    })

    it('태그 칩을 누르면 그 태그를 붙이라고 알린다', async () => {
        const wrapper = build({}, {expanded: true})
        await wrapper.findAll('.chip--tag')[0].trigger('click')
        expect(wrapper.emitted('updateTag').at(-1)).toEqual([7])
    })

    it('이미 붙은 태그를 다시 누르면 뗀다', async () => {
        const wrapper = build({tagId: 7, tagName: '체험학습'}, {expanded: true})
        await wrapper.findAll('.chip--tag')[0].trigger('click')
        expect(wrapper.emitted('updateTag').at(-1)).toEqual([null])
    })
})
