<script setup>
/**
 * 표. 열 정의만 받고 각 행은 슬롯으로 그린다.
 *
 * 표의 테두리·간격·머리글 색을 한 곳에 모아 두려는 것이지, 내용까지 추상화하려는
 * 것이 아니다. 셀 내용은 화면마다 다르므로 슬롯에 맡긴다.
 */
defineProps({
    /** [{key, label, width?, align?}] */
    columns: {type: Array, required: true},
    rows: {type: Array, required: true},
    rowKey: {type: String, default: 'id'},
    emptyText: {type: String, default: '표시할 내용이 없습니다.'},
})
</script>

<template>
    <div class="ui-table__wrap">
        <table class="ui-table">
            <thead>
            <tr>
                <th v-for="col in columns" :key="col.key"
                    :style="{width: col.width, textAlign: col.align || 'left'}">
                    {{ col.label }}
                </th>
            </tr>
            </thead>
            <tbody>
            <tr v-for="(row, index) in rows" :key="row[rowKey] ?? index">
                <slot :index="index" :row="row" name="row"/>
            </tr>
            <tr v-if="!rows.length">
                <td :colspan="columns.length" class="ui-table__empty">{{ emptyText }}</td>
            </tr>
            </tbody>
        </table>
    </div>
</template>

<style scoped>
.ui-table__wrap {
    overflow-x: auto;
}

.ui-table {
    width: 100%;
    border-collapse: collapse;
}

.ui-table :deep(th) {
    padding: 10px 12px;
    color: var(--c-ink-3);
    font-weight: 500;
    white-space: nowrap;
}

.ui-table :deep(td) {
    padding: 8px 12px;
    border-top: 1px solid var(--c-line);
    vertical-align: middle;
}

.ui-table__empty {
    color: var(--c-ink-3);
    text-align: center;
    padding: 28px 12px;
}
</style>
