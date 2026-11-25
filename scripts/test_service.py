"""本地无部署测试：仅在临时目录生成测试主机密钥，验证生成的SSH配置。"""
import pathlib
import shlex
import subprocess
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).with_name('service.sh')


class ServiceTests(unittest.TestCase):
    def test_help_and_syntax(self):
        subprocess.run(['bash', '-n', str(SCRIPT)], check=True)
        result = subprocess.run(['bash', str(SCRIPT), '--help'], capture_output=True, text=True, check=True)
        self.assertIn('Nginx仅输出片段', result.stdout)

    def test_every_executable_line_has_comment(self):
        for number, line in enumerate(SCRIPT.read_text().splitlines(), 1):
            if line.strip() and not line.lstrip().startswith('#'):
                self.assertIn('#', line, f'第{number}行缺少注释')

    def test_generated_sshd_syntax(self):
        line = next(line for line in SCRIPT.read_text().splitlines() if "'Match all'" in line)
        words = shlex.split(line.strip().split(' >> ', 1)[0])
        directives = [word.replace('$USER_NAME', 'mac-tunnel') for word in words[2:] if word != '$END']
        with tempfile.TemporaryDirectory(prefix='mac-tunnel-sshd-test-') as folder:
            key = pathlib.Path(folder) / 'host'
            subprocess.run(['/usr/bin/ssh-keygen', '-q', '-t', 'ed25519', '-N', '', '-f', str(key)], check=True)
            config = pathlib.Path(folder) / 'sshd_config'
            config.write_text(f'HostKey {key}\n' + '\n'.join(directives) + '\n')
            subprocess.run(['/usr/sbin/sshd', '-t', '-f', str(config)], check=True)
            effective = subprocess.run(['/usr/sbin/sshd', '-T', '-f', str(config), '-C', 'user=mac-tunnel,host=example.com,addr=127.0.0.1'], capture_output=True, text=True, check=True).stdout
            for expected in ['allowtcpforwarding remote', 'gatewayports no', 'passwordauthentication no', 'kbdinteractiveauthentication no', 'forcecommand /bin/false']:
                self.assertIn(expected, effective)


if __name__ == '__main__':
    unittest.main()
