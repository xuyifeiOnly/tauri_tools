"""Identity-checked cleanup of managed app bundles, never application data."""
import argparse
import pathlib
import plistlib
import shutil
import subprocess

IDENTIFIER = 'cn.rutang.mac-tunnel'
LSREGISTER = '/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister'


def identifier(app):
    return plistlib.loads((app / 'Contents/Info.plist').read_bytes()).get('CFBundleIdentifier')


def validate_target(app):
    if app.is_symlink():
        raise ValueError(f'拒绝处理符号链接：{app}')
    if app.exists() and (not app.is_dir() or identifier(app) != IDENTIFIER):
        raise ValueError(f'目标不是随享（原Mac Tunnel），拒绝覆盖：{app}')


def candidates(roots):
    result = []
    for root in dict.fromkeys(roots):
        if root.is_symlink():
            raise ValueError(f'扫描目录不能是符号链接：{root}')
        for app in sorted(root.glob('*.app')):
            if app.is_symlink():
                # Do not follow or remove unrelated links either.
                if app.name in ('随享.app', 'Mac Tunnel.app'):
                    raise ValueError(f'拒绝处理符号链接：{app}')
                continue
            try:
                if identifier(app) == IDENTIFIER:
                    result.append(app)
            except (OSError, ValueError, plistlib.InvalidFileException):
                if app.name in ('随享.app', 'Mac Tunnel.app'):
                    raise ValueError(f'无法确认应用身份：{app}')
    return result


def cleanup(target, roots, unregister):
    validate_target(target)
    if not target.exists():
        raise ValueError(f'安装目标不存在，拒绝清理：{target}')
    apps = candidates(roots)
    for app in apps:
        if app == target:
            continue
        validate_target(app)
        unregister(app)
        shutil.rmtree(app)
        print(f'已移除受管重复应用：{app}')
    remaining = candidates(roots)
    if remaining != [target]:
        raise ValueError(f'应用去重核验失败：{remaining}')


def unregister(app):
    result = subprocess.run([LSREGISTER, '-u', str(app)], capture_output=True, text=True)
    if result.returncode:
        # A freshly built bundle may never have been registered (-10814).
        # Register then unregister to distinguish that case from a real failure.
        if '-10814' not in result.stdout + result.stderr:
            result.check_returncode()
        subprocess.run([LSREGISTER, '-f', str(app)], check=True, capture_output=True)
        subprocess.run([LSREGISTER, '-u', str(app)], check=True, capture_output=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('mode', choices=['preflight', 'cleanup'])
    parser.add_argument('target', type=pathlib.Path)
    parser.add_argument('roots', nargs='+', type=pathlib.Path)
    args = parser.parse_args()
    validate_target(args.target)
    apps = candidates(args.roots)
    for app in apps:
        # Verify write access before building; never sudo or silently skip.
        import os
        if not os.access(app.parent, os.W_OK) or not os.access(app, os.W_OK):
            raise ValueError(f'无权迁移重复应用，请手动处理：{app}')
    if args.mode == 'cleanup':
        cleanup(args.target, args.roots, unregister)
        subprocess.run([LSREGISTER, '-f', str(args.target)], check=True)
        print(f'唯一受管安装：{args.target}')


if __name__ == '__main__':
    main()
