import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {useAppStore} from './app'

/**
 * 학교 설정 — 최대 교시 · 제출 기한 · 휴업일 · 태그 · 한도 규정 · 강좌 묶음, 그리고 담당 학급 · 강좌.
 *
 * 전부 **학교에 종속된 값**이다. 앱 전역 설정이 아니다. 순회 교사는 학교를 둘 이상
 * 등록하고, 학년도가 바뀌면 그 학년도의 학교를 처음부터 다시 만든다.
 *
 * 태그와 규정은 **UPDATE로 고치지 않는다.** valid_to를 채워 마감하고 새 행을 넣는다 —
 * 과거 기록이 가리키던 이름이 소급 변경되면 작년 통계가 조용히 달라진다.
 *
 * **강좌 묶음(`class_tag`)은 출결 태그(`span_tag`)와 다른 표다.** 앞은 화면에서
 * `프로그래밍A · B · C`를 함께 보이게 하는 이름표이고, 뒤는 한도를 세는 대상이다.
 * 한 목록에 섞으면 `프로그래밍`이 한도 규정의 후보로 뜬다.
 *
 * **담당 학급 · 강좌(`teaching_class`)를 고치고 마감하는 것도 여기 있다.** 제자리는 app 스토어이지만
 * 설정 화면이 학교 단위로 담당 학급 · 강좌를 관리하므로 임시로 같은 곳에 둔다 — 옮길 때 화면은
 * 고치지 않아도 되도록 이름은 그대로 쓴다.
 */
