"""Published settings API + local STARTTLS fixture; no external email or credentials."""
import base64, json, os, pathlib, socket, socketserver, ssl, subprocess, tempfile, threading, time, urllib.error, urllib.request, uuid
app = pathlib.Path(__file__).resolve().parents[1] / 'output/Server'
dll = app / 'LicenseVault.Server.dll'
assert dll.is_file(), 'Publish the server first'
with tempfile.TemporaryDirectory() as temp:
    root = pathlib.Path(temp)
    def openssl(*args): subprocess.run(['openssl', *args], cwd=root, check=True, capture_output=True)
    openssl('req','-x509','-newkey','rsa:2048','-nodes','-keyout','ca.key','-out','ca.crt','-days','1','-subj','/CN=LKV test CA','-addext','basicConstraints=critical,CA:TRUE')
    openssl('req','-newkey','rsa:2048','-nodes','-keyout','server.key','-out','server.csr','-subj','/CN=127.0.0.1')
    (root/'server.ext').write_text('subjectAltName=IP:127.0.0.1\nbasicConstraints=CA:FALSE\nextendedKeyUsage=serverAuth\n')
    openssl('x509','-req','-in','server.csr','-CA','ca.crt','-CAkey','ca.key','-CAcreateserial','-out','server.crt','-days','1','-extfile','server.ext')
    tls=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);tls.load_cert_chain(root/'server.crt',root/'server.key')
    received=[];auths=[];smtp_errors=[]
    class SMTP(socketserver.BaseRequestHandler):
        def handle(self):
            connection=self.request;connection.settimeout(10);stream=connection.makefile('rb');encrypted=False
            def reply(data):connection.sendall(data.encode())
            try:
                reply('220 fixture ESMTP\r\n')
                while True:
                    line=stream.readline();command=line.decode().strip()
                    if not line:break
                    if command.startswith('EHLO'):
                        reply('250-fixture\r\n250-STARTTLS\r\n250 AUTH LOGIN\r\n' if not encrypted else '250-fixture\r\n250 AUTH LOGIN\r\n')
                    elif command=='STARTTLS':
                        reply('220 Start TLS\r\n');stream.close();connection=tls.wrap_socket(connection,server_side=True);stream=connection.makefile('rb');encrypted=True
                    elif command.upper().startswith('AUTH LOGIN'):
                        assert encrypted
                        parts=command.split()
                        if len(parts)>2:user=base64.b64decode(parts[2]).decode()
                        else:
                            reply('334 VXNlcm5hbWU6\r\n');user=base64.b64decode(stream.readline().strip()).decode()
                        reply('334 UGFzc3dvcmQ6\r\n');secret=base64.b64decode(stream.readline().strip()).decode();auths.append((user,secret));reply('235 Authenticated\r\n')
                    elif command.startswith(('MAIL FROM:','RCPT TO:')):assert encrypted;reply('250 OK\r\n')
                    elif command=='DATA':
                        assert encrypted;reply('354 End with dot\r\n');data=[]
                        while True:
                            chunk=stream.readline()
                            if chunk in (b'.\r\n',b''):break
                            data.append(chunk)
                        received.append(b''.join(data));reply('250 Accepted\r\n')
                    elif command=='QUIT':reply('221 Bye\r\n');break
                    else:raise AssertionError(command)
            except Exception as exc:smtp_errors.append(type(exc).__name__+': '+str(exc))
            finally:stream.close();connection.close()
    smtp=socketserver.ThreadingTCPServer(('127.0.0.1',0),SMTP);smtp.daemon_threads=True
    thread=threading.Thread(target=smtp.serve_forever,daemon=True);thread.start()
    env=dict(os.environ,LKV_DATA=str(root/'data'),SSL_CERT_FILE=str(root/'ca.crt'))
    password='synthetic-integration-password'
    subprocess.run(['dotnet',str(dll),'--init-stdin'],input=f'test\n{password}\n{password}\n',text=True,env=env,cwd=app,check=True,capture_output=True)
    with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
    base=f'http://127.0.0.1:{port}';token=''
    def request(path,body=None,method=None,auth=True):
        headers={'Content-Type':'application/json'}
        if auth:headers['Authorization']='Bearer '+token
        req=urllib.request.Request(base+path,data=json.dumps(body).encode() if body is not None else None,headers=headers,method=method)
        try:
            with urllib.request.urlopen(req,timeout=40) as response:return response.status,json.loads(response.read())
        except urllib.error.HTTPError as error:return error.code,json.loads(error.read() or b'{}')
    def start():
        process=subprocess.Popen(['dotnet',str(dll),'--urls',base],env=env,cwd=app,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        for _ in range(100):
            try:request('/health',auth=False);return process
            except OSError:time.sleep(.1)
        process.terminate();process.wait();raise AssertionError('Server did not start')
    def login():
        status,data=request('/api/login',{'username':'test','password':password},auth=False);assert status==200;return data
    proc=start()
    try:
        for endpoint,method,body in [('/api/settings','GET',None),('/api/settings/mail','PUT',{}),('/api/settings/mail/test','POST',{})]:
            assert request(endpoint,body,method,False)[0]==401
        session=login();token=session['token'];status,data=request('/api/settings');assert status==200
        assert data['mail']=={'configured':False,'host':'','port':587,'from':'','username':'','passwordSaved':False,'revision':'new'}
        config=dict(host='127.0.0.1',port=smtp.server_address[1],from_='vault@example.invalid',username='fixture',password='SECRET-SMTP-PASSWORD',clearPassword=False,revision='new')
        config['from']=config.pop('from_')
        status,saved=request('/api/settings/mail',config,'PUT');assert status==200,(status,saved)
        assert saved['passwordSaved'] and 'password' not in saved and 'SECRET-SMTP' not in json.dumps(saved)
        encrypted=(root/'data/smtp.lkv').read_bytes();assert encrypted.startswith(b'LKV1') and b'SECRET-SMTP-PASSWORD' not in encrypted
        assert (root/'data/smtp.lkv').stat().st_mode & 0o777==0o600
        # Keep a stored password only for the same endpoint and account.
        config.update(password='',revision=saved['revision']);config['from']='changed@example.invalid'
        status,saved=request('/api/settings/mail',config,'PUT');assert status==200
        assert request('/api/settings/mail',config,'PUT')[0]==409
        config['revision']=saved['revision']
        for change in [dict(port=465),dict(host='https://smtp.example.invalid'),dict(host='localhost'),dict(username='different'),{'from':'good@example.invalid\r\nBcc:bad@example.invalid'}]:
            assert request('/api/settings/mail',dict(config,**change),'PUT')[0]==400,change
        assert request('/api/settings')[1]['mail']['revision']==saved['revision']
        test=dict(requestId=str(uuid.uuid4()),recipient='test@example.invalid',revision=saved['revision'])
        status,result=request('/api/settings/mail/test',test);assert status==200 and result['status']=='submitted',(status,result,smtp_errors)
        assert len(received)==1 and auths==[('fixture','SECRET-SMTP-PASSWORD')]
        assert b'SECRET-SMTP-PASSWORD' not in received[0] and b'SYNTHETIC-LICENSE' not in received[0]
        assert request('/api/settings/mail/test',test)[1]['status']=='submitted' and len(received)==1
        assert request('/api/settings/mail/test',dict(test,recipient='changed@example.invalid'))[0]==409
        assert request('/api/settings/mail/test',dict(test,requestId=str(uuid.uuid4()),revision='stale'))[0]==409
        # The very same settings work for the existing license-mail endpoint.
        def entry(kind,name):return dict(id=str(uuid.uuid4()),kind=kind,name=name,ownerId=None,code='',softwareVersion='',notes='',expires=None,installations=[],deleted=False,revision=0,mutationId=str(uuid.uuid4()))
        person=entry('person','Fixture');license=entry('license','Fixture software');license.update(ownerId=person['id'],code='SYNTHETIC-LICENSE')
        status,synced=request('/api/sync',dict(vaultId=session['vaultId'],changes=[dict(value=person,baseRevision=0),dict(value=license,baseRevision=0)]));assert status==200
        status,result=request('/api/mail/send',dict(requestId=str(uuid.uuid4()),recipient='test@example.invalid',subject='Fixture',licenses=[dict(id=license['id'],revision=1)]));assert status==200 and result['status']=='submitted'
        assert len(received)==2
        # Restart keeps configuration and the durable deduplication record.
        proc.terminate();proc.wait();proc=start();token=login()['token']
        assert request('/api/settings')[1]['mail']==saved
        assert request('/api/settings/mail/test',test)[1]['status']=='submitted' and len(received)==2
        config.update(username='',clearPassword=True,revision=saved['revision'])
        status,saved=request('/api/settings/mail',config,'PUT');assert status==200 and not saved['passwordSaved']
        assert not smtp_errors,smtp_errors
        print('PASS: authenticated settings, validation, stale writes, encrypted/password-preserving storage, STARTTLS testmail + license mail, request deduplication, restart and relay switch')
    finally:proc.terminate();proc.wait(timeout=10);smtp.shutdown();smtp.server_close();thread.join()
