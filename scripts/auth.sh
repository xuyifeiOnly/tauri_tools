# 以 root 执行；目标账号须已存在。旧 OpenSSH 不支持按远端端口限权。
set -eu
user='image-tunnel'
home=$(getent passwd "$user" | cut -d: -f6)
[ -n "$home" ] || exit 1
install -d -m 700 -o "$user" "$home/.ssh"
touch "$home/.ssh/authorized_keys"
chmod 600 "$home/.ssh/authorized_keys"
chown "$user" "$home/.ssh/authorized_keys"
key='restrict,port-forwarding,command="/bin/false" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINr0ts4eivPkc3rEUgrF1Lfn/o4sQQj6cJv+QT7wM+9v mac-tunnel'
grep -qxF -- "$key" "$home/.ssh/authorized_keys" || printf '\n%s\n' "$key" >> "$home/.ssh/authorized_keys"
# sshd 须允许此账号 AllowTcpForwarding remote，GatewayPorts no；禁止密码、TTY及Agent转发。


# /opt/1panel/apps/openresty/openresty/conf/default/00.default.conf
