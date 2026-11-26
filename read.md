# 本地打包安装
```bash
./scripts/app_install.sh --open
```
# 缓存清理
```bash
cargo clean --manifest-path src-tauri/Cargo.toml
```

# 线上自动打包流程
![](./images/image.tiff)


## windows部署
```bash
# 查看隧道账号
sudo grep -nEi '^[[:space:]]*(Match|AllowUsers|AllowTcpForwarding|PermitListen|DisableForwarding)' \
  /etc/ssh/sshd_config /etc/ssh/sshd_config.d/*.conf 2>/dev/null
```
