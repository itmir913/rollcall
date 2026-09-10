<script setup>
/**
 * 화면 하나의 머리와 본문. 모든 화면이 같은 여백과 같은 머리 구조를 갖는다.
 */
defineProps({
    title: {type: String, default: ''},
    subtitle: {type: String, default: ''},
})
</script>

<template>
    <section class="main">
        <div class="main__head">
            <div>
                <h3>{{ title }}</h3>
                <p v-if="subtitle" class="main__sub">{{ subtitle }}</p>
            </div>
            <div class="main__acts">
                <slot name="actions"/>
            </div>
        </div>
        <slot/>
    </section>
</template>
