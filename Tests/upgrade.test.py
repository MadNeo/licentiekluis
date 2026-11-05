"""Exercise actual upgrade script in isolated paths with fake external services.

Runs on Linux CI; no real system service or user data is changed.
"""
from pathlib import Path
import os
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'scripts/update-server.sh'


class UpgradeTests(unittest.TestCase):
    def exercise(self, scenario):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            app = root / 'opt/licentiekluis/app'
            data = root / 'var/lib/licentiekluis'
            source = root / 'source'
            commands = root / 'commands'
            for path in [app, data, source / 'scripts', commands,
                         root / 'etc/systemd/system', root / 'etc/caddy',
                         root / 'opt/licentiekluis/dotnet']:
                path.mkdir(parents=True, exist_ok=True)
            (app / 'LicenseVault.Server.dll').write_text('old')
            expected = {'server.lkv': b'original-encrypted-database',
                        'secrets.json': b'original-key-and-account',
                        'smtp.lkv': b'original-mail-settings'}
            for name, content in expected.items():
                (data / name).write_bytes(content)
            (root / 'etc/systemd/system/licentiekluis.service').write_text('service')
            (root / 'etc/caddy/Caddyfile').write_text('https config')
            (source / 'GITHUB-REVISION').write_text('test-commit')
            (source / 'scripts/register-upgrade.sh').write_text('#!/bin/bash\nexit 0\n')
            script = SCRIPT.read_text().replace('$EUID -eq 0', '1 -eq 1')
            for prefix in ['/opt/licentiekluis', '/var/lib', '/etc/systemd', '/etc/caddy']:
                script = script.replace(prefix, str(root) + prefix)
            target = source / 'scripts/update-server.sh'
            target.write_text(script)
            mock = '''#!/usr/bin/env python3
import os, sys
from pathlib import Path
root = Path(os.environ['TEST_ROOT'])
scenario = os.environ['TEST_SCENARIO']
name = Path(sys.argv[0]).name
args = sys.argv[1:]
app = root / 'opt/licentiekluis/app/LicenseVault.Server.dll'
if name == 'curl':
    if '-o' in args:
        Path(args[args.index('-o')+1]).write_text('#!/bin/bash\\nexit 0\\n')
    elif scenario == 'health-failure':
        sys.exit(22)
elif name == 'dotnet':
    if scenario == 'build-failure':
        sys.exit(1)
    if 'publish' in args:
        dest = Path(args[args.index('-o')+1]); dest.mkdir(parents=True)
        (dest / 'LicenseVault.Server.dll').write_text('new')
elif name == 'systemctl':
    with (root / 'service.log').open('a') as f: f.write(' '.join(args)+'\\n')
    if args[0] == 'start' and app.read_text() == 'new':
        if scenario in ['health-failure', 'start-failure']:
            (root / 'var/lib/licentiekluis/server.lkv').write_text('changed-by-new-server')
            (root / 'var/lib/licentiekluis/new-file').write_text('new')
        if scenario == 'start-failure': sys.exit(1)
elif name == 'tar':
    if scenario == 'backup-failure' and '-czf' in args: sys.exit(1)
    os.execv('/usr/bin/tar', ['tar']+args)
'''
            for name in ['curl', 'systemctl', 'sleep', 'tar']:
                path = commands / name
                path.write_text(mock)
                path.chmod(0o755)
            dotnet = root / 'opt/licentiekluis/dotnet/dotnet'
            dotnet.write_text(mock)
            dotnet.chmod(0o755)
            env = dict(os.environ, PATH=str(commands)+':'+os.environ['PATH'],
                       TEST_ROOT=str(root), TEST_SCENARIO=scenario)
            result = subprocess.run(['bash', str(target)], env=env,
                                    capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode == 0, scenario == 'success', result.stderr)
            self.assertEqual((app / 'LicenseVault.Server.dll').read_text(),
                             'new' if scenario == 'success' else 'old')
            self.assertEqual({p.name: p.read_bytes() for p in data.iterdir()}, expected)
            if scenario == 'build-failure':
                self.assertFalse((root / 'service.log').exists())
            else:
                self.assertIn('start licentiekluis', (root / 'service.log').read_text())
            if scenario in ['success', 'health-failure', 'start-failure']:
                backups = list((root / 'opt/licentiekluis').glob('backup-*/data.tar.gz'))
                self.assertEqual(len(backups), 1)

    def test_success_preserves_data(self): self.exercise('success')
    def test_build_failure_leaves_live_server_untouched(self): self.exercise('build-failure')
    def test_backup_failure_restarts_old_server(self): self.exercise('backup-failure')
    def test_health_failure_restores_database_and_program(self): self.exercise('health-failure')
    def test_start_failure_restores_database_and_program(self): self.exercise('start-failure')


if __name__ == '__main__':
    unittest.main()
