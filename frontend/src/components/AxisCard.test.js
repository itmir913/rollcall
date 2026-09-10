/**
 * 축 카드 — 이 앱에서 가장 자주 눌리는 곳이다.
 *
 * 여기서 지키는 것은 둘이다.
 *   · **카드가 움직이지 않는다** — 구분을 바꿔도 버튼 수가 변하지 않고 활성 여부만 바뀐다.
 *   · **구분이 기간을 정한다** — 결석은 고를 것이 없고, 지각에 조회가, 조퇴에 종례가 없다.
 */
import {describe, expect, it} from 'vitest'
import {mount} from '@vue/test-utils'
import AxisCard from './AxisCard.vue'

const TYPES = [
    {id: 1, label: '지각', slotPrompt: 'end'},
    {id: 2, label: '조퇴', slotPrompt: 'start'},
    {id: 3, label: '결석', slotPrompt: 'none'},
    {id: 4, label: '결과', slotPrompt: 'multi'},
]
const REASONS = [
    {id: 10, label: '질병'},
    {id: 11, label: '미인정'},
]

function build(model = {reasonId: 10, typeId: 1, slots: []}) {
    return mount(AxisCard, {
        props: {modelValue: model, reasons: REASONS, types: TYPES, maxSlot: 7},
    })
}

/** 기간 줄의 버튼만 추린다. 구분·종류 줄은 앞의 두 줄이다. */
function slotButtons(wrapper) {
    return wrapper.findAll('.axis__line')[2].findAll('button')
}

function labelOf(button) {
    return button.text()
}

describe('축 카드', () => {
    it('구분을 바꿔도 버튼 수가 변하지 않는다', async () => {
        const wrapper = build()
        const before = slotButtons(wrapper).length

        await wrapper.setProps({modelValue: {reasonId: 10, typeId: 3, slots: []}})
        expect(slotButtons(wrapper).length).toBe(before)
    })

    it('결석은 기간 전체가 꺼진다 — 하루 종일이라 고를 것이 없다', () => {
        const wrapper = build({reasonId: 10, typeId: 3, slots: []})
        expect(slotButtons(wrapper).every((b) => b.attributes('disabled') !== undefined)).toBe(true)
    })

    it('지각에는 조회가 없다. 조회에 이미 왔으면 지각이 아니다', () => {
        const wrapper = build({reasonId: 10, typeId: 1, slots: []})
        const homeroom = slotButtons(wrapper).find((b) => labelOf(b) === '조회')
        expect(homeroom.attributes('disabled')).toBeDefined()
    })

    it('조퇴에는 종례가 없다. 종례까지 있었으면 조퇴가 아니다', () => {
        const wrapper = build({reasonId: 10, typeId: 2, slots: []})
        const closing = slotButtons(wrapper).find((b) => labelOf(b) === '종례')
        expect(closing.attributes('disabled')).toBeDefined()
    })

    it('결과는 조회 · 종례 · ?를 고를 수 없다', () => {
        const wrapper = build({reasonId: 10, typeId: 4, slots: []})
        const off = slotButtons(wrapper)
            .filter((b) => ['조회', '종례', '?'].includes(labelOf(b)))
            .every((b) => b.attributes('disabled') !== undefined)
        expect(off).toBe(true)
    })

    it('구분을 바꿔 못 쓰는 기간이 되면 미정으로 되돌린다', async () => {
        // 조퇴로 종례까지 고른 상태에서 지각으로 바꾸면 그 기간은 쓸 수 없다.
        const wrapper = build({reasonId: 10, typeId: 2, slots: ['조회']})
        await slotButtons(wrapper) // 렌더를 기다린다
        const typeButtons = wrapper.findAll('.axis__line')[0].findAll('button')
        await typeButtons.find((b) => b.text() === '지각').trigger('click')

        const emitted = wrapper.emitted('update:modelValue').at(-1)[0]
        expect(emitted.slots).toEqual([])
    })

    it('결과만 여러 교시가 켜진다', async () => {
        const wrapper = build({reasonId: 10, typeId: 4, slots: ['1']})
        await slotButtons(wrapper).find((b) => labelOf(b) === '3').trigger('click')
        expect(wrapper.emitted('update:modelValue').at(-1)[0].slots).toEqual(['1', '3'])
    })

    it('하나만 고르는 구분은 앞의 선택을 갈아 끼운다', async () => {
        const wrapper = build({reasonId: 10, typeId: 1, slots: ['1']})
        await slotButtons(wrapper).find((b) => labelOf(b) === '3').trigger('click')
        expect(wrapper.emitted('update:modelValue').at(-1)[0].slots).toEqual(['3'])
    })

    it('지금 고른 조합을 한 줄로 말해 준다', () => {
        const wrapper = build({reasonId: 10, typeId: 1, slots: ['2']})
        expect(wrapper.find('.stamp').text()).toContain('질병 지각 · 조회부터 2교시까지')
    })

    it('축이 비어 있으면 미정이라고 말한다 — 미완성 기록은 정상 상태다', () => {
        const wrapper = build({reasonId: null, typeId: null, slots: []})
        expect(wrapper.find('.stamp').text()).toContain('미정 미정 · 기간 미정')
    })
})
