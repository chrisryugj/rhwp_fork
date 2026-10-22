# Issue #8151 편람 HWPX 144쪽 정합성 개선 계획

- 이슈: https://github.com/edwardkim/rhwp/issues/8151
- 브랜치: `fix/handbook-hwpx-p144-20261022`
- 고정 base: `2e087a5b4bc1515ad877e8020b30114639a59392` (시작 시 최신 `upstream/devel`과 동일)
- 입력: `samples/2025 행정업무운영 편람(최종).hwpx`
- 독립 기준: `pdf/2025 행정업무운영 편람(최종)-hwpx-kopub-2024.pdf` (383쪽)

## 기존 해결과 현재 증거

#3930의 저장 후 쪽 소속·희소 바탕쪽 보존은 PR #4058에서 통합됐다.
도장 상자 가로 배치는 #3820의 `ba39fac28f0b28512edf3de358bd37d3d5538408`,
PR #4517에 포함된 보정이다. 해당 동작은 유지한다.
제출 TSV에서 144쪽 Native raw 97.88889%, fresh WASM raw 97.85537%다.
이 점수는 도장 좌표 자체의 오류 증거가 아니며 페이지 전체의 원인 조사 출발점이다.
기존 절대 x ±0.3px 검사는 기본 compat이고, TSV는 compat2024여서 조건을 맞춘다.

## 실행 단계

1. 동일 원본·독립 Print·96dpi·print·compat2024의 base Native/fresh WASM
   144쪽 review·overlay를 산출하고 실제 차이와 독립 기대값을 확인한다.
2. 원인 값의 생산 → 측정 소비 → 배치 소비 → 최종 paint까지 호출 경로를 추적하고
   공통 로직에서 보정한다. 문서별 좌표 보정이나 지표·마스크·DPI 변경은 하지 않는다.
3. 영향 쪽에서 Native/fresh WASM raw 각각 98% 이상과 확대 판독을 확인한다.
4. 같은 문서 전 383쪽의 base/head Native/fresh WASM TSV와 쪽수·내용 보존을 대조한다.
   관련 정상 경로의 영향도 검사한다.
5. 검증 후 독립 Print 좌표 ±2px 및 소유·내용·순서·포함·비겹침 관계를 보호하도록
   회귀 검사를 보완한다. 수정 전 FAIL / 수정 후 PASS를 확인하고 관련 회귀와 필수 lint를 실행한다.

## 산출물

작업 로그·중간 PNG/JSON/TSV와 보존용 producer는 ignored
`output/pr-review/handbook-hwpx-p144-20261022/`에 생성한다.
검증을 마친 TSV·대표 PNG·독립 좌표 근거는 `mydocs/pr/assets/issue_8151/`에 보존한다.
현재 단계는 조사·기준 출력 준비이며 구현 및 개선 완료를 주장하지 않는다.
원격 push·PR 제출은 이번 요청에 포함하지 않는다.
