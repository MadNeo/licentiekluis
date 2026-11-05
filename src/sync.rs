use anyhow::{ensure,Result};
use crate::model::{LocalState,SyncResponse};
use serde::Deserialize;
use reqwest::blocking::Client;
use uuid::Uuid;

#[derive(Clone)] pub struct Session {pub url:String,pub token:String,pub vault_id:Uuid}
#[derive(Deserialize)] #[serde(rename_all="camelCase")] struct LoginResponse {token:String,vault_id:Uuid}
fn client()->Result<Client>{Ok(Client::builder().connect_timeout(std::time::Duration::from_secs(5)).timeout(std::time::Duration::from_secs(15)).redirect(reqwest::redirect::Policy::none()).build()?)}
pub fn login(url:&str,user:&str,password:&str)->Result<Session>{
    let u=reqwest::Url::parse(url)?;ensure!(u.scheme()=="https"&&u.username().is_empty()&&u.password().is_none()&&u.path()=="/"&&u.query().is_none()&&u.fragment().is_none(),"Gebruik een HTTPS-adres zonder pad of accountgegevens");
    let url=u.to_string();let r=client()?.post(format!("{}api/login",url)).json(&serde_json::json!({"username":user,"password":password})).send()?;
    ensure!(r.status().is_success(),"Aanmelden mislukt (HTTP {}); controleer account of wacht een minuut",r.status());let data:LoginResponse=r.json()?;Ok(Session{url,token:data.token,vault_id:data.vault_id})
}
pub fn synchronize(state:&mut LocalState,session:&Session)->Result<()> {
    ensure!(state.conflicts.is_empty(),"Los eerst de conflicten op");let mut next=state.clone();next.bind(session.url.clone(),session.vault_id)?;
    let r=client()?.post(format!("{}api/sync",session.url)).bearer_auth(&session.token).json(&next.request()?).send()?;
    ensure!(r.status().is_success(),"Synchronisatie mislukt (HTTP {}). Bij 401 opnieuw aanmelden",r.status());let response:SyncResponse=r.json()?;next.merge(response)?;*state=next;Ok(())
}
pub fn logout(session:&Session){if let Ok(c)=client(){let _=c.post(format!("{}api/logout",session.url)).bearer_auth(&session.token).send();}}
pub fn send_mail(session:&Session,request:&crate::mail::Request)->Result<crate::mail::Outcome>{
    let c=Client::builder().timeout(std::time::Duration::from_secs(45)).redirect(reqwest::redirect::Policy::none()).build()?;
    let r=c.post(format!("{}api/mail/send",session.url)).bearer_auth(&session.token).json(request).send()?;
    let status=r.status();let data:serde_json::Value=r.json()?;
    ensure!(status.is_success(),"{}",data.get("error").and_then(|e|e.as_str()).unwrap_or("Verzending niet bevestigd; controleer ontvangst voordat je opnieuw probeert"));
    Ok(serde_json::from_value(data)?)
}

#[cfg(test)] mod tests {
    use super::*;
    use std::{net::TcpListener,io::{Read,Write}};
    #[test] fn outage_preserves_local_database_and_recovery_syncs_pending() {
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("vault.db");
        let mut state=LocalState::default();
        let person=crate::model::Entry::new("person","Offline user");state.edit(person.clone()).unwrap();
        crate::database::save(&path,&state,"local-password").unwrap();
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();let address=listener.local_addr().unwrap();drop(listener);
        let session=Session{url:format!("http://{address}/"),token:"test-only".into(),vault_id:Uuid::new_v4()};
        assert!(synchronize(&mut state,&session).is_err());
        assert_eq!(state.pending.len(),1);assert!(state.vault_id.is_none());
        state=crate::database::load(&path,"local-password").unwrap();
        let listener=TcpListener::bind(address).unwrap();
        let id=person.id;let mut accepted=state.entries[0].clone();accepted.revision=1;
        let body=serde_json::to_string(&crate::model::SyncResponse{entries:vec![accepted],applied:vec![id],conflicts:vec![]}).unwrap();
        let server=std::thread::spawn(move||{
            let (mut socket,_)=listener.accept().unwrap();
            socket.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut request=Vec::new();let mut buf=[0u8;4096];
            loop {
                let n=socket.read(&mut buf).unwrap();assert!(n>0);request.extend_from_slice(&buf[..n]);
                if let Some(end)=request.windows(4).position(|w|w==b"\r\n\r\n") {
                    let headers=String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let size:usize=headers.lines().find_map(|l|l.strip_prefix("content-length:")).unwrap().trim().parse().unwrap();
                    if request.len()>=end+4+size {break;}
                }
            }
            assert!(String::from_utf8_lossy(&request).contains("Offline user"));
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        });
        synchronize(&mut state,&session).unwrap();server.join().unwrap();
        crate::database::save(&path,&state,"local-password").unwrap();
        let reopened=crate::database::load(&path,"local-password").unwrap();
        assert!(reopened.pending.is_empty());assert_eq!(reopened.entries[0].revision,1);
    }
}
