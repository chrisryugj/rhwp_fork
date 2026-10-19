# PR #8085 리뷰 — 마커 없는 HWPX 첫 문단 저장 왕복 회귀

## 최종 판정

**머지 보류.** 이번 검토에서 합성 HWPX 원본과 편집·저장본의 독립 한컴 Print PDF를
확보하고 전쪽 Native/fresh WASM Visual Sweep를 직접 실행했다. 두 문서 모두 90% gate와
`devel` 대비 무회귀를 충족하며, 첫 글줄·표·뒤 문단의 소속과 순서를 직접 확인했다.
다만 HWP 저장본은 한컴의 손상·변조 가능성 경고로 열기가 거부돼 Print 출력 검증이
불가능했고, PR의 `#7642` 참조도 이 회귀와 다른 수식·글꼴 과제를 가리킨다.
기여자의 본문에는 이번 독립 PDF·전쪽 TSV·대표 PNG도 아직 반영되지 않았다.
이 기록은 GitHub 승인·댓글·push·merge가 아니다.

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

PR 본문은 독립 한컴 정본, Native/fresh WASM, 페이지별 TSV, 직접 판독을 **실행하지 않았다**고
명시했다. 원본 `samples/tac-host-spacing.hwpx`의 기존 PDF는 폭을 바꾼 이번 입력의
기준으로 재사용하지 않았다. 아래 검토자가 직접 실행한 시각 증거는 HWPX 경로의
누락·중복 및 표/본문 소속에 한정한다. HWP 저장 경로의 한컴 Print 결과는 확보하지 못했다.
모델 문자열의 동일성만으로 PR 본문의 “첫 쪽에서 글자가 사라졌다”는 시각 증상을
검사할 수 없다는 기존 코드 검토 의견은 유지한다.

### 2026-10-20 독립 Print와 Visual Sweep

`rhwp info --json`은 첨부 HWPX의 저장 제품을 `hancom-office-2020`으로 판정했다.
동일 원본에 CLI `edit insert-text --section 0 --para 0 --offset 0 --text MARKERX
--verify`를 적용해 HWPX 편집본을 만들었다. 이는 PR 테스트의 `insert_text_native`와
`export_hwpx_native` 경로이며 저장 후 IR 검증 `diffCount=0`이다. 입력 SHA-256은
원본 `a1bc3589f6add0676766388c092608037062161b3ffe5ff00d0bf57982042ce1`,
편집 HWPX `9ecc6cbe7696d77fa1e9b511c8f390c790794c60b64e9d4de70b2f03bc7031f6`이다.

HWP 2024 MCP client의 `start → status → download`에 `--engine 2020`, `--target pdf`,
출력 파일명 `-2020.pdf`를 명시했다. 두 job의 실제 한컴 버전은 `11.0.0.9136`,
`pdf_print_method=0`, `pdf_output_mode=hancom2020_pdf_driver_one_up`이며 글꼴 등록 실패는 0이다.
용지 재지정 없이 원본 A4, 전체 1쪽을 인쇄했다.

| 입력 / MCP job | PDF 위치 / SHA-256 | 전체 쪽수 |
| --- | --- | --- |
| 원본 HWPX / `00d32c70-1f43-4937-8898-1130f3a12383` | `/home/tsjang/rhwp/pdf/pr_8085/wide_tac_table-2020.pdf` / `38fd72fcf9f44bb9dbc10e1979f64819c8ee17652c52c4d8e72b9b3f4fb8b5fd` | 1 |
| 편집 HWPX / `7ef70fe5-a7ee-45c8-9f0a-ff0bc25659cb` | `/home/tsjang/rhwp/pdf/pr_8085/wide_tac_table_edited-2020.pdf` / `9650e6572d996c7d0949c0734a0378d8b964423481d7c25428232dbda9450fca` | 1 |

source PR head `284c8fe2591260d6ec23f0cde0057324d11c9344`와 기준 `devel`
`efc52afb738bd408ac821c4e156101b8d26e5cbd`의 차이는 신규 테스트·fixture·README뿐이다.
제품 소스와 Sweep 스크립트는 동일하다. 로컬 검토 문서 커밋 `77b87d0239b7179409280b7a6999f7559e66dce0`
worktree에서 `native-skia` CLI (`d288f9d9972e40595560c303339452aa9682bcfcb58faa5092dcafe7033f18ef`)
및 최적화 완료한 fresh WASM (`rhwp_bg.wasm` SHA-256
`3f39df9499a764c134d0874f5f163b0a41880da332c948a5d61a5a4ad9f52006`)을 빌드했다.
`--compat 2022 --dpi 96`, Print 프로필, 동일 PDFium으로 `--silhouette-only` 전쪽을
`output/pr-review/8085/head/{native,wasm}`과 기본 작업공간의
`output/pr-review/8085/base/{native,wasm}`에 각각 산출했다. 대표 1쪽은 별도
`output/pr-review/8085/head/{native_review,wasm_review}` 일반 모드에서
`--pages 1`로 compare·overlay·review PNG를 생성했다. 명령의 입력은 위 표의
원본·편집 HWPX와 대응 PDF이며 Native에는 `--rhwp-bin /home/tsjang/rhwp/target/pr-review/debug/rhwp`,
WASM에는 `--wasm-pkg /home/tsjang/rhwp-pr8085-review/pkg`를 추가했다.
기준과 후보의 제품 소스가 같아 같은 검증용 CLI·WASM package를 사용하되,
기준 작업공간과 후보 worktree에서 각각 재출력했다. 예를 들어 후보 전쪽 TSV 명령은 다음과 같다.

