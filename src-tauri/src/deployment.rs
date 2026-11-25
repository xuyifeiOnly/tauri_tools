use crate::config::Config;
use serde::Serialize;
#[derive(Serialize)]
pub struct Deployment {
    pub authorization: String,
    pub nginx: String,
}
pub fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
pub fn generate(c: &Config, public: &str) -> Result<Deployment, String> {
    c.validate()?;
    if public.contains(['\n', '\r']) || !public.starts_with("ssh-") {
        return Err("公钥不合法".into());
    }
    let line = format!("restrict,port-forwarding,command=\"/bin/false\" {public}");
    Ok(Deployment{authorization:format!("# 以 root 执行；目标账号须已存在。旧 OpenSSH 不支持按远端端口限权。\nset -eu\nuser={}\nhome=$(getent passwd \"$user\" | cut -d: -f6)\n[ -n \"$home\" ] || exit 1\ninstall -d -m 700 -o \"$user\" \"$home/.ssh\"\ntouch \"$home/.ssh/authorized_keys\"\nchmod 600 \"$home/.ssh/authorized_keys\"\nchown \"$user\" \"$home/.ssh/authorized_keys\"\nkey={}\ngrep -qxF -- \"$key\" \"$home/.ssh/authorized_keys\" || printf '\\n%s\\n' \"$key\" >> \"$home/.ssh/authorized_keys\"\n# sshd 须允许此账号 AllowTcpForwarding remote，GatewayPorts no；禁止密码、TTY及Agent转发。\n",quote(&c.user),quote(&line)),nginx:format!("# 放入对应 HTTPS server 块中，保留前缀。\nlocation = {0} {{ return 308 {0}/; }}\nlocation ^~ {0}/ {{\n    proxy_pass http://127.0.0.1:{1};\n    proxy_http_version 1.1;\n    proxy_set_header Host $host;\n    proxy_set_header X-Forwarded-Proto $scheme;\n    proxy_buffering off;\n    proxy_read_timeout 300s;\n}}\n# 保存后：nginx -t && nginx -s reload\n",c.base_path,c.remote_port)})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn injection() {
        assert_eq!(quote("a'b"), "'a'\\''b'");
        assert!(generate(&Config::default(), "ssh-ed25519 abc\nbad").is_err())
    }
}