export const useSchoolStore = defineStore('school', {
    state: () => ({
        school: null,
        offDays: [],
        tags: [],
        rules: [],
        /** 강좌 묶음 이름표. 교과 강좌를 화면에서 함께 보이게 한다. */
        classTags: [],
        error: '',
    }),

    actions: {
        /**
         * 지금 학교의 값을 전부 읽는다.
         *
         * **가리키는 학교가 없으면 전부 비운다.** 지난 학교의 값을 그대로 들고 있으면
         * 설정 화면과 첫 실행 화면이 **지난해 학교를 고친다** — 2027학년도 화면에서
         * 최대 교시를 바꾸면 2026학년도 A학교의 값이 바뀐다. 학년도를 옮기면 학교도
         * 다시 만드는 앱이라, 학교가 0개인 상태는 드물지 않다.
         *
         * **읽기 실패도 비운다.** 실패했는데 지난 학교의 설정이 그대로 보이면 교사는
         * 그것을 지금 학교의 값으로 읽는다. 둘은 `error`로 구분한다 — 비어 있고
         * `error`가 없으면 학교가 없는 것이고, 담겨 있으면 읽지 못한 것이다.
         */
        async fetchAll() {
            const app = useAppStore()
            this.error = ''
            if (!app.schoolId) {
                this.clear()
                return
            }
            try {
                const [school, offDays, tags, rules, classTags] = await Promise.all([
                    invoke('get_school', {schoolId: app.schoolId}),
                    invoke('get_off_days', {schoolId: app.schoolId, from: null, to: null}),
                    invoke('get_tags', {schoolId: app.schoolId}),
                    invoke('get_quota_rules', {schoolId: app.schoolId}),
                    invoke('get_class_tags', {schoolId: app.schoolId}),
                ])
                this.school = school
                this.offDays = offDays
                this.tags = tags
                this.rules = rules
                this.classTags = classTags
            } catch (e) {
                this.clear()
                this.error = String(e)
                throw e
            }
        },

        /**
         * 학교에 딸린 값을 전부 비운다. `school`이 null인 것이 **가리키는 학교가 없다**는
         * 뜻이고, 화면은 그것을 보고 "등록한 학교가 없습니다"를 적는다.
         */
        clear() {
            this.school = null
            this.offDays = []
            this.tags = []
            this.rules = []
            this.classTags = []
        },

        /**
         * 학교 설정을 고친다. **커맨드는 평평한 인자를 받는다** — 화면은 고친 칸만
         * 넘기고, 나머지는 지금 값으로 채운다. 화면마다 다섯 값을 다 들고 있게 하면
         * 한 칸을 고치려고 나머지 넷을 다시 적어야 하고, 그중 하나가 낡으면 소리 없이
         * 옛 값으로 되돌아간다.
         */
        async saveSchool(patch) {
            const app = useAppStore()
            this.error = ''
            const next = {...this.school, ...patch}
            // **가리키는 학교가 없으면 고치지 않는다.** 번호 없이 보내면 커맨드가
            // 거절하는데, 그 오류는 무엇이 잘못됐는지 말해 주지 않는다.
            if (next.id == null) {
                this.error = '고칠 학교가 없습니다. 학교를 먼저 등록해주세요.'
                throw new Error(this.error)
            }
            try {
                await invoke('update_school', {
                    schoolId: next.id,
                    name: next.name,
                    maxSlot: next.maxSlot,
                    dueDays: next.dueDays,
                    dueSkipOffdays: Boolean(next.dueSkipOffdays),
                })
                await this.fetchAll()
                await app.fetchSchools()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 학교를 하나 더 만든다. **순회 교사가 둘 이상을 맡는다.**
         *
         * 만든 학교는 고르지 않는다 — 고르는 것은 화면의 일이다(Welcome은 만들자마자
         * 옮겨 가고, 설정은 목록에 한 줄을 더할 뿐이다). 학교가 하나도 없던 상태였다면
         * `app.schoolId`가 첫 학교로 저절로 따라온다.
         *
         * **이 커맨드를 거치지 않고 학교 행을 직접 만들면 기본 출결 태그와 한도 규정이
         * 비어 있다.** 그 둘은 `create_school`이 학교마다 넣는다.
         */
        async createSchool({name = '', maxSlot = 7, dueDays = 7, dueSkipOffdays = true} = {}) {
            const app = useAppStore()
            this.error = ''
            try {
                const id = await invoke('create_school', {
                    yearId: app.yearId,
                    name,
                    maxSlot,
                    dueDays,
                    dueSkipOffdays,
                })
                await app.fetchSchools()
                return id
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 학교를 목록에서 내린다. **삭제가 아니다** — 그 학교의 학생 · 출결이 전부
         * 종속되어 있어 지우면 기록이 함께 사라진다. `active`만 0이 된다.
         *
         * 내린 것이 보고 있던 학교면 남은 첫 학교로 따라간다(`app.schoolId`).
         */
        async retireSchool(schoolId) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('retire_school', {schoolId})
                await app.fetchSchools()
                await app.fetchClasses()
                await app.saveScope()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 담당 학급 · 강좌를 더한다. **통째로 app 스토어에 맡긴다.**
         *
         * 전에는 다른 학교일 때만 여기서 직접 만들었는데, 그 갈래에는 "이미 담당하고 있으면
         * 만들지 않는다"는 확인이 없어 [추가]를 두 번 누르면 같은 학급이 두 줄
         * 생겼다. 만드는 길이 둘이면 그중 하나는 반드시 규칙을 빠뜨린다.
         */
        async createClass({
            schoolId, role, name, grade = null, classNo = null, groupTagId = null, select = true,
        }) {
            return useAppStore().createClass({schoolId, role, name, grade, classNo, groupTagId, select})
        },

        /**
         * 담당 학급 · 강좌의 이름과 묶음을 고친다. **이것은 UPDATE가 맞다** — 태그 · 규정과 달리
         * 학급 이름은 과거 기록이 가리키는 값이 아니라 그 행 하나를 부르는 이름이고,
         * 오타를 고치는 일이 더 잦다. 학년 · 반은 담임 학급만 가진다.
         *
         * **역할은 고칠 수 없다.** 담임 학급을 교과 강좌로 바꾸면 이미 달린 담임 출결이
         * 교과 강좌에 붙은 채 남는다. DB 트리거가 막는다.
         *
         * 묶음을 넘기지 않으면 **지금 것을 그대로 보낸다.** 커맨드가 다섯 값을 한 번에
         * 받으므로, 이름만 고치려던 호출이 이름표까지 조용히 풀어 버린다.
         */
        async renameClass(classId, {name, grade = null, classNo = null, groupTagId}) {
            const app = useAppStore()
            this.error = ''
            const kept = app.classes.find((c) => c.id === classId)?.groupTagId ?? null
            try {
                await invoke('update_teaching_class', {
                    classId, name, grade, classNo,
                    groupTagId: groupTagId === undefined ? kept : groupTagId,
                })
                await app.fetchClasses()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 담당 학급 · 강좌를 마감한다. **삭제가 아니다** — 지난 출결이 이 학급을 가리키므로
         * 지우면 기록이 함께 사라진다. 목록에서만 내려간다.
         *
         * 마감한 것이 보고 있던 학급이면 **같은 학교 · 같은 모드**의 남은 것으로 옮긴다.
         * 남은 것이 없으면 학급 없이 열린다 — 그것도 정상 상태이고, 화면이 무엇을
         * 등록해야 하는지 알린다. 모드를 말없이 넘기지 않는다.
         */
        async retireClass(classId) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('retire_teaching_class', {classId, validTo: app.today})
                // 사라진 번호를 지우고 남은 것으로 옮기는 것은 `fetchClasses`가 맡는다.
                await app.fetchClasses()
                await app.saveScope()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async createClassTag(name) {
            const app = useAppStore()
            this.error = ''
            try {
                const id = await invoke('create_class_tag', {schoolId: app.schoolId, name})
                await this.fetchAll()
                return id
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 묶음 이름을 고친다. **여기는 UPDATE가 맞다** — 출결 태그와 달리 이 이름은
         * 세는 대상이 아니라 화면에서 강좌를 묶어 보이는 이름표일 뿐이다.
         */
        async renameClassTag(id, name) {
            this.error = ''
            try {
                await invoke('rename_class_tag', {id, name})
                await this.fetchAll()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 묶음을 지운다. 강좌는 남고 이름표만 풀린다(`ON DELETE SET NULL`) —
         * 묶음은 화면의 이름일 뿐이라 지운다고 기록이 사라지지 않는다.
         */
        async deleteClassTag(id) {
            const app = useAppStore()
            this.error = ''
            try {
                await invoke('delete_class_tag', {id})
                await this.fetchAll()
                await app.fetchClasses()
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
