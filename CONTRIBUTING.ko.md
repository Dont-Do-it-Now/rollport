# 기여하기

[English](CONTRIBUTING.md) | [한국어](CONTRIBUTING.ko.md)

[Tauri 2](https://tauri.app)와 SvelteKit을 사용합니다. iPhone 측에서는
[`idevice`](https://crates.io/crates/idevice) 크레이트를 사용합니다.

## 필요한 도구

- [Rust](https://rustup.rs) 1.90 이상
- [Node.js](https://nodejs.org) 22.18 이상(스크립트에서 Node의 기본 TypeScript 지원 사용)
- [pnpm](https://pnpm.io/installation) 11
- Tauri의 [사전 요구 사항](https://v2.tauri.app/start/prerequisites/): Mac에서는
  Xcode Command Line Tools, Windows에서는 Microsoft C++ Build Tools와 WebView2

## 명령어

```sh
pnpm install
pnpm tauri dev      # 실행
pnpm tauri build    # 빌드
pnpm screens        # 창의 모든 상태를 촬영하여 screens/index.html에 저장
pnpm clip           # README용 밝은 테마와 어두운 테마 클립을 docs/에 저장
                    # (brew install webp 필요)
```

## Mac에서 Windows용 교차 컴파일

cargo-xwin과 LLVM이 필요합니다.

```sh
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin
brew install llvm makensis

PATH="/opt/homebrew/opt/llvm/bin:$PATH" \
  pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --no-bundle
```

`--no-bundle` 대신 `--bundles nsis`를 사용하면 설치 프로그램을 빌드합니다.

## 버그 신고

[이슈를 열 때](../../issues/new) 운영체제, iPhone 모델, iOS 버전과 오류 메시지가
있다면 함께 작성해 주세요.
