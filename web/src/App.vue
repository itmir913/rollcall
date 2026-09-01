<script setup>
/**
 * 아직 골격이다.
 *
 * 만들 것: 결석한 학생 번호를 눌러 고르면 짧은 코드가 나오고, 교사가 그 코드를
 * 데스크톱 앱에 넣으면 그 학생들이 한꺼번에 기록되는 화면.
 *
 * 코드는 생마스크 6자(base32)다. 30명 중 15명이 결석하는 규모에서는 조합 압축이
 * 무의미하다는 계산이 이미 나와 있다(명세서 §10).
 *
 * **이 화면은 서버도 DB도 쓰지 않는다.** 번호를 코드로 바꾸는 계산뿐이므로
 * 정적 페이지로 충분하고, 그래서 GitHub Pages에 올라간다.
 * 학생 이름은 여기 오지 않는다 — 번호만 다룬다.
 */
</script>

<template>
    <main class="page">
        <h1 class="title">출결관리 웹앱</h1>
        <p class="lead">
            결석한 학생 번호를 골라 짧은 코드를 만드는 화면입니다. 아직 준비 중입니다.
        </p>
        <p class="note">
            이 페이지는 번호를 코드로 바꾸는 계산만 합니다. 이름이나 사유는 다루지 않고,
            서버로 나가는 것도 없습니다.
        </p>
    </main>
</template>

<style scoped>
.page {
    max-width: 560px;
    margin: 0 auto;
    padding: 64px 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
}

.title {
    margin: 0;
    font-size: 2rem;
    font-weight: 700;
    letter-spacing: -0.02em;
}

.lead {
    margin: 0;
    color: var(--c-ink-2);
    line-height: 1.7;
}

.note {
    margin: 0;
    padding: 12px 14px;
    border-radius: 8px;
    background: var(--c-raised);
    color: var(--c-ink-3);
    line-height: 1.6;
}
</style>
