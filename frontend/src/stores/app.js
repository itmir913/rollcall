import {defineStore} from 'pinia'
import {invoke} from '@tauri-apps/api/core'
import {academicYearOf, yearSpanOf} from '../services/academicYear'

/**
 * 담당 학급 · 강좌의 두 갈래. 담임은 구분 · 종류 · 기간을 기록하고, 교과는 그 교시에
 * 있었는지만 기록한다. 그래서 화면이 나뉘고, 한쪽의 숫자가 다른 쪽에 유출되면 안 된다.
 */
export const MODES = ['homeroom', 'subject']

/**
 * 앱이 지금 무엇을 보고 있는가 — **학년도 → 학교 → 담당 학급 · 강좌.**
 *
 * 이것이 이 앱의 계층이다. 전에는 학급을 선택하면 학교가 역산됐는데, 그러면 그 학년도에
 * 아직 담당 학급 · 강좌가 없는 학교를 가리킬 수 없다 — 순회 교사가 2027학년도에 B학교를
 * 추가하고 강좌를 아직 안 넣은 하루가 그 상태다. 그래서 **학교가 저장되는 범위**이고,
 * 담당 학급 · 강좌 목록은 그 학교 것만 온다(`get_teaching_classes(schoolId)`).
 *
 * **모드는 선택한 학급의 역할이다.** 따로 담지 않고 게터로 둔 이유는, 둘을 나란히
 * 담으면 담임 모드에 교과 강좌가 걸린 화면이 나올 수 있고 그때는 DB 트리거가
 * 거절할 때까지 아무도 모르기 때문이다. `lastMode`는 아직 아무 학급도 선택하지
 * 않았을 때 적용되는 값이고, **그 상태가 정상이다** — 비담임 교사가 담임 모드로 들어오면
 * 담임 학급이 없는 채로 열린다. 말없이 교과로 돌리지 않는다.
 *
 * 이 값들은 **앱 설정**(app_config)에 남는다. 담는 열쇠는 —
 * `yearId` · `schoolId` · `mode` · `homeroomClassId` · `subjectClassId`, 그리고
 * 시작한 적이 있는지를 적는 `onboarded`.
 * 최대 교시와 제출 기한은 여기가 아니라 **학교**가 들고 있다.
 */
