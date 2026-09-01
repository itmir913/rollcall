<script setup>
/** 화면 하나의 껍데기. 제목 줄과 본문 간격을 통일한다. */
defineProps({
    title: {type: String, default: ''},
    subtitle: {type: String, default: ''},
})
</script>

<template>
    <div class="ui-page">
        <header v-if="title || $slots.actions" class="ui-page__head">
            <div>
                <h1 class="ui-page__title">{{ title }}</h1>
                <p v-if="subtitle" class="ui-page__subtitle">{{ subtitle }}</p>
            </div>
            <div v-if="$slots.actions" class="ui-page__actions">
                <slot name="actions"/>
            </div>
        </header>
        <slot/>
    </div>
</template>

<style scoped>
.ui-page {
    display: flex;
    flex-direction: column;
    gap: 18px;
}

.ui-page__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
}

.ui-page__title {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
}

.ui-page__subtitle {
    margin: 4px 0 0;
    color: var(--c-ink-3);
}

.ui-page__actions {
    display: flex;
    gap: 8px;
    align-items: center;
}
</style>
