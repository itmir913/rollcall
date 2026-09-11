/**
 * 내보내기 스토어.
 *
 * 이 스토어가 있는 이유는 하나다 — **눌러도 아무 일이 없는 단추를 두지 않기 위해서.**
 * 전에는 템플릿에서 서비스를 바로 불러 실패가 아무 데도 남지 않았다.
 */
import {beforeEach, describe, expect, it, vi} from 'vitest'
import {createPinia, setActivePinia} from 'pinia'
import {useDownloadStore} from './download'
import {exportCsv} from '../services/download'

vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))
vi.mock('../services/download', () => ({exportCsv: vi.fn()}))

beforeEach(() => {
    setActivePinia(createPinia())
    exportCsv.mockReset()
})

describe('csv', () => {
    it('저장한 경로를 알린다 — 대화상자에서 고른 곳을 곧 잊는다', async () => {
        exportCsv.mockResolvedValue('C:/temp/출결.csv')
        const store = useDownloadStore()
        await store.csv('spans', {}, '출결.csv')

        expect(store.done).toContain('C:/temp/출결.csv')
        expect(store.error).toBe('')
    })

    it('교사가 취소한 것은 실패가 아니다 — 아무 말도 하지 않는다', async () => {
        exportCsv.mockResolvedValue(null)
        const store = useDownloadStore()
        await store.csv('spans', {}, '출결.csv')

        expect(store.done).toBe('')
        expect(store.error).toBe('')
    })

    it('실패를 화면에 남기고 다시 던진다', async () => {
        exportCsv.mockRejectedValue('디스크에 쓸 수 없습니다')
        const store = useDownloadStore()

        await expect(store.csv('spans', {}, '출결.csv')).rejects.toBeTruthy()
        expect(store.error).toContain('디스크에 쓸 수 없습니다')
        expect(store.busy).toBe(false)
    })
})