export const useAppStore = defineStore('app', {
    state: () => ({
        booted: false,
        /** 이번 실행에서 DB 파일이 새로 만들어졌는가. `init_db`가 알려준다. */
        firstRun: false,
        error: '',
        /**
         * 부팅이 실패한 이유. **`error`와 따로 담는다** — `error`는 뒤이은 액션이
         * 비우므로, 부팅이 실패한 것을 "아직 아무것도 없다"와 구별해 주지 못한다.
         * 읽지 못한 것과 시작하지 않은 것은 둘 다 빈 목록으로 보인다.
         */
        bootError: '',
        /**
         * 담당 학급 · 강좌를 한 번이라도 만든 적이 있는가. **첫 실행 판단의 근거다.**
         * 설정(`app_config`)의 `onboarded`에 남으므로 학년도를 옮겨도 따라온다 —
         * 지금 담당 학급 · 강좌가 0개인 것은 시작하지 않은 것이 아니라 추가할 차례다.
         */
        onboarded: false,
        years: [],
        yearId: null,
        /** 이 학년도의 학교. 학교는 학년도 안에 있다. */
        schools: [],
        /**
         * 교사가 선택한 학교. **목록에 없는 번호는 `schoolId` 게터가 첫 학교로 되돌린다** —
         * 학년도를 바꾸면 지난해 학교 번호가 설정에 남아 있고, 그 번호로 묻는 질의는 전부 빈다.
         */
        pickedSchoolId: null,
        /**
         * 이 학년도의 담당 학급 · 강좌 전부. **학교가 여럿이면 여러 학교 것이 한 목록에 온다.**
         * 화면이 보는 것은 지금 학교 것(`schoolClasses`)이고, 이 목록 전체는
         * "아직 시작하지 않았는가"와 "이미 담당하고 있는가"를 판단하는 데 쓴다.
         */
        classes: [],
        /** 지금 보고 있는 학급. 담임 · 교과 커맨드가 받는 단 하나의 범위다. */
        classId: null,
        /**
         * 모드마다 마지막에 본 학급. **오가며 쓰는 값이라 따로 기억한다** —
         * 돌아왔을 때 있던 자리가 아니면 매번 다시 선택해야 한다.
         */
        lastClassId: {homeroom: null, subject: null},
        /** 학급을 선택 전까지의 모드. 선택한 뒤에는 학급의 역할이 우선한다. */
        lastMode: 'homeroom',
        today: isoToday(),
    }),

    getters: {
        currentYear: (s) => s.years.find((y) => y.id === s.yearId) ?? null,
        /**
         * 지금 보고 있는 학교. 선택한 적이 없거나 그 학교가 사라졌으면 첫 학교다 —
         * 학교를 가리키지 못하면 설정도 명단도 읽을 곳이 없다.
         */
        schoolId: (s) => schoolOf(s)?.id ?? null,
        school: (s) => schoolOf(s),
        /** 하루의 마지막 교시. 화면이 앱 상수를 알지 않게 한다. */
        maxSlot: (s) => schoolOf(s)?.maxSlot ?? 7,
        /**
         * 지금 학교의 담당 학급 · 강좌. **역할이 결정되는 자리다** —
         * A학교에서는 담임 + 교과, B학교에서는 교과만. 그것이 순회 교사다.
         */
        schoolClasses: (s) => s.classes.filter((c) => c.schoolId === (schoolOf(s)?.id ?? null)),
        /** 지금 보고 있는 학급 행. 이름 · 역할 · 학교가 여기 붙어 있다. */
        currentClass: (s) => classOf(s),
        /**
         * homeroom | subject. **선택한 학급이 결정한다.**
         *
         * 화면이 이 값으로 구별되므로, 학급과 어긋나는 순간 담임 화면이 교과 강좌의
         * 명단을 그린다. 그래서 따로 담지 않고 여기서 읽는다.
         */
        mode: (s) => classOf(s)?.role ?? s.lastMode,
        /** 지금 담임 모드인가. 화면이 문자열을 직접 비교하지 않게 한다. */
        isHomeroom: (s) => (classOf(s)?.role ?? s.lastMode) === 'homeroom',
        homeroomClasses: (s) => schoolClassesOf(s).filter((c) => c.role === 'homeroom'),
        subjectClasses: (s) => schoolClassesOf(s).filter((c) => c.role === 'subject'),
        /**
         * 그 모드로 들어갔을 때 보여줄 것이 있는가.
         *
         * **없어도 정상 상태다.** 비담임 교사에게도 스위치는 늘 보이고, 담임 모드로
         * 들어오면 빈 화면과 [담임 학급 추가]가 뜬다. 사이드바 항목은 지우지 않고
         * 비활성화한다 — 항목이 사라지면 이 앱의 절반이 고장 난 것처럼 보인다.
         */
        hasClassIn: (s) => (mode) => schoolClassesOf(s).some((c) => c.role === mode),
        /** 지금 모드에서 선택한 학급이 있는가. 없으면 화면들이 질의를 던지지 않는다. */
        ready: (s) => s.classId != null,
        /** 부팅이 실패했는가. 화면은 이것으로 "읽지 못했다"와 "아직 없다"를 구별한다. */
        bootFailed: (s) => s.booted && s.bootError !== '',
        /**
         * Welcome 화면으로 보내야 하는가. 흐름은 `Welcome → 개요`다.
         *
         * **담당 학급 · 강좌를 한 번이라도 만든 적이 있는가로 판단한다.** 지금 몇 개를
         * 담당하는가로 보면 두 자리에서 틀린다 — 3월에 지난해 학급을 전부 마감한 교사가
         * 사이드바도 없는 마법사로 튕기고, 새 학년도로 옮긴 직후도 같은 모습이 된다. 그 둘은
         * 시작하지 않은 것이 아니라 **추가할 차례**다.
         *
         * **부팅이 실패했으면 보내지 않는다.** 읽지 못해 비어 있는 것과 아직 만들지
         * 않아 비어 있는 것은 화면에서 같아 보이는데, 앞의 경우 마법사로 보내면
         * 교사는 이미 넣은 학교와 명렬표가 사라졌다고 읽는다.
         *
         * 지금 담당 학급 · 강좌가 있으면 그것도 시작한 증거다. 설정에 적는 것이 한 번이라도
         * 실패했을 때 그 교사만 마법사에 갇히는 일이 없도록 둘을 함께 본다.
         *
         * `firstRun`으로 판단하지 않는 이유는 따로 있다 — 교사가 Welcome을 끝내지 않고
         * 앱을 닫으면 다음 실행부터 영영 거짓이 되어, 사이드바에 항목도 없는 그 화면에
         * 다시 닿을 길이 없어진다.
         */
        needsWelcome: (s) =>
            s.booted && s.bootError === '' && !s.onboarded && s.classes.length === 0,
    },

    actions: {
        async boot() {
            this.error = ''
            this.bootError = ''
            try {
                // 파일이 이번에 만들어졌는지는 `init_db`만 안다. 버리면 첫 실행인지
                // 알 방법이 없어, 처음 켠 교사가 안내 없이 개요 화면부터 보게 된다.
                const status = await invoke('init_db')
                this.firstRun = Boolean(status?.created)

                this.years = await invoke('get_years')
                // **첫 실행에는 학년도 행이 없다.** 시드가 만들지 않는 이유는 오늘이
                // 언제인지 DB가 모르기 때문이다. 여기서 오늘로 채운다 — 교사에게
                // 되묻지 않는다. 아직 학교도 읽지 않았으므로 옮기지는 않는다.
                if (this.years.length === 0) {
                    await this.createYear(academicYearOf(this.today), {select: false})
                }

                // app_config는 키 하나씩 읽는다. 범위 다섯과 시작 여부가 전부다.
                const [year, school, mode, homeroom, subject, onboarded] = await Promise.all([
                    invoke('get_config', {key: 'yearId'}),
                    invoke('get_config', {key: 'schoolId'}),
                    invoke('get_config', {key: 'mode'}),
                    invoke('get_config', {key: 'homeroomClassId'}),
                    invoke('get_config', {key: 'subjectClassId'}),
                    invoke('get_config', {key: 'onboarded'}),
                ])
                const kept = toNumber(year)
                this.yearId = this.years.find((y) => y.id === kept)?.id ?? this.years[0]?.id ?? null
                this.pickedSchoolId = toNumber(school)
                this.lastMode = MODES.includes(mode) ? mode : 'homeroom'
                this.onboarded = onboarded === '1'

                await this.fetchSchools()
                await this.fetchClasses()
                this.restore({homeroom: toNumber(homeroom), subject: toNumber(subject)})
                // 담당 학급 · 강좌가 있다는 것은 이미 시작했다는 뜻이다. 이 열쇠가 없던 때에
                // 쓰던 설정에는 값이 적혀 있지 않으므로 여기서 한 번 채운다 —
                // 채우지 않으면 그 교사는 학급을 전부 마감한 날 마법사로 돌아간다.
                if (this.classes.length > 0) await this.markStarted()
                this.booted = true
            } catch (e) {
                this.error = String(e)
                this.bootError = this.error
                this.booted = true
                throw e
            }
        },

        /**
         * 시작한 적이 있다고 적는다. **한 번만 쓴다** — 이미 적혀 있으면 아무것도
         * 하지 않으므로, 부팅마다 설정에 같은 값을 다시 쓰지 않는다.
         *
         * 실패를 삼키지 않는다. 적지 못하면 다음 실행에서 다시 마법사로 가고,
         * 그때 교사는 자기가 만든 학급이 사라졌다고 읽는다.
         */
        async markStarted() {
            if (this.onboarded) return
            try {
                await invoke('set_config', {key: 'onboarded', value: '1'})
                this.onboarded = true
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        async fetchSchools() {
            this.error = ''
            try {
                this.schools = this.yearId == null
                    ? []
                    : await invoke('get_schools', {yearId: this.yearId})
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /** 옛 이름. 화면이 `fetchSchools`로 옮겨 가면 지운다. */
        async refreshSchool() {
            return this.fetchSchools()
        },

        /**
         * 이 학년도의 담당 학급 · 강좌를 전부 읽는다. **학교마다 한 번씩 묻는다** —
         * 커맨드의 범위가 학교 하나이기 때문이고, "아직 시작하지 않았다"는 판단은
         * 학년도 전체를 봐야 하기 때문이다. 한 학교만 읽으면 그 학교에 아직 담당 학급 · 강좌가
         * 없다는 것과 앱을 처음 켰다는 것이 같은 모양으로 보인다.
         */
        async fetchClasses() {
            this.error = ''
            try {
                const lists = await Promise.all(
                    this.schools.map((s) => invoke('get_teaching_classes', {schoolId: s.id})),
                )
                this.classes = lists.flat()
                // 목록을 새로 읽을 때마다 기억을 목록에 맞춘다. 만드는 것도 마감하는 것도
                // 설정에서 일어나고 그 뒤에 반드시 이 액션이 돌므로, 그 자리마다 따로
                // 적지 않아도 된다.
                this.syncClassMemory()
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 설정에 적힌 학급을 복원한다. **목록에 없는 번호는 버린다** —
         * 학년도가 바뀌어 지난해 학급이 마감되면 그 번호로 묻는 질의가 전부 빈다.
         * 적힌 것이 없으면 그 역할의 첫 학급을 선택한다.
         *
         * **마지막에 본 모드는 그대로 둔다.** 그 모드에 담당 학급 · 강좌가 없으면 학급 없이
         * 열린다 — 담임 학급이 없는 담임 모드가 정상 상태이고, 말없이 반대 모드로
         * 열면 교사는 자기가 무엇을 눌렀는지 모른 채 다른 화면을 보게 된다.
         */
        restore({homeroom = null, subject = null} = {}) {
            const pick = (id, role) => {
                const own = this.schoolClasses.filter((c) => c.role === role)
                return own.find((c) => c.id === id)?.id ?? own[0]?.id ?? null
            }
            this.lastClassId = {
                homeroom: pick(homeroom, 'homeroom'),
                subject: pick(subject, 'subject'),
            }
            this.classId = this.lastClassId[this.lastMode] ?? null
        },

        /**
         * 모드마다 기억해 둔 학급을 목록에 맞춘다. **사라진 것을 지우고 빈 자리를 채운다.**
         *
         * 마감한 학급이 `lastClassId`에 남아 있으면, 스위치를 눌렀을 때 목록에 없는
         * 학급으로 돌아간다 — 사이드바는 "담임 학급 없음"을 표시하는데 화면들은 그 번호로
         * 질의를 던지므로, 무엇이 잘못됐는지 보이지 않는다.
         *
         * **비어 있는 자리도 같은 고장이다.** 설정에서 첫 교과 강좌를 `select: false`로
         * 만들면 기억이 `null`인 채로 남아, 교과 스위치를 눌러도 학급이 없어 화면이
         * "아직 교과 강좌를 추가하지 않았습니다"라고 잘못 말한다. 사이드바는 열려 있으므로
         * 교사는 이동 화면에서 손으로 선택하거나 앱을 껐다 켜야 회복한다.
         * **선택하는 것과 기억하는 것은 다른 일이다** — 만든 것을 곧바로 보여주지
         * 않더라도 그 모드의 자리는 채워 둔다.
         */
        syncClassMemory() {
            const alive = new Set(this.classes.map((c) => c.id))
            for (const mode of MODES) {
                const kept = this.lastClassId[mode]
                if (kept == null || !alive.has(kept)) {
                    this.lastClassId[mode] =
                        this.schoolClasses.find((c) => c.role === mode)?.id ?? null
                }
            }
            // 보고 있는 학급도 같은 규칙이다. **모드는 넘기지 않는다** — 지금 모드의
            // 기억에서만 채우므로, 담임 모드가 교과 강좌로 열리는 일은 없다.
            if (this.classId == null || !alive.has(this.classId)) {
                this.classId = this.lastClassId[this.lastMode] ?? null
            }
        },

        /**
         * 범위 다섯을 한 번에 적는다. **한 곳에서만 쓴다** — 액션마다 따로 적으면
         * 어느 하나가 빠졌을 때 다음 실행에서 엉뚱한 자리가 열리고, 그것을 재현하려면
         * 앱을 껐다 켜야 한다.
         */
        async saveScope() {
            this.error = ''
            try {
                await Promise.all([
                    invoke('set_config', {key: 'yearId', value: idText(this.yearId)}),
                    invoke('set_config', {key: 'schoolId', value: idText(this.schoolId)}),
                    invoke('set_config', {key: 'mode', value: this.lastMode}),
                    invoke('set_config', {
                        key: 'homeroomClassId', value: idText(this.lastClassId.homeroom),
                    }),
                    invoke('set_config', {
                        key: 'subjectClassId', value: idText(this.lastClassId.subject),
                    }),
                ])
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 학년도를 만들고 그리로 옮긴다.
         *
         * **여는 날 · 닫는 날은 묻지 않는다.** 3월에 열리는 학년도라 계산으로 나오고,
         * 되물어 봐야 교사가 달력을 다시 확인하게 할 뿐이다. 이미 있는 해면 만들지
         * 않고 그리로 옮긴다 — 같은 해를 두 번 만들면 UNIQUE가 거절한다.
         *
         * 부팅이 첫 학년도를 채울 때는 옮기지 않는다(`select: false`). 그때는 학교도
         * 설정도 아직 읽기 전이라, 옮기는 일이 곧바로 뒤에 한 번 더 일어난다.
         */
        async createYear(year, {select = true} = {}) {
            this.error = ''
            const same = this.years.find((y) => y.year === year)
            if (same) {
                if (select) await this.selectYear(same.id)
                return same.id
            }
            try {
                const {startsOn, endsOn} = yearSpanOf(year)
                const id = await invoke('create_year', {year, startsOn, endsOn})
                this.years = await invoke('get_years')
                if (select) await this.selectYear(id)
                return id
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },

        /**
         * 학년도를 바꾼다. **학교부터 다시 선택한다** — 학교는 학년도 안에 있고,
         * 2026학년도의 A학교와 2027학년도의 A학교는 다른 행이다. 지난해 학교 번호를
         * 그대로 들고 있으면 최대 교시 · 제출 기한을 지난해 값으로 읽는다.
         */
        async selectYear(yearId) {
            this.error = ''
            const target = this.years.find((y) => y.id === yearId)
            if (!target) {
                this.error = `없는 학년도입니다: ${yearId}`
                throw new Error(this.error)
            }
            this.yearId = target.id
            this.pickedSchoolId = null
            await this.fetchSchools()
            await this.fetchClasses()
            this.restore()
            await this.saveScope()
        },

        /**
         * 학년도를 지운다. **그 아래가 통째로 사라진다** — 학교 · 담당 학급 · 강좌 ·
         * 명단 · 출결 · 교과 차시가 전부 함께 지워진다(CASCADE). 되돌릴 수 없으므로
         * 화면은 묻기 전에 무엇이 사라지는지 보여준다 — 그 수는 목록 한 줄
         * (`get_years`)에 이미 실려 온다(`schoolCount` · `classCount` · `studentCount` ·
         * `spanCount` · `sessionCount`).
         *
         * **보고 있던 학년도를 지웠으면 남은 것으로 옮긴다.** Rust가 설정(`app_config`)의
         * 범위 열쇠를 비우지만 그것은 다음 실행에서야 읽힌다 — 지금 화면은 사라진 학년도의
         * 번호와 그 아래 학교 · 학급을 그대로 들고 있고, 질의마다 빈 결과가 돌아오는데
         * 이유는 어디에도 표시되지 않는다. 그래서 메모리의 범위를 여기서 함께 비우고
         * `selectYear`로 옮긴다 — 학교와 담당 학급 · 강좌를 다시 읽는 일이 그쪽에 있다.
         *
         * **다른 학년도를 지운 것이면 목록만 다시 읽는다.** 보고 있던 자리는 그대로다.
         * 옮겨 버리면 교사가 보던 화면이 이유 없이 처음으로 돌아간다.
         *
         * 마지막 하나는 Rust가 거절한다 — 학년도가 0개가 되면 학교도 담당 학급 · 강좌도
         * 만들 수 없다. 거절 문구를 `error`에 담고 **다시 던진다.**
         */
        async deleteYear(yearId) {
            this.error = ''
            const watching = this.yearId === yearId
            try {
                await invoke('delete_year', {yearId})
                this.years = await invoke('get_years')
            } catch (e) {
                this.error = String(e)
                throw e
            }
            if (!watching) return
            // 사라진 번호를 **먼저** 비운다. 옮기다가 실패해도 없는 학년도와 그 아래
            // 학교 · 학급을 가리킨 채로 남지 않는다. 열쇠가 비어 있다는 것은
            // "아직 선택하지 않았다"는 뜻이고, 지운 직후가 정확히 그 상태다.
            this.yearId = null
            this.pickedSchoolId = null
            this.classId = null
            this.lastClassId = {homeroom: null, subject: null}
            this.schools = []
            this.classes = []
            // 목록은 최신 학년도가 먼저다. 부팅이 선택하는 것과 같은 규칙이라,
            // 여기서 다른 규칙을 쓰면 지운 뒤와 다시 켠 뒤가 서로 다른 자리를 연다.
            const next = this.years[0]?.id ?? null
            if (next != null) await this.selectYear(next)
        },

        /**
         * 학교를 옮긴다. **담당 학급 · 강좌도 그 학교 것으로 다시 선택한다** —
         * 학교만 바뀌고 학급이 남으면 B학교 화면에 A학교 명단이 그려진다.
         *
         * 담당 학급 · 강좌가 아직 없는 학교도 가리킬 수 있다. 그 상태에서는 학급 없이 열리고,
         * 화면이 무엇을 추가해야 하는지 알린다.
         */
        async selectSchool(schoolId) {
            this.error = ''
            const target = this.schools.find((s) => s.id === schoolId)
            if (!target) {
                this.error = `없는 학교입니다: ${schoolId}`
                throw new Error(this.error)
            }
            this.pickedSchoolId = target.id
            this.restore(this.lastClassId)
            await this.saveScope()
        },

        /**
         * 지금 볼 학급을 선택한다. **모드가 그 학급의 역할로 따라온다** —
         * 선택한 것과 보고 있는 것이 어긋날 자리를 남기지 않는다.
         *
         * 다른 학교의 것을 선택했으면 학교도 함께 옮긴다. 학교가 뒤에 남으면 그 학교의
         * 최대 교시 · 제출 기한으로 다른 학교의 출결을 입력하게 된다.
         */
        async selectClass(classId) {
            this.error = ''
            const target = this.classes.find((c) => c.id === classId)
            if (!target) {
                this.error = `담당 학급 · 강좌가 아닙니다: ${classId}`
                throw new Error(this.error)
            }
            this.pickedSchoolId = target.schoolId
            this.lastMode = target.role
            // 반대 모드의 기억은 `restore`가 이 학교 것으로 다시 맞춘다.
            this.restore({...this.lastClassId, [target.role]: target.id})
            await this.saveScope()
        },

        /**
         * 담임과 교과를 오간다. 학급은 **그 모드에서 마지막에 본 것**으로 돌아온다 —
         * 매번 다시 선택하게 하면 하루에도 몇 번씩 오가는 교사에게 그만큼 클릭이 는다.
         *
         * **그 모드에 담당 학급 · 강좌가 없어도 넘어간다.** 스위치는 늘 보이고, 비어 있는 쪽은
         * 비어 있다고 말하는 화면이 받는다.
         */
        async setMode(mode) {
            this.error = ''
            if (!MODES.includes(mode)) {
                this.error = `모르는 모드입니다: ${mode}`
                throw new Error(this.error)
            }
            this.lastMode = mode
            this.classId = this.lastClassId[mode] ?? null
            await this.saveScope()
        },

        /**
         * 담당 학급 · 강좌를 새로 만들고 그것으로 옮긴다. Welcome과 설정이 부른다.
         *
         * **이미 담당하고 있으면 만들지 않고 선택한다.** 교사가 [저장]을 두 번 누르는 것은
         * 흔한 일이고, 그때마다 학급이 늘면 명단이 어느 쪽에 붙었는지 알 수 없게 된다.
         * 담임은 학년 · 반이, 교과는 이름이 같은 것을 가리킨다.
         */
        async createClass({
            role = 'homeroom', name = null, grade = null, classNo = null,
            // 어느 학교에 만드는가. 비우면 지금 보고 있는 학교다 — 순회 교사가
            // 새 학교를 만든 직후에는 그 학교를 가리켜야 첫 학급이 제자리에 생성된다.
            schoolId = null,
            // 화면에서 함께 묶어 보일 이름표. `프로그래밍A · B · C`를 묶는다.
            groupTagId = null,
            // 만든 것을 곧바로 선택할지. **설정에서는 선택하지 않는다** — 교과 강좌 하나를
            // 더했다고 화면이 통째로 교과 모드로 넘어가면 무엇을 잘못 눌렀는지 확인하게 된다.
            select = true,
        }) {
            this.error = ''
            const school = schoolId ?? this.schoolId
            if (school == null) {
                this.error = '학교를 먼저 추가해야 담당 학급 · 강좌를 만들 수 있습니다.'
                throw new Error(this.error)
            }
            // **학교까지 본다.** `classes`는 학년도 전체라 다른 학교의 학급도 들어 있고,
            // 학교를 보지 않으면 A 학교의 `통합사회`가 B 학교에 만들려던 것을 가로챈다.
            const same = this.classes.find(
                (c) =>
                    c.role === role &&
                    !c.validTo &&
                    c.schoolId === school &&
                    (role === 'homeroom'
                        ? c.grade === grade && c.classNo === classNo
                        : c.name === name),
            )
            if (same) {
                if (select) await this.selectClass(same.id)
                return same.id
            }
            try {
                const id = await invoke('create_teaching_class', {
                    schoolId: school,
                    role,
                    name,
                    grade,
                    classNo,
                    groupTagId,
                    // 학급이 언제부터 내 것인가. 학년도가 열리는 날이 기본이고,
                    // 학기 중에 맡게 되면 그날부터다. 마감은 valid_to가 맡는다.
                    validFrom: this.currentYear?.startsOn ?? this.today,
                })
                await this.fetchClasses()
                if (select) await this.selectClass(id)
                // 담당 학급 · 강좌를 만든 순간이 곧 시작한 순간이다. 선택하지 않고 만들었어도
                // 마찬가지다 — 설정에서 강좌 하나를 더한 교사도 이미 시작한 교사다.
                // **보여주는 일을 끝낸 뒤에 적는다.** 적는 데 실패해도 만든 학급은
                // 그 자리에 열려 있어야 한다.
                await this.markStarted()
                return id
            } catch (e) {
                this.error = String(e)
                throw e
            }
        },
    },
})

/** 지금 보고 있는 학급 행. 게터 여럿이 같은 것을 묻기에 한 곳에 둔다. */
function classOf(state) {
    return state.classes.find((c) => c.id === state.classId) ?? null
}

/**
 * 지금 보고 있는 학교. **선택한 것이 목록에 없으면 첫 학교다** —
 * 학년도를 바꾸면 지난해 학교 번호가 설정에 남아 있고, 가리킬 학교가 없으면
 * 설정도 명단도 읽을 곳이 없어 화면이 통째로 빈다.
 */
function schoolOf(state) {
    return (
        state.schools.find((s) => s.id === state.pickedSchoolId) ?? state.schools[0] ?? null
    )
}

/** 지금 학교의 담당 학급 · 강좌. 게터 셋이 같은 것을 묻기에 한 곳에 둔다. */
function schoolClassesOf(state) {
    const id = schoolOf(state)?.id ?? null
    return state.classes.filter((c) => c.schoolId === id)
}

/** 오늘 날짜를 ISO로. 저장은 언제나 ISO다. */
export function isoToday(now = new Date()) {
    const pad = (n) => String(n).padStart(2, '0')
    return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

/** 화면·파일 저장 표기. `2026.09.10.(목)` */
export function formatKorean(iso) {
    if (!iso) return ''
    const [y, m, d] = iso.split('-').map(Number)
    const week = ['일', '월', '화', '수', '목', '금', '토'][new Date(y, m - 1, d).getDay()]
    return `${y}.${String(m).padStart(2, '0')}.${String(d).padStart(2, '0')}.(${week})`
}

function toNumber(v) {
    const n = Number(v)
    return Number.isFinite(n) && v !== undefined && v !== null && v !== '' ? n : null
}

/** app_config는 문자열만 담는다. 비어 있는 것은 빈 문자열이고, 읽을 때 null로 돌아온다. */
function idText(id) {
    return id == null ? '' : String(id)
}
