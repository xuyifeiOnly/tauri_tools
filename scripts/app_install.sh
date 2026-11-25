#!/bin/bash
# 本机测试用构建和安装；安装前退出客户端，不修改应用数据或手工隧道。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE=release
BUMP=patch
VERSION=''
INSTALL=true
LAUNCH=false
DRY_RUN=false
INSTALL_DIR="$HOME/Applications"

usage() {
  cat <<'HELP'
用法：./scripts/app_install.sh [选项]

默认：补丁版本号 +1，构建 release .app，安装到 ~/Applications；不自动启动。
  --version X.Y.Z     指定版本号（同步 package.json、Tauri、Cargo）
  --bump TYPE         major / minor / patch（默认 patch）
  --keep-version      保持当前版本号
  --debug             构建 debug 而非 release（仍不代表签名/公证）
  --release           构建 release（默认）
  --no-install        只修改版本并打包
  --install-dir DIR   安装到指定的绝对目录，不使用 sudo
  --open              安装成功后启动；不能和 --no-install 同用
  --dry-run           仅打印计划，不改版本、不构建、不安装
  -h, --help          显示帮助

打包成功后自动请求旧客户端退出，等待最多15秒；退出失败则中止安装。
退出客户端会中断它管理的共享服务，不影响独立的手工隧道。
直接覆盖安装，不保留旧版或压缩备份；应用数据、密钥不受影响。
构建失败时保留已经更新的版本号，重试请加 --keep-version。
HELP
}

fail() { printf '错误：%s\n' "$*" >&2; exit 1; }
need_value() { [[ $# -ge 2 && -n "$2" && "$2" != --* ]] || fail "$1 缺少参数"; }
while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) need_value "$@"; VERSION="$2"; BUMP=none; shift 2 ;;
    --bump) need_value "$@"; BUMP="$2"; VERSION=''; shift 2 ;;
    --keep-version) BUMP=none; VERSION=''; shift ;;
    --debug) PROFILE=debug; shift ;;
    --release) PROFILE=release; shift ;;
    --no-install) INSTALL=false; shift ;;
    --install-dir) need_value "$@"; INSTALL_DIR="$2"; shift 2 ;;
    --open) LAUNCH=true; shift ;;
    --dry-run) DRY_RUN=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "未知选项：$1（使用 --help 查看帮助）" ;;
  esac