```bash
python3 scripts/visual_sweep.py --silhouette-only \
  --file-target raw tests/fixtures/issue_7642_unmarked_hwpx_edit_save/wide_tac_table.hwpx /home/tsjang/rhwp/pdf/pr_8085/wide_tac_table-2020.pdf \
  --file-target edited output/pr-review/8085/inputs/wide_tac_table_edited.hwpx /home/tsjang/rhwp/pdf/pr_8085/wide_tac_table_edited-2020.pdf \
  --rhwp-bin /home/tsjang/rhwp/target/pr-review/debug/rhwp --compat 2022 --dpi 96 \
  --out output/pr-review/8085/head/native
```

WASM은 같은 명령에서 `--wasm-pkg /home/tsjang/rhwp-pr8085-review/pkg`와
`--out output/pr-review/8085/head/wasm`을 사용한다. 기준 run은 기본 작업공간
`/home/tsjang/rhwp`에서 동일 입력을 절대 경로로 지정하고 `base/{native,wasm}`에 저장했다.
대표 PNG는 해당 명령의 `--silhouette-only`를 빼고 `--pages 1`을 더해
`head/{native_review,wasm_review}`에 별도로 생성했다.
PDF와 입력·TSV·PNG·manifest의 로컬 보존 사본은
`/home/tsjang/rhwp/pdf/pr_8085/evidence/{review_worktree,devel_base}`에 있으며
`pdf/pr_8085/`는 `.git/info/exclude`에 등록해 검토 산출물이 PR에 섞이지 않게 했다.

| 전체 1쪽 입력 | Native raw / TSV SHA-256 | fresh WASM raw / TSV SHA-256 | devel 대비 |
| --- | --- | --- | --- |
| 원본 HWPX | `99.31338%` / `a56452c13efaab6f25a39df8b9381abebf214c3b40246c3903ad816224adbd67` | `99.34138%` / `8fb4b0a9d95e67ca1b7cf40bf0c443ba34a8a1b31641d8ea5e6627d0f727602e` | 양쪽 `0.00000%p` |
| 편집 HWPX | `99.22414%` / `cfba5f3974b425e171a7211bc00c6b4a29c97ecf1e3415d0d327d14f6e11f931` | `99.33946%` / `44a81e84c56ddada080a391b159168c019083114cf6ddf718a0d4e047a71de35` | 양쪽 `0.00000%p` |

각 TSV의 `below_90=0`, 누락 쪽 0, Native·WASM·PDF 전체 쪽수는 모두 1이다.
기준·후보의 대응 PNG SHA-256도 바이트 단위로 일치한다. `silhouette-only`의
`not_evaluated`는 직접 판독 전 상태이고, 대표 PNG를 생성한 일반 모드의 gate는
원본·편집본 Native/WASM 모두 `passed`였다. 대표 편집본의
`native_review/edited/review/review_001.png`와
`wasm_review/edited/review/review_001.png`, 각 `overlay/overlay_001.png`를 직접 열어
한컴 Print PDF와 비교했다. `MARKERX`가 첫 글줄에 있고 다음 줄의 `CELL`은 표 안,
`NEXT PARAGRAPH`는 표 뒤에 있으며 누락·중복·빈 페이지는 보이지 않는다.
글자를 2배 확대해 확인한 x 시작 위치는 한컴과 두 rhwp backend 간 최대 2px 차이였다.
표 테두리는 rhwp 쪽이 한컴 PDF보다 옅게 보이는 잔여 색상·안티앨리어싱 차이가 있으나
외곽 위치와 뒤 문단 순서는 일치했다. 이 fixture에는 수식이 없다.

별도로 편집 HWPX를 `rhwp convert --verify`로 HWP 저장 후 재열어 IR `diffCount=0`을
확인했다(입력 SHA-256 `c30f92802758cf16c339b32f8195821b8e5cf5998fa64706733a9d291d242d49`).
그러나 이 HWP를 동일 engine 2020으로 Print 요청한 job
`41fa7a89-5aa6-41b3-88ee-f822514bb0c9`은 `document_open_rejected`,
`HANCOM_DOCUMENT_OPEN_REJECTED`로 끝났다. 한컴이 손상·변조 가능성을 경고해
무인 열기를 중단한 것이므로 PDF·Visual Sweep 결과는 없다. 지침에 따라 보안 경고를
자동 승인하거나 engine/timeout을 바꿔 재시도하지 않았다. HWP 저장 경로의 실제
한컴 출력 적합성은 **미검증**이다.

## 보류 해제 조건

1. 이 회귀의 실제 해결 이슈·원 PR·통합 커밋을 정확히 연결하고 `#7642` 표기를 정정한다.
2. 기여자는 HWPX 원본·편집본의 독립 Print PDF, 전쪽 Native/fresh WASM TSV 및 대표
   review·overlay를 정확한 PR head에 연결해 제출한다. 위 로컬 검토 증적을 참고할 수 있다.
3. HWP 저장 경로를 주장하려면 보안 경고의 원인을 조사하고 안전하게 열린 HWP의 독립
   Print 출력과 Native/fresh WASM 전쪽 비교를 제시하거나, PR의 해결 범위를 모델 왕복
   보존으로 정확히 한정한다.
4. 새 code head의 focused 검증과 required CI를 다시 확인한다. GitHub 게시·push·merge는
   작업지시자의 해당 조치 승인이 있을 때만 진행한다.
