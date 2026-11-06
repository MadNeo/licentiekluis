"""Exercise registration in temporary paths; never modify host commands or data."""
from pathlib import Path
import os
import subprocess
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parents[1] / 'scripts/register-upgrade.sh'

class RegistrationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.scripts = self.root / 'source/scripts'
        self.scripts.mkdir(parents=True)
        for folder in ['usr/bin', 'usr/local/bin', 'opt/licentiekluis', 'var/lib/licentiekluis']:
            (self.root / folder).mkdir(parents=True, exist_ok=True)
        self.data = self.root / 'var/lib/licentiekluis/server.lkv'
        self.data.write_bytes(b'unchanged-encrypted-database')
        self.installer = self.scripts / 'install-lxc.sh'
        self.installer.write_text('#!/usr/bin/env bash\nprintf "%s\\n" "$*"\n')
        script = SOURCE.read_text().replace('$EUID -eq 0', '1 -eq 1')
        for prefix in ['/opt/licentiekluis', '/usr/local/bin', '/usr/bin/upgrade']:
            script = script.replace(prefix, str(self.root) + prefix)
        self.target = self.scripts / 'register-upgrade.sh'
        self.target.write_text(script)

    def run_registration(self):
        return subprocess.run(['bash', str(self.target)], capture_output=True, text=True)

    def test_missing_command_repaired_and_repeatable(self):
        for _ in range(2):
            result = self.run_registration()
            self.assertEqual(result.returncode, 0, result.stderr)
        wrapper = self.root / 'usr/local/bin/upgrade'
        self.assertEqual(wrapper.stat().st_mode & 0o777, 0o755)
        env = dict(os.environ, PATH=str(self.root / 'usr/bin')+':/usr/bin:/bin')
        result = subprocess.run(['bash','-c','upgrade --help'],env=env,capture_output=True,text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('LicentieKluis upgrade', result.stdout)
        result = subprocess.run(['bash','-c','upgrade'],env=env,capture_output=True,text=True)
        self.assertEqual(result.stdout.strip(), 'upgrade')
        self.assertEqual(self.data.read_bytes(), b'unchanged-encrypted-database')

    def test_unrelated_commands_not_overwritten(self):
        for relative in ['usr/local/bin/upgrade', 'usr/bin/upgrade']:
            path=self.root / relative
            path.write_text('unrelated-command')
            self.assertNotEqual(self.run_registration().returncode,0)
            self.assertEqual(path.read_text(),'unrelated-command')
            self.assertFalse((self.root/'opt/licentiekluis/github-installer.sh').exists())
            path.unlink()

    def test_missing_source_keeps_existing_updater(self):
        self.assertEqual(self.run_registration().returncode,0)
        wrapper=self.root/'usr/local/bin/upgrade'
        before=wrapper.read_bytes()
        self.installer.unlink()
        self.assertNotEqual(self.run_registration().returncode,0)
        self.assertEqual(wrapper.read_bytes(),before)

    def test_broken_unrelated_symlink_is_preserved(self):
        path=self.root/'usr/bin/upgrade'
        path.symlink_to('/missing-unrelated-command')
        self.assertNotEqual(self.run_registration().returncode,0)
        self.assertEqual(os.readlink(path),'/missing-unrelated-command')

if __name__ == '__main__': unittest.main()
