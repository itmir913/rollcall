# GLOBAL RULES — 출결관리 (rollcall)

School-Record-App에서 배운 것을 그대로 가져온 규칙이다. 근거가 적힌 항목은
그 앱에서 실제로 넘어졌던 지점이다.

## ARCHITECTURE
- Component MUST NOT call `invoke()`. ALWAYS use Store.
- Rust handles ALL DB and core logic. Frontend = UI + state only.
- Store = single source of truth. No duplicated logic.
- 순수 로직(슬롯 순서, 문구 생성, 마감일 계산)은 DB를 모르는 모듈(`slots.rs`,
  `phrase.rs`, `due.rs`)에 둔다. 커맨드에 인라인하면 테스트할 수 없다.

## COMMAND LAYER
- 모든 Tauri 커맨드는 **얇은 래퍼**다. 실제 로직은 `*_impl(conn: &Connection, ...)`.
  `State<DbState>`를 받는 함수는 테스트에서 호출할 수 없기 때문이다.
- Rust 커맨드 이름: snake_case. 프론트 인자: camelCase (Tauri가 변환).
- MUST handle errors explicitly. Silent failures 금지.
- Store의 액션은 에러를 `error`에 담고 **다시 던진다.** 삼키면 읽기 실패가
  "데이터 없음"과 구분되지 않는 빈 화면으로 보인다.

## TRANSACTION
- 트랜잭션은 반드시 `db::with_transaction`으로만 연다. Connection은 Mutex로 공유되는
  하나뿐이라, 트랜잭션을 연 채 함수를 빠져나가면 세션 내내 그 상태로 남는다.
  이후 모든 BEGIN이 실패하고, 트랜잭션 없는 쓰기가 열린 트랜잭션에 묶여
  앱을 닫을 때 통째로 롤백된다.
- 조기 반환이 필요하면 **클로저 안에서** `?`를 쓴다. 클로저 밖의 `?`는 ROLLBACK을 건너뛴다.

## DB SCHEMA RULES
사용자 PC에 기존 구조의 DB가 남는다. **`schema.sql`을 고치면 아래를 모두 한다.**
1. `db.rs`의 `SCHEMA_VERSION`을 올린다.
2. `db.rs`의 `MIGRATIONS`에 이전 → 새 버전 SQL을 추가한다.
3. 수정한 `schema.sql`을 `src-tauri/src/tests/schema_history/vN.sql`로 복사한다.
4. `schema_lock_tests.rs`의 `SCHEMA_BASELINES`에 항목을 추가한다.
5. `tauri.conf.json`의 앱 버전도 올린다.

- **`schema_history/vN.sql`은 절대 수정 금지.** 배포된 DB 구조의 기록이다.
- 테스트를 맞추려고 지문만 고치는 것 금지.

## DOMAIN RULES (이 앱의 설계 결정)
- **프로그램은 판정하지 않는다.** 교사가 입력한 것을 그대로 저장한다.
  미인정 우선 같은 규칙을 자동 적용하지 않는다.
- **슬롯을 펼치지 않는다.** 열린 구간은 NULL(`*`)로 둔다. 학사일정·시간표 테이블을
  만들지 않는다. (근거: 나이스 실데이터에서 그날 슬롯 수가 1·3·6·7교시로 제각각이었다.
  시간표로 펼치는 설계였다면 단축수업·시험일에 전부 틀렸다.)
- **코드는 데이터다.** 출결 구분·체크 항목·서류 규정을 하드코딩하지 않는다.
  `absence_span.code_id`는 FK다. 문자열이나 enum으로 저장하면 설정 기능에서 전면 재작업이다.
- **수정은 마감 후 추가다.** 코드나 체크 항목을 UPDATE로 고치면 과거 기록이 소급
  변경된다. `valid_to`를 채워 마감하고 새 행을 추가한다.
- **구간(`absence_span`)과 사유(`daily_reason`)는 별도 테이블이다.** 한 테이블이면
  하루 2구간에서 막힌다.
- **`daily_check`는 행 구조다.** 항목마다 컬럼을 두면 추가할 때마다 마이그레이션이다.
- 날짜는 ISO(`2026-07-15`)로 저장하고 화면·내보내기에서만 `2026.07.15.(수)`로 포맷한다.
- 재가져오기는 교체가 아니라 **차분**이다. 사라진 번호는 삭제하지 않고
  `enrolled_to`를 채운다(전출).

## UX 기준선
> **마우스로 조작한다. 출결 한 건이 클릭 몇 번으로 끝나야 한다.**

- 후보가 정해져 있고 늘어나도 십여 개인 값(출결 코드, 교시)은 **버튼으로 펼친다.**
  드롭다운은 열기 전까지 후보가 보이지 않아 클릭이 한 번 더 든다.
- 타이핑은 증상·사유 한 칸으로 제한한다. 그마저도 과거에 쓴 단어를 후보 버튼으로 띄운다.
- 확인 대화상자를 넣지 않는다. 편집기의 [저장]이 곧 저장이다.

**키보드 단축 입력은 배포 직전에 따로 정한다.** 지금은 없다. 그때까지 키보드 전용
경로를 전제한 설계를 넣지 않는다 — 마우스 경로가 그 보조로 밀리면 안 된다.

- Font size: `text-base` minimum. `text-sm` / `text-xs`는 예외(표 셀 미리보기, 배지,
  캡션)에만, 이유를 명시하고 쓴다.

## UI 규칙