done
case "$BUMP" in major|minor|patch|none) ;; *) fail '--bump 仅支持 major / minor / patch' ;; esac
[[ "$INSTALL_DIR" == /* ]] || fail '--install-dir 必须是绝对路径'
[[ "$LAUNCH" == false || "$INSTALL" == true ]] || fail '--open 不能与 --no-install 同用'
[[ "$(uname -s)" == Darwin ]] || fail '此脚本仅用于 macOS'
for cmd in bun cargo python3; do command -v "$cmd" >/dev/null || fail "缺少依赖：$cmd"; done
[[ -x "$ROOT/node_modules/.bin/tauri" ]] || fail '缺少 Tauri CLI，请先在项目目录运行 bun install'

DEST="$INSTALL_DIR/随享.app"
APP="$ROOT/src-tauri/target/$PROFILE/bundle/macos/随享.app"
MANAGED_ROOTS=("$INSTALL_DIR" "$HOME/Applications" /Applications
  "$ROOT/src-tauri/target/debug/bundle/macos"
  "$ROOT/src-tauri/target/release/bundle/macos")
check_not_running() {
  local status=0
  pgrep -x mac-tunnel >/dev/null 2>&1 || status=$?
  [[ "$status" -eq 1 ]] || fail '随享仍在运行或进程检查失败；已中止覆盖，请从菜单栏“停止服务并退出”后重试'
}
stop_app() {
  # 不使用 killall/pkill，也不启动尚未运行的应用。
  python3 - <<'PY'
import subprocess, sys, time
def running():
    result = subprocess.run(['pgrep', '-x', 'mac-tunnel'], capture_output=True)
    if result.returncode not in (0, 1):
        sys.exit('错误：无法检查客户端进程，停止安装')
    return result.returncode == 0
if not running():
    sys.exit(0)
print('正在退出旧版随享（Mac Tunnel）；客户端托管的共享服务将中断……', flush=True)
try:
    result = subprocess.run(
        ['/usr/bin/osascript', '-e', 'tell application id "cn.rutang.mac-tunnel" to quit'],
        capture_output=True, text=True, timeout=5)
except subprocess.TimeoutExpired:
    sys.exit('错误：请求退出超时，未覆盖安装，请手动停止服务并退出')
if result.returncode != 0:
    sys.exit('错误：请求退出失败，未覆盖安装：' + result.stderr.strip())
deadline = time.monotonic() + 10
while running():
    if time.monotonic() >= deadline:
        sys.exit('错误：客户端未在15秒内退出，未覆盖安装；请从菜单栏停止服务并退出')
    time.sleep(0.2)
print('旧客户端已退出。')
PY
}
if [[ "$INSTALL" == true ]]; then
  [[ ! -L "$DEST" ]] || fail "安装目标是符号链接，请手动处理：$DEST"
  if [[ -e "$DEST" ]]; then
    [[ -d "$DEST" ]] || fail "安装目标不是应用目录：$DEST"
    IDENTIFIER=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$DEST/Contents/Info.plist")
    [[ "$IDENTIFIER" == cn.rutang.mac-tunnel ]] || fail "安装目标不是随享，拒绝覆盖：$DEST"
  fi
  python3 "$ROOT/scripts/app_bundles.py" preflight "$DEST" "${MANAGED_ROOTS[@]}"
fi

# 版本同步与构建统一交给 scripts/package.sh，避免安装与直接打包重复递增版本。
PACKAGE_ARGS=(--"$PROFILE")
if [[ -n "$VERSION" ]]; then
  PACKAGE_ARGS+=(--version "$VERSION")
elif [[ "$BUMP" == none ]]; then
  PACKAGE_ARGS+=(--keep-version)
else
  PACKAGE_ARGS+=(--bump "$BUMP")
fi
if [[ "$DRY_RUN" == true ]]; then PACKAGE_ARGS+=(--dry-run); fi
if [[ "$INSTALL" == true ]]; then printf '安装目标：%s\n' "$DEST"; fi
"$ROOT/scripts/package.sh" "${PACKAGE_ARGS[@]}"
if [[ "$DRY_RUN" == true ]]; then exit 0; fi

BUILT_VERSION=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")
if [[ "$INSTALL" == false ]]; then printf '打包完成：%s（版本 %s）\n' "$APP" "$BUILT_VERSION"; exit 0; fi

# 完整复制到暂存目录后再替换，避免把半成品装到目标位置。
mkdir -p "$INSTALL_DIR"
STAGE=$(mktemp -d "$INSTALL_DIR/.mac-tunnel-install.XXXXXX")
printf '暂存目录：%s\n' "$STAGE"
/usr/bin/ditto "$APP" "$STAGE/next.bundle"
stop_app
check_not_running
BACKUP=''
if [[ -e "$DEST" || -L "$DEST" ]]; then
  [[ ! -L "$DEST" && -d "$DEST" ]] || fail '安装目标在构建期间发生变化，拒绝覆盖'
  IDENTIFIER=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$DEST/Contents/Info.plist")
  [[ "$IDENTIFIER" == cn.rutang.mac-tunnel ]] || fail '安装目标标识在构建期间发生变化，拒绝覆盖'
  # 仅在替换过程中保留临时旧包；成功后删除，失败则恢复。
  BACKUP="$STAGE/previous.bundle"
  mv "$DEST" "$BACKUP"
fi
if ! mv "$STAGE/next.bundle" "$DEST"; then
  if [[ -n "$BACKUP" && ! -e "$DEST" ]]; then mv "$BACKUP" "$DEST"; fi
  fail "安装失败，暂存文件保留于：$STAGE"
fi
if [[ -n "$BACKUP" ]]; then
  # 路径只可能是本次mktemp目录内的previous.bundle，不接受外部路径。
  python3 - "$BACKUP" <<'PY'
import pathlib, shutil, sys
p = pathlib.Path(sys.argv[1])
assert p.name == 'previous.bundle' and p.parent.name.startswith('.mac-tunnel-install.')
shutil.rmtree(p)
PY
fi
rmdir "$STAGE"
# 不仅注销：移除可被macOS重新扫描的旧名/构建副本，保留唯一安装包。
python3 "$ROOT/scripts/app_bundles.py" cleanup "$DEST" "${MANAGED_ROOTS[@]}"
printf '安装完成：%s（版本 %s）\n注意：本机测试包未做 Developer ID 签名/公证。\n' "$DEST" "$BUILT_VERSION"
if [[ "$LAUNCH" == true ]]; then /usr/bin/open "$DEST"; fi
