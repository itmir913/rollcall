import {defineStore} from 'pinia'
import {ref} from 'vue'
import {invoke} from '@tauri-apps/api/core'

export const useCheckStore = defineStore('check', () => {
    const items = ref([])
    const pending = ref([])
    const summary = ref([])
    const error = ref('')

    async function fetchItems(activeOnly = true) {
        error.value = ''
        try {
            items.value = await invoke('get_check_items', {activeOnly})
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function createItem(payload) {
        const id = await invoke('create_check_item', payload)
        await fetchItems(false)
        return id
    }

    async function updateItem(payload) {
        await invoke('update_check_item', payload)
        await fetchItems(false)
    }

    async function deactivateItem(id) {
        await invoke('deactivate_check_item', {id})
        await fetchItems(false)
    }

    async function setCheck(studentId, date, itemId, done) {
        await invoke('set_check', {studentId, date, itemId, done})
    }

    async function setDue(studentId, date, itemId, due) {
        await invoke('set_check_due', {studentId, date, itemId, due})
    }

    async function setGroupCheck(groupId, itemId, done) {
        return await invoke('set_group_check', {groupId, itemId, done})
    }

    async function fetchPending(yearId, grade, classNo, today) {
        error.value = ''
        try {
            pending.value = await invoke('get_pending', {yearId, grade, classNo, today})
        } catch (e) {
            error.value = String(e)
            throw e
        }
    }

    async function fetchSummary(yearId, grade, classNo) {
        summary.value = await invoke('get_pending_summary', {yearId, grade, classNo})
    }

    async function exportPendingCsv(yearId, grade, classNo, today, dest) {
        return await invoke('export_pending_csv', {yearId, grade, classNo, today, dest})
    }

    async function exportBackupCsv(yearId, grade, classNo, dest) {
        return await invoke('export_backup_csv', {yearId, grade, classNo, dest})
    }

    return {
        items, pending, summary, error,
        fetchItems, createItem, updateItem, deactivateItem,
        setCheck, setDue, setGroupCheck,
        fetchPending, fetchSummary, exportPendingCsv, exportBackupCsv,
    }
})