### 화면 흐름
```
Welcome  →  초기 설정  →  HOME
```
HOME이 일일 입력 격자이고 라우트는 `/`다. 매일 열자마자 바로 입력할 수 있어야 하므로
Welcome과 설정은 첫 실행에서만 지나간다. 두 화면은 `meta.bare`가 붙어 머리글 없이
전체 화면으로 뜬다.

### 컴포넌트
- 색·여백·모서리를 화면에 직접 적지 않는다. `src/components/ui/`의 프리미티브
  (`UiButton` `UiCard` `UiTable` `UiToggle` `UiNotice` `UiPage`)와 `style.css`의
  토큰만 쓴다. **시각 디자인은 아직 정해지지 않았다.** 나중에 정할 때 이 두 곳만
  고치면 전 화면이 함께 바뀌도록 하려는 것이다.
- 같은 모양이 두 화면에 나오면 프리미티브로 올린다.

### 체크박스를 쓰지 않는다
불리언 입력은 예외 없이 `UiToggle`이다. 체크박스는 클릭 표적이 작아 30행짜리
격자에서 옆 칸을 누르기 쉽고, 켜짐/꺼짐이 색과 글자로 같이 보여야 훑어볼 때
한눈에 들어온다.

### 테마
- **기본은 라이트다.** 다크는 `html[data-theme="dark"]`로만 켜진다. CSS에
  `prefers-color-scheme` 미디어 쿼리를 쓰지 않는다 — 상태가 `data-theme` 한 곳에만
  있어야 토글과 시스템 설정이 어긋나지 않는다. "시스템 따름"을 고른 경우에만
  부트 스크립트가 그 결과를 `data-theme`에 찍는다.
- 테마 값은 DB가 아니라 `localStorage`에 둔다. 첫 페인트 전에 읽어야 하는데 DB 조회는
  비동기라 흰 화면이 한 번 번쩍인다. 화면 취향은 출결 데이터가 아니므로 백업에도
  들어가지 않는다.

## 명령어는 셋뿐이다

```
npm run dev     # 앱 실행
npm run build   # 설치본 빌드
npm test        # Rust + 프론트엔드 전체 테스트
```

나머지는 전부 이 셋의 조립이다. `vite`, `cargo`, `vitest`를 별도 스크립트로
package.json에 추가하지 않는다. 늘어나면 어느 것이 진짜 진입점인지 알 수 없게 된다.

- `tauri.conf.json`의 `beforeDevCommand`/`beforeBuildCommand`는 `vite`를 **직접** 부른다.
  `npm run dev`를 부르면 `dev`가 `tauri dev`이므로 무한 재귀한다.
- IntelliJ 실행 구성 `.idea/runConfigurations/{dev,build,test}.xml`이 같은 셋을 가리킨다.
  `.gitignore`는 `.idea` 디렉터리째 무시하면 안 된다 — git이 그 안으로 내려가지 않아
  negation이 먹지 않는다. `/.idea/*`로 항목을 무시해야 한다.

## GIT / COMMIT RULES
- **GPG 서명 필수**: 모든 커밋에 서명한다. `commit.gpgsign = true`가 전역에 설정돼 있어
  `git commit`이 자동으로 서명하지만, 명시하려면 `git commit -S`.
- **Co-Authored-By / Co-Worked 문구 삽입 금지**: 커밋 메시지에 Claude 관련 문구를
  일절 포함하지 않는다.
- 커밋 메시지: 한국어 또는 영어, 간결하게. `feat:` `fix:` `docs:` 같은 접두사를 쓴다.
- 기본 브랜치는 `master`.

### PR 머지는 로컬에서 (웹 UI 머지 금지)
GitHub 웹 UI나 `gh pr merge`로 머지하면 GitHub이 자기 키(web-flow)로 서명한다.
로컬에 그 공개키가 없으면 `git log --format=%G?`에서 `E`(검증 불가)로 뜨고,
"모든 커밋에 서명" 규칙이 히스토리상 깨진다.

```
git fetch origin
git checkout master && git pull --ff-only
git merge --no-ff origin/<PR 브랜치> -m "Merge pull request #N from <브랜치>"
git log --format="%h %G? %s" -1     # G인지 확인
git push origin master
```

- `--no-ff`로 머지 커밋을 남긴다. PR head가 조상이 되므로 GitHub이 PR을 자동으로
  Merged 처리한다.
- **`merge.verifySignatures`는 켜지 말 것.** dependabot 커밋은 GitHub 키로 서명돼
  있는데 로컬에 그 공개키가 없으면 켜는 순간 머지가 전부 거부된다.

## PROHIBITED
- Silent failures
- Business logic in frontend
- 프론트엔드로 암호화 키를 넘기는 것
- 컴포넌트에 색값 직접 적기 (토큰만 쓴다)
- `<input type="checkbox">` (UiToggle을 쓴다)
- package.json에 dev/build/test 외의 스크립트 추가

## 문서에 낡는 숫자를 적지 않는다
주석·README·매뉴얼에 **세면 바뀌는 수**를 적지 말 것. 테스트 개수, 파일 개수,
커맨드 개수는 다음 커밋에 바로 틀린 값이 되고 아무도 고치지 않아 문서 전체의
신뢰를 깎는다.

- ❌ `// 엣지 케이스 (32개)` → ✅ `// 엣지 케이스`
- ❌ `테스트 161개 전부 통과` → ✅ `전체 통과`

**예외:** 날짜가 붙은 이력, 고정 상수(`SCHEMA_VERSION`, PBKDF2 반복 수 등),
UI 목업 속 예시 숫자.

판단 기준: **"코드가 바뀌면 이 문장이 틀려지는가?"**
