# RHWP Bindings

This directory separates the shared native ABI from language-specific bindings.

- `Native/`: Rust `cdylib` crate that exposes the C ABI used by bindings.
- `csharp/`: C# P/Invoke wrapper over the shared native library.
- `swift/`: Swift Package wrapper over the shared native library.

New language bindings are downstream by default. Adding one here requires explicit maintainer
adoption, a stable compatibility contract, and a tested long-term release path.

## Direct PDF (P50)

`rhwp_export_pdf_direct(input_path, output_file, page)`는 기존 direct Skia PDF
renderer를 **print** profile로 호출한다. `page=-1`은 전체 쪽, 그 외에는 0부터 시작하는
쪽 번호다. 입력은 최대 **128 MiB**, 한 번에 선택하는 쪽은 최대 **256쪽**이다.
모든 선택 쪽의 검증과 PDF 생성을 마친 뒤 파일을 만들며, 부모 디렉터리는 이미 존재해야 하고
출력 파일은 새 경로여야 한다. 기존 파일·입력 파일·symlink·hard link를 덮어쓰지 않는다.
지원하지 않는 입력은 오류를 반환하고 compatibility SVG PDF로 우회하지 않는다.
성공 JSON의 `pageCount`는 원본 전체 쪽수, `files`는 생성한 PDF 한 파일이다.
반환한 문자열은 다른 C ABI와 동일하게 `rhwp_string_free`로 해제한다.

기본 빌드도 이 심볼을 내보낸다. null·UTF-8·쪽 번호 인자 검사를 통과하면 파일 I/O 없이
`{"ok":false,"code":"featureDisabled","error":"Direct PDF export requires the native-skia feature."}`를
반환한다. 기능을 사용하려면 공유 target 경로를 지정해 빌드한다.

```sh
CARGO_TARGET_DIR="$PWD/target/pr-review" cargo build --locked -p rhwp-native-ffi --features native-skia
```

C#은 `RhwpNative.ExportPdfDirect(inputPath, outputFile, page)`, Swift는
`Rhwp.exportPDFDirect(inputFile:outputFile:page:)`를 제공한다. C#은 JSON을 그대로 반환하고,
Swift는 기존 `RhwpExportResult`를 반환하거나 `RhwpError.exportFailed`를 던진다.
폰트는 기존 native renderer의 기본 font resolution을 사용한다. 이번 ABI는 별도 font path,
profile 또는 compatibility backend 선택 옵션을 노출하지 않는다.

Linux에서 native ABI를 검증해도 Apple XCFramework build/link가 검증된 것은 아니다.
Apple 산출물과 독립 한컴 PDF 출력 대조는 별도 검증 대상이다.
