<script setup>
/**
 * 휴지통 모양.
 *
 * **지우기는 글자가 아니라 이 모양 + 경고색이다.** 화면마다 모양을 다시 그리면
 * 한쪽만 굵기나 획이 달라져 다른 버튼과 섞여 읽힌다. 되돌릴 수 없는 동작이므로
 * 어느 화면에서나 같은 것으로 보여야 한다.
 *
 * 색은 이 아이콘을 감싸는 `UiButton variant="danger"`가 정한다 — `currentColor`를 따른다.
 */
</script>

<template>
    <svg aria-hidden="true" class="icon" fill="none" stroke="currentColor"
         stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"
         viewBox="0 0 24 24">
        <path d="M4 7h16"/>
        <path d="M10 4h4a1 1 0 0 1 1 1v2H9V5a1 1 0 0 1 1-1z"/>
        <path d="M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12"/>
        <path d="M10 11v6M14 11v6"/>
    </svg>
</template>
