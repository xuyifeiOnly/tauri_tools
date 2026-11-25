#!/bin/bash
# 统一 Tauri 打包入口：默认 release，并在真正打包前自动递增 patch 版本。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE=release
BUMP=patch
VERSION=''
DRY_RUN=false

usage() {
  cat <<'HELP'
用法：./scripts/package.sh [选项]

默认：补丁版本号 +1，构建 release .app。
  --debug             构建 debug .app
  --release           构建 release .app（默认）
  --version X.Y.Z     指定版本号（同步 package.json、Tauri、Cargo）
  --bump TYPE         major / minor / patch（默认 patch）
  --keep-version      保持当前版本号
  --dry-run           仅打印计划，不改版本、不构建
  -h, --help          显示帮助

构建失败时保留已经更新的版本号，重试请加 --keep-version。
HELP
}

fail() { printf '错误：%s\n' "$*" >&2; exit 1; }
need_value() { [[ $# -ge 2 && -n "$2" && "$2" != --* ]] || fail "$1 缺少参数"; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --debug) PROFILE=debug; shift ;;
    --release) PROFILE=release; shift ;;
    --version) need_value "$@"; VERSION="$2"; BUMP=none; shift 2 ;;
    --bump) need_value "$@"; BUMP="$2"; VERSION=''; shift 2 ;;
    --keep-version) BUMP=none; VERSION=''; shift ;;
    --dry-run) DRY_RUN=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "未知选项：$1（使用 --help 查看帮助）" ;;
  esac
done

case "$BUMP" in major|minor|patch|none) ;; *) fail '--bump 仅支持 major / minor / patch' ;; esac
for cmd in bun cargo python3; do command -v "$cmd" >/dev/null || fail "缺少依赖：$cmd"; done
[[ -x "$ROOT/node_modules/.bin/tauri" ]] || fail '缺少 Tauri CLI，请先在项目目录运行 bun install'

APP="$ROOT/src-tauri/target/$PROFILE/bundle/macos/随享.app"

# 校验所有版本后再写入；不格式化/覆盖其余配置。只接受稳定版 X.Y.Z。
python3 - "$ROOT" "$VERSION" "$BUMP" "$DRY_RUN" <<'PY'
import json, pathlib, re, sys

root, explicit, bump, dry = sys.argv[1:]
root = pathlib.Path(root)
paths = [root / 'package.json', root / 'src-tauri/tauri.conf.json', root / 'src-tauri/Cargo.toml']
texts = [p.read_text() for p in paths]
versions = [json.loads(texts[0])['version'], json.loads(texts[1])['version']]
package = re.search(r'(?ms)^\[package\]\s*\n(.*?)(?=^\[|\Z)', texts[2])
if not package:
    sys.exit('错误：Cargo.toml 缺少 [package]')
match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', package.group(1))
if not match:
    sys.exit('错误：Cargo.toml 缺少 package.version')
versions.append(match.group(1))
pattern = r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)'
if any(not re.fullmatch(pattern, v) for v in versions):
    sys.exit('错误：当前版本须为 X.Y.Z 稳定版')
if len(set(versions)) != 1 and not explicit:
    sys.exit(f'错误：版本不一致 {versions}，请使用 --version X.Y.Z 显式统一')
target = explicit or versions[0]
if not explicit and bump != 'none':
    numbers = list(map(int, target.split('.')))
    index = ['major', 'minor', 'patch'].index(bump)
    numbers[index] += 1
    numbers[index + 1:] = [0] * (2 - index)
    target = '.'.join(map(str, numbers))
if not re.fullmatch(pattern, target):
    sys.exit('错误：--version 须为 X.Y.Z 稳定版，例如 0.1.1')

updated = []
for text, version in zip(texts[:2], versions[:2]):
    # 只替换顶层 version 的唯一匹配，避免误改依赖版本。
    regex = r'("version"\s*:\s*")' + re.escape(version) + r'(")'
    if len(re.findall(regex, text)) != 1:
        sys.exit('错误：JSON version 匹配不唯一，未修改文件')
    updated.append(re.sub(regex, lambda m: m[1] + target + m[2], text, count=1))
start = package.start(1) + match.start(1)
end = package.start(1) + match.end(1)
updated.append(texts[2][:start] + target + texts[2][end:])
print(f'版本：{versions} → {target}' + ('（预览，不写入）' if dry == 'true' else ''))
if dry != 'true':
    for path, old, new in zip(paths, texts, updated):
        if old != new:
            path.write_text(new)
PY

printf '构建模式：%s\n构建产物：%s\n' "$PROFILE" "$APP"
if [[ "$DRY_RUN" == true ]]; then exit 0; fi

cd "$ROOT"
# 不使用 package.json 的 tauri 别名，避免 tauri build build。
# Cargo 在构建时同步 Cargo.lock 的本项目版本。
ARGS=(build --bundles app)
if [[ "$PROFILE" == debug ]]; then ARGS+=(--debug); fi
unset CARGO_TARGET_DIR CARGO_BUILD_TARGET
"$ROOT/node_modules/.bin/tauri" "${ARGS[@]}"
[[ -f "$APP/Contents/Info.plist" && -x "$APP/Contents/MacOS/mac-tunnel" ]] || fail "未找到有效构建产物：$APP"
BUILT_VERSION=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")
EXPECTED_VERSION=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$ROOT/src-tauri/tauri.conf.json")
[[ "$BUILT_VERSION" == "$EXPECTED_VERSION" ]] || fail "产物版本不匹配：$BUILT_VERSION != $EXPECTED_VERSION"
printf '打包完成：%s（版本 %s）\n' "$APP" "$BUILT_VERSION"
