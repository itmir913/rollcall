import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'

/**
 * 학교 설정 — 최대 교시 · 제출 기한 · 휴업일 · 태그 · 한도 규정.
 *
 * 전부 **학교에 매달린 값**이다. 앱 전역 설정이 아니다. 순회 교사가 학교를 둘 이상
 * 등록하는 날이 와도 값을 옮기지 않아도 되게 하려는 것이다.
 *
 * 태그와 규정은 **UPDATE로 고치지 않는다.** valid_to를 채워 마감하고 새 행을 넣는다 —
 * 과거 기록이 가리키던 이름이 소급 변경되면 작년 통계가 조용히 달라진다.
 */
export const useSchoolStore = defineStore('school', {
    state: () => ({
        school: null,
        offDays: [],
        tags: [],
        rules: [],
        error: '',
    }),

    actions: {
        async fetchAll() {
            const app = useAppStore()
            if (!app.schoolId) return
            this.error = ''
            try {
                const [school, offDays, tags, rules] = await Promise.all([
                    invoke('get_school', {schoolId: app.schoolId}),
                    invoke('get_off_days', {schoolId: app.schoolId, from: null, to: null}),
                    invoke('get_tags', {schoolId: app.schoolId}),
                    invoke('get_quota_rules', {schoolId: app.schoolId}),
                ])
                this.school = school
                this.offDays = offDays
                this.tags = tags
                this.rules = rules
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async saveSchool(patch) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('update_school', {school: {...this.school, ...patch}})
                await this.fetchAll()
                await app.refreshSchool()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async addOffDay(date, label) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('add_off_day', {schoolId: app.schoolId, day: {date, label}})
                await this.fetchAll()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async removeOffDay(offDayId) {
            this.error = ''
            try {
                await invoke('remove_off_day', {offDayId})
                await this.fetchAll()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async createTag(name) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('create_tag', {schoolId: app.schoolId, name})
                await this.fetchAll()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async renameTag(tagId, name) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('rename_tag', {tagId, name, today: app.today})
                await this.fetchAll()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async retireTag(tagId) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('retire_tag', {tagId, today: app.today})
                await this.fetchAll()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async createRule(rule) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('create_quota_rule', {schoolId: app.schoolId, rule})
                await this.fetchAll()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async reviseRule(rule) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('revise_quota_rule', {rule, today: app.today})
                await this.fetchAll()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async retireRule(ruleId) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('retire_quota_rule', {ruleId, today: app.today})
                await this.fetchAll()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})
