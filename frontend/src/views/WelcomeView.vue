<script setup>
/**
 * 첫 실행 — 학교와 학급을 정하고 명렬표를 넣는다.
 *
 * 이 화면은 **첫 실행에서만** 지나간다. 매일 열자마자 바로 입력할 수 있어야 하므로
 * 개요가 곧 기본 화면이고, 여기는 그 앞에 한 번 서는 자리다.
 */
import {computed, onMounted, ref} from 'vue'
import {useRouter} from 'vue-router'
import {useAppStore} from '../stores/app'
import {useSchoolStore} from '../stores/school'
import RosterPanel from '../components/RosterPanel.vue'
import {UiButton, UiLedger, UiNotice} from '../components/ui'
import {MAX_SLOT_CHOICES} from '../data/slotChoices'

const app = useAppStore()
const school = useSchoolStore()
const router = useRouter()

const form = ref({name: '', maxSlot: 7, grade: 3, classNo: 1})
const error = ref('')

const canFinish = computed(() => app.ready)

/**
 * 학교를 저장하고 담임 학급을 만든다.
 *
 * **학급은 이제 행이다.** 명렬표도 출결도 이 학급(`classId`)에 매달리므로 명렬표보다
 * 먼저 있어야 한다. 같은 학년 · 반으로 다시 저장해도 학급이 늘지 않는 것은
 * `app.createClass`가 맡는다 — 교사가 [저장]을 두 번 누르는 것은 흔한 일이고,
 * 그때마다 학급이 늘면 명단이 어느 쪽에 붙었는지 알 수 없게 된다.
 *
 * 학급 이름은 화면에 적히는 값이라 여기서 짓는다(`3학년 6반`). 교과 강좌는 반이
 * 섞여 이렇게 지을 수 없으므로, 이 규칙은 담임 학급에만 해당한다.
 */
async function saveSchool() {
    error.value = ''
    const grade = Number(form.value.grade)
    const classNo = Number(form.value.classNo)
    try {
        await school.saveSchool({name: form.value.name.trim() || '우리 학교', maxSlot: form.value.maxSlot})
        await app.createClass({
            role: 'homeroom', name: `${grade}학년 ${classNo}반`, grade, classNo,
        })
    } catch (e) {
        error.value = String(e)
    }
}

// 읽기에 실패해도 화면은 그린다. 실패는 `school.error`에 담겨 아래 UiNotice가
// 그대로 보여준다 — 첫 화면이 기본값만 놓인 멀쩡한 모습으로 보이면 안 된다.
onMounted(async () => {
    await school.fetchAll().catch(() => {
    })
    form.value.name = school.school?.name ?? ''
    form.value.maxSlot = school.school?.maxSlot ?? 7
})
</script>

<template>
    <div class="main">
        <div class="main__head">
            <div>
                <h3>출결관리를 시작합니다</h3>
                <p class="main__sub">학교와 학급을 한 번만 정하면, 다음부터는 바로 오늘 화면이 열립니다.</p>
            </div>
        </div>

        <UiNotice :text="error" kind="error"/>
        <UiNotice :text="school.error" kind="error"/>

        <UiLedger hint="나중에 설정에서 바꿀 수 있습니다" title="학교">
            <div class="set__row">
                <span class="set__label">학교 이름</span>
                <span class="set__value">
                    <input v-model="form.name" class="field" placeholder="한빛고등학교" type="text"/>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">최대 교시</span>
                <span class="set__value">
                    <button v-for="n in MAX_SLOT_CHOICES" :key="n"
                            :class="['pick', 'pick--slot', form.maxSlot === n ? 'is-on' : '']"
                            type="button" @click="form.maxSlot = n">
                        {{ n }}
                    </button>
                    <span class="set__hint">조회와 종례는 언제나 하루의 양 끝입니다</span>
                </span>
            </div>
            <div class="set__row">
                <span class="set__label">우리 반</span>
                <span class="set__value">
                    <input v-model="form.grade" class="field num" min="1" type="number"/>
                    <span class="set__hint">학년</span>
                    <input v-model="form.classNo" class="field num" min="1" type="number"/>
                    <span class="set__hint">반</span>
                    <UiButton variant="primary" @click="saveSchool">저장</UiButton>
                </span>
            </div>
        </UiLedger>

        <UiLedger hint="번호와 이름만 있으면 됩니다" title="명렬표">
            <div class="set__row">
                <span class="set__label">파일에서 가져오기</span>
                <span class="set__value">
                    <!-- 명단이 붙을 곳은 학급이다. 학급이 없는 동안 가져오기를 열어 두면
                         파일을 읽고 [저장]까지 누른 뒤에야 갈 곳이 없다는 것을 안다. -->
                    <RosterPanel v-if="app.ready"/>
                    <span v-else class="set__hint">위에서 우리 반을 먼저 저장해주세요</span>
                </span>
            </div>
        </UiLedger>

        <div class="main__acts">
            <UiButton :disabled="!canFinish" variant="primary" @click="router.push('/')">
                시작하기
            </UiButton>
        </div>
    </div>
</template>
