"""Exercise the actual host wizard with isolated fake Proxmox commands."""
import io, os, subprocess, tarfile, tempfile, unittest
from pathlib import Path
SOURCE = Path(__file__).resolve().parents[1]

class ProxmoxTests(unittest.TestCase):
    def exercise(self, scenario):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); commands = root/'bin'; commands.mkdir()
            (root/'etc/pve').mkdir(parents=True)
            (root/'sys/class/net/vmbr0/bridge').mkdir(parents=True)
            (root/'run/lock').mkdir(parents=True); (root/'host-root').mkdir()
            with tarfile.open(root/'source.tar.gz','w:gz') as archive:
                for name, value in {'scripts/install-local.sh':'# LKV_INIT_FILE\n', 'Server/LicenseVault.Server.csproj':'project'}.items():
                    data=value.encode(); info=tarfile.TarInfo('MadNeo-test/'+name);info.size=len(data);archive.addfile(info,io.BytesIO(data))
            mock='''#!/usr/bin/env python3
import os,sys,json,shutil
from pathlib import Path
root=Path(os.environ['TEST_ROOT']); scenario=os.environ['SCENARIO']; name=Path(sys.argv[0]).name; args=sys.argv[1:]
with (root/'calls').open('a') as f: f.write(json.dumps([name]+args)+'\\n')
if name=='pvesh':
    if '--vmid' in args and scenario=='occupied': sys.exit(1)
    print('120')
elif name=='pvesm': print('Name Type Status Total Used Available %\\nlocal dir active 100 0 100 0')
elif name=='pveam' and args[0]=='available': print('system ubuntu-24.04-standard_24.04-2_amd64.tar.zst')
elif name=='dpkg-query': print('install ok installed')
elif name=='curl':
    if scenario=='bad-token': sys.exit(22)
    shutil.copy(root/'source.tar.gz',args[args.index('-o')+1])
elif name=='pct':
    if args[0]=='create': (root/'created').touch()
    if args[0]=='push':
        if args[3].endswith('licentiekluis-init.txt'):
            assert Path(args[2]).read_text()=='admin\\nvery-secret-password\\nvery-secret-password\\n'
        if args[3].endswith('github-token'): assert Path(args[2]).read_text()=='github-secret-token'
    if args[0]=='exec' and 'addr' in args:
        if scenario!='no-network': print('2: eth0 inet 192.168.178.50/24 scope global eth0')
    if args[0]=='exec' and any(a.endswith('/scripts/install-local.sh') for a in args):
        if scenario=='install-failure': sys.exit(1)
    if args[0]=='pull': Path(args[3]).write_text('PUBLIC-CA')
'''
            for name in ['pveversion','pvesh','pvesm','pveam','pct','curl','dpkg-query','sleep']:
                p=commands/name;p.write_text(mock);p.chmod(0o755)
            script=(SOURCE/'scripts/install-proxmox.sh').read_text().replace('[[ -t 0 ]]','true').replace('[[ $EUID -eq 0 ]]','true')
            for original,new in [('/etc/pve',str(root/'etc/pve')),('/sys/class/net',str(root/'sys/class/net')),('/run/lock',str(root/'run/lock')),('/root/licentiekluis-',str(root/'host-root/licentiekluis-'))]:
                script=script.replace(original,new)
            target=root/'installer.sh';target.write_text(script)
            # ID, hostname, two storages, bridge, IP, VLAN, CPU, RAM, disk, DNS, account, passwords, token, confirmation.
            answers=['']*11+['admin','very-secret-password','very-secret-password','github-secret-token','ja']
            if scenario=='inherited-token': answers.pop(-2)
            if scenario=='static':
                answers[5]='192.168.178.51/24';answers.insert(6,'192.168.178.1')
            result=subprocess.run(['bash',str(target)],input='\n'.join(answers)+'\n',text=True,capture_output=True,
                env=dict(os.environ,PATH=str(commands)+':'+os.environ['PATH'],TEST_ROOT=str(root),SCENARIO=scenario,GITHUB_TOKEN='github-secret-token' if scenario=='inherited-token' else ''),timeout=30)
            self.assertEqual(result.returncode==0,scenario in ['success','static','inherited-token'],result.stdout+result.stderr)
            calls=(root/'calls').read_text()
            self.assertNotIn('very-secret-password',calls);self.assertNotIn('github-secret-token',calls)
            self.assertNotIn('"destroy"',calls);self.assertNotIn('"stop"',calls)
            if scenario in ['occupied','bad-token']: self.assertFalse((root/'created').exists())
            else: self.assertTrue((root/'created').exists())
            if scenario=='success':
                self.assertIn('https://192.168.178.50',result.stdout)
                self.assertIn('"--unprivileged", "1"',calls)
                self.assertIn('"/etc/licentiekluis/github-token", "--perms", "0600"',calls)
            if scenario in ['no-network','install-failure']: self.assertIn('behouden',result.stderr)
    def test_token_passed_by_bootstrap(self): self.exercise('inherited-token')
    def test_static_network(self): self.exercise('static')
    def test_success(self): self.exercise('success')
    def test_existing_guest_not_touched(self): self.exercise('occupied')
    def test_invalid_token_fails_before_creation(self): self.exercise('bad-token')
    def test_no_network_preserves_guest(self): self.exercise('no-network')
    def test_install_failure_preserves_guest(self): self.exercise('install-failure')

if __name__=='__main__': unittest.main()
