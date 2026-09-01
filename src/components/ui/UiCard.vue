<script setup>
/** 한 덩어리의 내용. 제목과 설명을 함께 받는다. */
defineProps({
    title: {type: String, default: ''},
    description: {type: String, default: ''},
})
</script>

<template>
    <section class="ui-card">
        <header v-if="title || $slots.actions" class="ui-card__head">
            <div>
                <h2 v-if="title" class="ui-card__title">{{ title }}</h2>
                <p v-if="description" class="ui-card__desc">{{ description }}</p>
            </div>
            <div v-if="$slots.actions" class="ui-card__actions">
                <slot name="actions"/>
            </div>
        </header>
        <slot/>
    </section>
</template>

<style scoped>
.ui-card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px;
    background: var(--c-surface);
    border: 1px solid var(--c-line);
    border-radius: 12px;
    box-shadow: var(--c-shadow);
}

.ui-card__head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
}

.ui-card__title {
    font-size: 1.05rem;
    font-weight: 600;
    margin: 0;
}

.ui-card__desc {
    margin: 6px 0 0;
    color: var(--c-ink-3);
}

.ui-card__actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
}
</style>
