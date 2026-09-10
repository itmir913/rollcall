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
import RosterImport from '../components/RosterImport.vue'
import {UiButton, UiLedger, UiNotice} from '../components/ui'

const app = useAppStore()
const school = useSchoolStore()
const router = useRouter()

const form = ref({name: '', maxSlot: 7, grade: 3, classNo: 1})
const error = ref('')

const SLOT_CHOICES = [1, 2, 3, 4, 5, 6, 7, 8, 9]

const canFinish = computed(() => app.ready)

async function saveSchool() {
    error.value = ''
    try {
        await school.saveSchool({name: form.value.name.trim() || '우리 학교', maxSlot: form.value.maxSlot})
        await app.selectClass({
            yearId: app.yearId ?? app.years[0]?.id,
            grade: Number(form.value.grade),
            classNo: Number(form.value.classNo),
        })
    } catch (e) {
        error.value = String(e)
    }
}

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
                    <button v-for="n in SLOT_CHOICES" :key="n"
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
                <span class="set__value"><RosterImport/></span>
            </div>
        </UiLedger>

        <div class="main__acts">
            <UiButton :disabled="!canFinish" variant="primary" @click="router.push('/')">
                시작하기
            </UiButton>
        </div>
    </div>
</template>
