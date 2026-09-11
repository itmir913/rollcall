/**
 * 대화상자.
 *
 * 가장 중요한 것은 **집중 등재가 ESC와 바깥 클릭으로 닫히지 않는다**는 것이다.
 * 나이스 저장이 실패하는 날이 있어, 저장인지 취소인지 반드시 고르게 해야 한다.
 */
import {describe, expect, it} from 'vitest'
import {mount} from '@vue/test-utils'
import UiModal from './UiModal.vue'
import UiToggle from './UiToggle.vue'
import UiNotice from './UiNotice.vue'
import UiButton from './UiButton.vue'

describe('UiModal', () => {
    it('닫혀 있으면 아무것도 그리지 않는다', () => {
        mount(UiModal, {props: {open: false, title: '지웁니다'}, attachTo: document.body})
        expect(document.querySelector('.modal')).toBeNull()
    })

    it('바깥을 누르면 닫는다', async () => {
        const wrapper = mount(UiModal, {props: {open: true, title: '지웁니다'}, attachTo: document.body})
        await document.querySelector('.modal').click()
        expect(wrapper.emitted('close')).toBeTruthy()
        wrapper.unmount()
    })

    it('집중 등재는 바깥 클릭으로 닫히지 않는다', async () => {
        const wrapper = mount(UiModal, {
            props: {open: true, title: '한 명씩 등재', dismissible: false},
            attachTo: document.body,
        })
        await document.querySelector('.modal').click()
        expect(wrapper.emitted('close')).toBeFalsy()
        wrapper.unmount()
    })

    it('집중 등재는 ESC로도 닫히지 않는다', async () => {
        const wrapper = mount(UiModal, {
            props: {open: true, title: '한 명씩 등재', dismissible: false},
            attachTo: document.body,
        })
        document.dispatchEvent(new KeyboardEvent('keydown', {key: 'Escape'}))
        expect(wrapper.emitted('close')).toBeFalsy()
        wrapper.unmount()
    })

    it('보통 대화상자는 ESC로 닫힌다', async () => {
        const wrapper = mount(UiModal, {props: {open: true, title: '지웁니다'}, attachTo: document.body})
        document.dispatchEvent(new KeyboardEvent('keydown', {key: 'Escape'}))
        expect(wrapper.emitted('close')).toBeTruthy()
        wrapper.unmount()
    })
})

describe('UiToggle', () => {
    it('체크박스가 아니라 버튼이다 — 클릭 표적이 커야 한다', () => {
        const wrapper = mount(UiToggle, {
            props: {modelValue: false, onLabel: '서류 제출', offLabel: '서류 미제출'},
        })
        expect(wrapper.find('input[type="checkbox"]').exists()).toBe(false)
        expect(wrapper.find('button.mark').exists()).toBe(true)
    })

    it('켜짐과 꺼짐이 글자로도 보인다 — 색만으로 말하지 않는다', async () => {
        const wrapper = mount(UiToggle, {
            props: {modelValue: false, onLabel: '서류 제출', offLabel: '서류 미제출'},
        })
        expect(wrapper.text()).toBe('서류 미제출')
        await wrapper.setProps({modelValue: true})
        expect(wrapper.text()).toBe('서류 제출')
        expect(wrapper.classes()).toContain('is-on')
    })

    it('누르면 뒤집는다', async () => {
        const wrapper = mount(UiToggle, {
            props: {modelValue: false, onLabel: '등재', offLabel: '미등재'},
        })
        await wrapper.trigger('click')
        expect(wrapper.emitted('update:modelValue').at(-1)).toEqual([true])
    })
})

describe('UiNotice', () => {
    it('빈 문자열이면 자리를 차지하지 않는다', () => {
        expect(mount(UiNotice, {props: {text: ''}}).find('.notice').exists()).toBe(false)
    })

    it('실패를 조용히 삼키지 않는다', () => {
        const wrapper = mount(UiNotice, {props: {text: '학생을 찾을 수 없습니다', kind: 'error'}})
        expect(wrapper.text()).toBe('학생을 찾을 수 없습니다')
        expect(wrapper.classes()).toContain('notice--error')
    })
})

describe('UiButton', () => {
    it('내려받기는 어디서나 같은 모양이다 — 강조색 채움 + 아래 화살표', () => {
        const wrapper = mount(UiButton, {props: {variant: 'download'}})
        expect(wrapper.classes()).toContain('btn--download')
        expect(wrapper.find('svg').exists()).toBe(true)
    })

    it('가져오기는 위 화살표다', () => {
        expect(mount(UiButton, {props: {variant: 'upload'}}).find('svg').exists()).toBe(true)
    })

    it('보통 버튼에는 아이콘이 붙지 않는다', () => {
        expect(mount(UiButton, {}).find('svg').exists()).toBe(false)
    })
})
