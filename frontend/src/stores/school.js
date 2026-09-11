import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'

/**
 * 학교 설정 — 최대 교시 · 제출 기한 · 휴업일 · 태그 · 한도 규정, 그리고 맡은 것.
 *
 * 전부 **학교에 매달린 값**이다. 앱 전역 설정이 아니다. 순회 교사가 학교를 둘 이상
 * 등록하는 날이 와도 값을 옮기지 않아도 되게 하려는 것이다.
 *
 * 태그와 규정은 **UPDATE로 고치지 않는다.** valid_to를 채워 마감하고 새 행을 넣는다 —
 * 과거 기록이 가리키던 이름이 소급 변경되면 작년 통계가 조용히 달라진다.
 *
 * **맡은 것(`teaching_class`)을 고치고 마감하는 것도 여기 있다.** 제자리는 app 스토어이지만
 * 지금은 그 파일을 건드리지 않기로 했다. 설정 화면이 학교 단위로 맡은 것을 관리하므로
 * 임시로 같은 곳에 둔다 — 옮길 때 화면은 고치지 않아도 되도록 이름은 그대로 쓴다.
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

        /**
         * 학교를 하나 더 만든다. **순회 교사가 둘 이상을 맡는다.**
         *
         * 최대 교시와 제출 기한은 학교마다 다르므로 기본값을 넣어 두고 설정에서 고친다.
         * 새 학교는 아직 맡은 것이 없어 `app.schoolId`가 가리키지 않는다 — 그 학교에
         * 담임 · 교과를 만들 때 학교를 명시해 넘기는 이유다(`createClass`).
         */
        async createSchool(patch) {
            const app = useAppStore()
            this.error = ''
            try {
                const id = await invoke('create_school', {
                    school: {name: '', maxSlot: 7, dueDays: 7, dueSkipOffdays: true, ...patch},
                })
                await app.refreshSchool()
                return id
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 맡은 것 하나를 만든다. **학교를 명시해 받는다.**
         *
         * 지금 학교에 넣는 것은 app 스토어가 맡는다 — 이미 맡고 있는 것이면 만들지 않고
         * 고르는 판단이 거기 있고, 두 곳에 두면 한쪽만 고쳐진다. 다른 학교에 넣을 때만
         * 직접 만든다. `app.createClass`가 학교를 받게 되면 이 갈래는 사라진다.
         */
        /**
         * 맡은 것을 더한다. **통째로 app 스토어에 맡긴다.**
         *
         * 전에는 다른 학교일 때만 여기서 직접 만들었는데, 그 갈래에는 "이미 맡고 있는
         * 것이면 만들지 않는다"는 확인이 없어 [추가]를 두 번 누르면 같은 학급이 두 줄
         * 생겼다. 만드는 길이 둘이면 그중 하나는 반드시 규칙을 빠뜨린다.
         */
        async createClass({schoolId, role, name, grade = null, classNo = null, select = true}) {
            return useAppStore().createClass({schoolId, role, name, grade, classNo, select})
        },

        /**
         * 맡은 것의 이름을 고친다. **이것은 UPDATE가 맞다** — 태그 · 규정과 달리 학급 이름은
         * 과거 기록이 가리키는 값이 아니라 그 행 하나를 부르는 이름이고, 오타를 고치는
         * 일이 더 잦다. 학년 · 반은 담임 학급만 가진다.
         */
        async renameClass(classId, {name, grade = null, classNo = null}) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('update_teaching_class', {classId, name, grade, classNo})
                await app.fetchClasses()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 맡은 것을 마감한다. **삭제가 아니다** — 지난 출결이 이 학급을 가리키므로
         * 지우면 기록이 함께 사라진다. 목록에서만 내려간다.
         *
         * 마감한 것이 보고 있던 학급이면 남은 것 하나로 옮긴다. 고른 학급이 없는 채로
         * 두면 모든 화면이 빈 채로 돌고, 교사는 고장인지 아닌지 알 수 없다.
         */
        async retireClass(classId) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('retire_teaching_class', {classId, validTo: app.today})
                await app.fetchClasses()
                if (app.classId === classId) {
                    const next =
                        app.classes.find((c) => c.role === app.mode) ?? app.classes[0] ?? null
                    if (next) await app.selectClass(next.id)
                    else app.classId = null
                }
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
