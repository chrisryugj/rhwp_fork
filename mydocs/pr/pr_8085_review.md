# PR #8085 리뷰 — 마커 없는 HWPX 첫 문단 저장 왕복 회귀

## 최종 판정

**머지 보류.** 신규 HWPX 조판 입력의 독립 한컴 Print PDF와 전쪽 Native/fresh WASM
Visual Sweep가 없고, PR의 관련 이슈 `#7642`는 이 회귀와 다른 수식·글꼴 과제다.
새 head에서 입력·기준 출력과 페이지별 TSV, 대표 PNG 및 직접 판독을 제시하고
이슈 번호·참조를 실제 범위에 맞게 정정한 뒤 재검토한다. 이 기록은 GitHub 승인·댓글·push·merge가 아니다.

## 접수 정보와 경로

| 항목 | 확인값 |
| --- | --- |
| PR | [#8085](https://github.com/edwardkim/rhwp/pull/8085), 첫 기여자 `hexisteme`, 외부 fork, Open/non-draft |
| base / source | `efc52afb738bd408ac821c4e156101b8d26e5cbd` (`upstream/devel`) / `284c8fe2591260d6ec23f0cde0057324d11c9344` |
| 로컬 branch | `review/hexisteme-20261019`, source와 같은 code head에서 시작 |
| reviewer | `jangster77` 요청을 GitHub API에서 확인 |
| 검토 절차 | maintainer_general; intake_and_review, local_validation, visual_fixture_evidence, first_time_contributor, rework_and_exceptions, review_only_fast_pass |
| 작성 시점 원격 상태 | mergeable `MERGEABLE`/`CLEAN`, required `Build & Test` 성공 ([run 39262820899](https://github.com/edwardkim/rhwp/actions/runs/39262820899)); merge 전 재확인 필요 |

사용자 지시에 따라 기본 작업공간의 `devel`을 `6ed1682c3267ea786734eb2e75252916a861d1fd`에서
`efc52afb738bd408ac821c4e156101b8d26e5cbd`로 fast-forward 동기화한 뒤 최신 지침을 재확인했다.

## 변경과 원인 이력

PR은 제품 소스 없이 테스트 `tests/cases/issue_7642_unmarked_hwpx_edit_save_line_starts.rs`,
합성 HWPX 입력 및 README를 추가한다. 메모리 안의 편집 직후 문단 `line_seg_text_start`와
본문 문자열을 HWPX 재열기·이어 HWP 재열기 결과와 비교한다. `[0, 23]`을 독립 기대값으로
직접 단언하거나 렌더된 쪽의 글자·표 소속을 검사하지 않는다.

- 관련 원 이슈 [#7526](https://github.com/edwardkim/rhwp/issues/7526)은 **마커 없는 HWPX 편집 후 재저장**을 명시적으로 범위 밖에 둔다. HWP5 계보 보정은 [통합 PR #7601](https://github.com/edwardkim/rhwp/pull/7601)의 merge `e8cc27778011ce8b54e823fa7edb446b979304a6`에 반영됐다.
- 이 PR이 회귀로 지키려는 후속 parser 보정은 `ef642bb0e1b64cdd57a777fc52f038862838df67`이며, [PR #8064](https://github.com/edwardkim/rhwp/pull/8064)에서 병합됐다. 이 변경은 명시적 rhwp 생산자 마커가 있을 때만 구역 슬롯을 올린다.
- [#7642](https://github.com/edwardkim/rhwp/issues/7642)는 한컴산뜻돋움 폭·수식·페이지 배치 과제다. PR 제목, 테스트 파일·fixture 디렉터리와 본문의 `Refs #7642`는 테스트의 원인·입력과 일치하지 않는다. 관련 이슈를 #7526 후속 또는 정확한 새 이슈로 연결해야 한다.

## 로컬 검증

| 항목 | 결과 |
| --- | --- |
| `git diff --check upstream/devel...HEAD` | exit 0 |
| `git merge-tree --write-tree upstream/devel HEAD` | exit 0, tree `6eed54a705bc215242910464ada33b077387097b` |
| `node scripts/rust-test-suite-manifest.mjs --prepare` | exit 0, 신규 테스트가 `regression_suite_019`에 배정됨 |
| `node scripts/rust-test-suite-manifest.mjs --check --base-ref efc52afb738bd408ac821c4e156101b8d26e5cbd` | exit 0, 1607 sources / 7018 static test attrs / 48 targets |
| `node scripts/run-rust-test.mjs issue_7642_unmarked_hwpx_edit_save_line_starts -- --cargo-profile release-test --target-dir /home/tsjang/rhwp/target/pr-review --no-fail-fast` | exit 0, 13m 29s 컴파일 후 2 PASS / 0 FAIL / 233 filtered skip, nextest run `6124896e-ee21-4068-ad2c-5ac4f94a0383` |

기여자는 `ef642bb0e`의 parser 변경을 되돌린 진단에서 2 FAIL, 수정 head에서 2 PASS를
PR 본문에 제시했다. 이번 로컬 검증은 최종 head의 2 PASS를 독립 실행했고, 수정 전 진단은
반복하지 않았다. exact code head의 GitHub Full CI가 성공했으므로 광범위 전체 nextest와
Native Skia, lint 묶음은 [동일 head 재사용 조건](../manual/pr_review/local_validation.md#430-pr-검토의-github-full-ci-재사용)에 따라 반복하지 않았다.

검증 입력 `tests/fixtures/issue_7642_unmarked_hwpx_edit_save/wide_tac_table.hwpx`는
검토 head에 포함됐고 4,163바이트, SHA-256
`a1bc3589f6add0676766388c092608037062161b3ffe5ff00d0bf57982042ce1`이다.
원본 `samples/tac-host-spacing.hwpx`도 커밋에 포함됐고 SHA-256
`a467f038345ab5aacbe3efac39fdf5d08018bc019249af64866b55ae4b82badd`다.
ZIP 항목 7개는 같으며 `Contents/section0.xml`만 다르다. XML에서는 첫 문단 책갈피 제거,
표 폭과 셀 폭 `20000→47000`, 셀 `horzsize` `19000→46718`을 확인했다. 생산자 마커는 없다.
합성 입력의 메모리 계약과 정상 한컴 출력 계약은 구별한다.

## 조판 원칙과 시각 증거

제품 측정·배치·pagination 코드 변경은 없어 그 구현 경로의 준수 검토는 비해당이다.
다만 새 테스트는 구역 첫 문단의 저장 줄 시작, 넓은 글자처럼 취급 표와 첫 쪽의 글자 표시를
결함 설명에 사용한다. 새 HWPX fixture 및 줄 소속 회귀의 시각 선행 조건은 적용된다.

PR 본문이 독립 한컴 정본, Native/fresh WASM, 페이지별 TSV, 직접 판독을 모두 **실행하지 않았다**고
명시한다. 원본 `samples/tac-host-spacing.hwpx`의 기존 PDF는 폭을 바꾼 이번 입력의
기준으로 재사용할 수 없다. 현재 head의 같은 입력·독립 Print PDF에서 전쪽 Native/fresh WASM
최저 raw 90% 이상, `upstream/devel` 대비 각 쪽 0.1%p 이상 하락 없음, 쪽수·표/본문 소속·
누락/중복 및 대표 영역 직접 판독을 확인하기 전에는 신규 조판 회귀를 수용하지 않는다.
별도로, 모델 문자열의 동일성은 PR 본문의 “첫 쪽에서 글자가 사라졌다”는 시각 증상의
검사가 아니다. PR의 주장을 모델 왕복 보존으로만 한정하거나 실제 출력 관계 검사를 보강해야 한다.

## 보류 해제 조건

1. 이 회귀의 실제 해결 이슈·원 PR·통합 커밋을 정확히 연결하고 `#7642` 표기를 정정한다.
2. 합성 입력의 독립 한컴 Print 출력과 편집 저장본의 대응 자료를 확보해 원본·출력 SHA를 고정한다.
3. 새 head에서 전쪽 Native/fresh WASM TSV와 대표 review·overlay를 제출하고, 기준 `devel` 대비
   페이지별 차이 및 직접 판독으로 줄·표·본문의 실제 출력 보존을 확인한다.
4. 새 code head의 focused 검증과 required CI를 다시 확인한다. GitHub 게시·push·merge는
   작업지시자의 해당 조치 승인이 있을 때만 진행한다.
