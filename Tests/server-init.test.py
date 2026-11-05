"""Validate noninteractive bootstrap against the actual published server."""
import hashlib, base64, json, os, subprocess, tempfile, unittest
from pathlib import Path
DLL=Path('output/Server/LicenseVault.Server.dll').resolve()
class InitTests(unittest.TestCase):
    def run_init(self, path, data):
        return subprocess.run(['dotnet',str(DLL),'--init-stdin'],input=data,text=True,capture_output=True,
            env=dict(os.environ,LKV_DATA=str(path)),timeout=20)
    def test_init_and_existing_account_protected(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d); password='test-only-long-password'
            result=self.run_init(p,f'admin\n{password}\n{password}\n')
            self.assertEqual(result.returncode,0,result.stderr)
            original=(p/'secrets.json').read_bytes();config=json.loads(original)
            self.assertEqual(config['Username'],'admin')
            expected=hashlib.pbkdf2_hmac('sha256',password.encode(),base64.b64decode(config['Salt']),600000,32)
            self.assertEqual(base64.b64decode(config['Hash']),expected)
            self.assertNotIn(password,original.decode());self.assertNotIn(password,result.stdout)
            self.assertNotEqual(self.run_init(p,'new\nother-long-password\nother-long-password\n').returncode,0)
            self.assertEqual((p/'secrets.json').read_bytes(),original)
    def test_invalid_password_does_not_create_account(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertNotEqual(self.run_init(Path(d),'admin\nshort\nshort\n').returncode,0)
            self.assertFalse((Path(d)/'secrets.json').exists())
if __name__=='__main__':unittest.main()
