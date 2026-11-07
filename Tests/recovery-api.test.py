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
        headers = {'Content-Type': 'application/json', 'X-LicentieKluis-Protocol':'2'}
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
        for name in ['/', '/app.js', '/mail.js', '/manage.js', '/vault.js', '/app.css', '/theme.js', '/changelog', '/changelog.txt', '/changelog.js', '/settings', '/settings.js', '/recovery', '/recovery.js']:
            assert request(name,auth=False), name
        try: request('/api/sync',{'vaultId':str(uuid.uuid4()),'changes':[]},False); raise AssertionError('Unauthenticated access')
        except urllib.error.HTTPError as e: assert e.code == 401
        session=login();token=session['token'];vault=session['vaultId']
        def entry(kind,name): return dict(id=str(uuid.uuid4()),kind=kind,name=name,ownerId=None,code='',softwareVersion='',notes='',expires=None,installations=[],deleted=False,revision=0,mutationId=str(uuid.uuid4()))
        import base64
        photo=dict(name='recovery.png',mediaType='image/png',data=base64.b64encode((app.parent.parent/'fixtures/recovery-test.png').read_bytes()).decode())
        def sync(changes): return json.loads(request('/api/sync',dict(vaultId=vault,changes=changes)))
        saved=[]
        for index,forms in enumerate([dict(text='SYNTHETIC MANUAL'),dict(textFile=dict(name='recovery.txt',text='SYNTHETIC FILE')),dict(photo=photo),dict(text='SYNTHETIC MANUAL',textFile=dict(name='recovery.txt',text='SYNTHETIC FILE'),photo=photo)]):
            e=entry('recovery','Recovery '+str(index));e['recovery']=dict(category='wallet' if index%2 else 'authenticator',text='',textFile=None,photo=None,**{})
            e['recovery'].update(forms);result=sync([dict(value=e,baseRevision=0)]);saved.append(next(x for x in result['entries'] if x['id']==e['id']))
        assert len(saved)==4
        overview=json.loads(request('/api/entries'))
        assert all(e.get('recovery') is None for e in overview)
        assert b'SYNTHETIC' not in request('/api/entries')
        # Old clients cannot drop fields they do not understand.
        old=urllib.request.Request(base+'/api/sync',data=json.dumps(dict(vaultId=vault,changes=[])).encode(),headers={'Content-Type':'application/json','Authorization':f'Bearer {token}'})
        try: urllib.request.urlopen(old);raise AssertionError('Old client allowed')
        except urllib.error.HTTPError as error: assert error.code==409
        for recovery in [dict(category='wallet',text=''),dict(category='wallet',textFile=dict(name='../bad.txt',text='x')),dict(category='wallet',photo=dict(name='evil.png',mediaType='image/png',data=base64.b64encode(b'<svg onload=x>').decode()))]:
            invalid=entry('recovery','Invalid');invalid['recovery']=recovery
            try:sync([dict(value=invalid,baseRevision=0)]);raise AssertionError('Invalid accepted')
            except urllib.error.HTTPError as error:assert error.code==400
        # Conflict retains complete local and remote payload; retries are idempotent.
        edited=dict(saved[3],mutationId=str(uuid.uuid4()),recovery=dict(saved[3]['recovery'],text='NEW SYNTHETIC TEXT'))
        updated=sync([dict(value=edited,baseRevision=1)])
        retry=sync([dict(value=edited,baseRevision=1)]);assert retry['applied']==[edited['id']]
        stale=dict(saved[3],mutationId=str(uuid.uuid4()));conflict=sync([dict(value=stale,baseRevision=1)]);assert conflict['conflicts']==[stale['id']]
        blob=(pathlib.Path(temp)/'server.lkv').read_bytes();assert blob.startswith(b'LKV1') and b'SYNTHETIC' not in blob and photo['data'].encode() not in blob
        proc.terminate();proc.wait();proc=start();session=login();token=session['token'];assert session['vaultId']==vault
        result=sync([]);assert len(result['entries'])==4
        final=next(e for e in result['entries'] if e['id']==edited['id']);assert final['recovery']['text']=='NEW SYNTHETIC TEXT' and final['recovery']['photo']==photo
        final.update(deleted=True,mutationId=str(uuid.uuid4()),recovery=dict(category='authenticator',text='',textFile=None,photo=None))
        deleted=sync([dict(value=final,baseRevision=2)]);assert next(e for e in deleted['entries'] if e['id']==final['id'])['deleted']
        print('PASS: recovery API auth, three inputs plus combination, redaction, old-client guard, validation, conflict/retry, encrypted storage, restart and tombstone')
    finally: proc.terminate();proc.wait(timeout=10)
