"""Published server smoke test: authenticated web/Windows protocol, durable encrypted data."""
import json, os, pathlib, socket, subprocess, tempfile, time, urllib.request, urllib.error, uuid
app = pathlib.Path(__file__).resolve().parents[1] / 'output/Server'
dll = app / 'LicenseVault.Server.dll'
assert dll.is_file(), 'Publish the server to output/Server first'
with tempfile.TemporaryDirectory() as temp:
    env = dict(os.environ, LKV_DATA=temp)
    password = 'synthetic-integration-password'
    subprocess.run(['dotnet', str(dll), '--init-stdin'], input=f'test\n{password}\n{password}\n', text=True, env=env, cwd=app, check=True, capture_output=True)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0)); port = sock.getsockname()[1]
    base = f'http://127.0.0.1:{port}'
    token = ''
    def request(path, value=None, auth=True):
        headers = {'Content-Type': 'application/json'}
        if auth: headers['Authorization'] = f'Bearer {token}'
        req = urllib.request.Request(base+path, data=None if value is None else json.dumps(value).encode(), headers=headers)
        with urllib.request.urlopen(req, timeout=10) as result: return result.read()
    def start():
        proc = subprocess.Popen(['dotnet',str(dll),'--urls',base],env=env,cwd=app,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        for _ in range(100):
            try: request('/health',auth=False); return proc
            except (OSError, urllib.error.URLError): time.sleep(.1)
        proc.terminate(); proc.wait(); raise AssertionError('Server did not start')
    def login(): return json.loads(request('/api/login',{'username':'test','password':password},False))
    proc = start()
    try:
        for name in ['/', '/app.js', '/mail.js', '/manage.js', '/vault.js', '/app.css', '/theme.js', '/changelog', '/changelog.txt', '/changelog.js']:
            assert request(name,auth=False), name
        try: request('/api/sync',{'vaultId':str(uuid.uuid4()),'changes':[]},False); raise AssertionError('Unauthenticated access')
        except urllib.error.HTTPError as e: assert e.code == 401
        session=login();token=session['token'];vault=session['vaultId']
        def entry(kind,name): return dict(id=str(uuid.uuid4()),kind=kind,name=name,ownerId=None,code='',softwareVersion='',notes='',expires=None,installations=[],deleted=False,revision=0,mutationId=str(uuid.uuid4()))
        person=entry('person','Testpersoon');license=entry('license','Testsoftware');license.update(ownerId=person['id'],code='SYNTHETIC-SECRET',installations=[dict(platform='Windows',device='PC')])
        def sync(changes): return json.loads(request('/api/sync',dict(vaultId=vault,changes=changes)))
        result=sync([dict(value=person,baseRevision=0),dict(value=license,baseRevision=0)]);assert len(result['applied'])==2
        current=next(e for e in result['entries'] if e['id']==license['id']);current.update(name='Windows update',mutationId=str(uuid.uuid4()))
        sync([dict(value=current,baseRevision=1)])
        license.update(name='Stale web update',mutationId=str(uuid.uuid4()));result=sync([dict(value=license,baseRevision=1)]);assert result['conflicts']==[license['id']]
        person.update(deleted=True,mutationId=str(uuid.uuid4()));result=sync([dict(value=person,baseRevision=1)]);assert not next(e for e in result['entries'] if e['id']==license['id'])['deleted']
        data=(pathlib.Path(temp)/'server.lkv').read_bytes();assert data.startswith(b'LKV1') and b'SYNTHETIC-SECRET' not in data
        proc.terminate();proc.wait();proc=start();session=login();token=session['token'];assert session['vaultId']==vault
        result=sync([]);assert next(e for e in result['entries'] if e['id']==license['id'])['name']=='Windows update'
        assert next(e for e in result['entries'] if e['id']==license['id'])['code']=='SYNTHETIC-SECRET'
        print('PASS: published assets, auth, web/Windows sync, conflicts, deletion preservation, encryption and restart durability')
    finally: proc.terminate();proc.wait(timeout=10)
