/**
 * 여러 날 찍기.
 *
 * 기간은 **화면이 아니라 입력의 한 축**이라 탭이 아니라 창이다.
 * 주말과 휴업일은 Rust가 빼 주지만, 남은 날 중 학교가 쉰 날은 교사가 지운다 —
 * 학사일정을 앱이 알 수 없다는 사실의 인정이다.
 */
import {describe, expect, it, vi} from 'vitest'
import {mount} from '@vue/test-utils'
import BulkStampModal from './BulkStampModal.vue'

const DAYS = [
    {date: '2026-09-07', label: '09.07.(월)', hasExisting: false},
    {date: '2026-09-08', label: '09.08.(화)', hasExisting: true},
    {date: '2026-09-09', label: '09.09.(수)', hasExisting: false},
]

function build(preview = vi.fn().mockResolvedValue(DAYS)) {
    const wrapper = mount(BulkStampModal, {
        props: {
            open: true,
            student: {studentId: 11, number: 5, name: '김하늘'},
            phrase: '질병 결석 · 하루 종일',
            preview,
        },
        attachTo: document.body,
    })
    return {wrapper, preview}
}

const dayButtons = () =>
    [...document.querySelectorAll('.bulk__days .pick')]

describe('여러 날 찍기', () => {
    it('기간을 고르기 전에는 날짜를 묻지 않는다', () => {
        const {wrapper} = build()
        expect(document.querySelector('.bulk')).toBeNull()
        wrapper.unmount()
    })

    it('기간을 비워 두면 묻지 않고 알린다', async () => {
        const {wrapper, preview} = build()
        // 창은 body로 옮겨 그려지므로 값은 컴포넌트에 직접 넣는다.
        await document.querySelector('.filters .btn').click()
        await wrapper.vm.$nextTick()

        expect(preview).not.toHaveBeenCalled()
        expect(document.querySelector('.notice').textContent).toContain('골라주세요')
        wrapper.unmount()
    })

    it('시작일이 마지막 날보다 뒤면 그렇게 말한다', async () => {
        const {wrapper, preview} = build()
        wrapper.vm.from = '2026-09-20'
        wrapper.vm.to = '2026-09-01'
        await wrapper.vm.$nextTick()

        await document.querySelector('.filters .btn').click()
        await wrapper.vm.$nextTick()

        expect(preview).not.toHaveBeenCalled()
        expect(document.querySelector('.notice').textContent).toContain('시작일이')
        wrapper.unmount()
    })

    it('기간을 고르면 셀 수 있는 날만 받아 온다 — 주말과 휴업일은 Rust가 뺀다', async () => {
        const {wrapper, preview} = build()
        wrapper.vm.from = '2026-09-07'
        wrapper.vm.to = '2026-09-09'
        await wrapper.vm.$nextTick()

        await document.querySelector('.filters .btn').click()
        await wrapper.vm.$nextTick()

        expect(preview).toHaveBeenCalledWith('2026-09-07', '2026-09-09')
        wrapper.unmount()
    })

    it('이미 그날 구간이 있는 학생임을 알린다 — 막지는 않는다', async () => {
        const {wrapper} = build()
        wrapper.vm.days = DAYS
        await wrapper.vm.$nextTick()

        expect(document.body.textContent).toContain('이미 있음')
        wrapper.unmount()
    })

    it('교사가 뺀 날은 찍지 않는다', async () => {
        const {wrapper} = build()
        wrapper.vm.days = DAYS
        await wrapper.vm.$nextTick()

        await dayButtons()[1].click()
        await wrapper.vm.$nextTick()

        const apply = [...document.querySelectorAll('.modal__foot .btn')].at(-1)
        await apply.click()
        await wrapper.vm.$nextTick()

        const payload = wrapper.emitted('apply').at(-1)[0]
        expect(payload.days).toEqual(['2026-09-07', '2026-09-09'])
        wrapper.unmount()
    })

    it('고른 날이 하나도 없으면 찍을 수 없다', async () => {
        const {wrapper} = build()
        wrapper.vm.days = DAYS
        await wrapper.vm.$nextTick()

        for (const button of dayButtons()) {
            button.click()
        }
        await wrapper.vm.$nextTick()

        const apply = [...document.querySelectorAll('.modal__foot .btn')].at(-1)
        expect(apply.hasAttribute('disabled')).toBe(true)
        wrapper.unmount()
    })
})
