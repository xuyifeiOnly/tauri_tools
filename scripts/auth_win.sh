#!/usr/bin/env bash
# 在 Linux 服务器以 root 执行：给已有隧道账号追加 Windows 公钥。
# 不创建账号、不覆盖现有授权、不修改 SSH 配置或重启服务。
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "错误：请通过 sudo bash scripts/auth_win.sh 执行" >&2
  exit 1
fi

read -r -p "请输入已有的 SSH 隧道账号名：" tunnel_user </dev/tty
[ -n "$tunnel_user" ] || {
  echo "错误：账号不能为空" >&2
  exit 1
}

account=$(getent passwd "$tunnel_user") || {
  echo "错误：服务器不存在账号 $tunnel_user，未修改任何授权" >&2
  exit 1
}

IFS=: read -r account_name _ account_uid account_gid _ account_home _ <<< "$account"

[ "$account_uid" != 0 ] || {
  echo "错误：请使用专用低权限隧道账号，不使用 root" >&2
  exit 1
}
[[ "$account_home" = /* && "$account_home" != / && -d "$account_home" ]] || {
  echo "错误：账号 home 目录无效：$account_home" >&2
  exit 1
}

ssh_dir="$account_home/.ssh"
authorized_keys="$ssh_dir/authorized_keys"

if [ -L "$ssh_dir" ] || [ -L "$authorized_keys" ]; then
  echo "错误：.ssh 或 authorized_keys 是符号链接，请人工检查" >&2
  exit 1
fi
if [ -e "$ssh_dir" ] && [ ! -d "$ssh_dir" ]; then
  echo "错误：.ssh 不是目录" >&2
  exit 1
fi
if [ -e "$authorized_keys" ] && [ ! -f "$authorized_keys" ]; then
  echo "错误：authorized_keys 不是普通文件" >&2
  exit 1
fi

key_type='ssh-ed25519'
key_data='AAAAC3NzaC1lZDI1NTE5AAAAIG5+pNc92+3J9VfopWyQ4jBSfjT7l6KxM3SXytja1eIx'
key_line="restrict,port-forwarding,command=\"/bin/false\" $key_type $key_data mac-tunnel"

install -d -m 700 -o "$account_uid" -g "$account_gid" "$ssh_dir"
touch "$authorized_keys"
chmod 600 "$authorized_keys"
chown "$account_uid:$account_gid" "$authorized_keys"

# 按公钥类型和内容去重，保留已有授权及其限制，不覆盖。
if awk -v type="$key_type" -v key="$key_data" '
  /^[[:space:]]*#/ { next }
  {
    for (i = 1; i < NF; i++) {
      if ($i == type && $(i + 1) == key) found = 1
    }
  }
  END { exit !found }
' "$authorized_keys"; then
  echo "此公钥已存在，未重复添加，原有限制保持不变。"
else
  printf '\n%s\n' "$key_line" >> "$authorized_keys"
  echo "Windows 公钥已追加，原有设备公钥保持不变。"
fi

if command -v restorecon >/dev/null 2>&1; then
  restorecon -R "$ssh_dir"
fi

printf '\n完成。Windows 随享的 SSH 用户名请填写：%s\n' "$account_name"
echo "密钥选择与本公钥对应的 Windows 私钥，保存配置后重新启动本地服务并连接隧道。"
