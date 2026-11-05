use anyhow::{ensure,Result};
use crate::model::{LocalState,SyncResponse};
use serde::Deserialize;
use reqwest::blocking::Client;
use uuid::Uuid;

#[derive(Clone)] pub struct Session {pub url:String,pub token:String,pub vault_id:Uuid}
#[derive(Deserialize)] #[serde(rename_all="camelCase")] struct LoginResponse {token:String,vault_id:Uuid}
fn client()->Result<Client>{Ok(Client::builder().timeout(std::time::Duration::from_secs(25)).redirect(reqwest::redirect::Policy::none()).build()?)}
pub fn login(url:&str,user:&str,password:&str)->Result<Session>{
    let u=reqwest::Url::parse(url)?;ensure!(u.scheme()=="https"&&u.username().is_empty()&&u.password().is_none()&&u.path()=="/"&&u.query().is_none()&&u.fragment().is_none(),"Gebruik een HTTPS-adres zonder pad of accountgegevens");
    let url=u.to_string();let r=client()?.post(format!("{}api/login",url)).json(&serde_json::json!({"username":user,"password":password})).send()?;
    ensure!(r.status().is_success(),"Aanmelden mislukt (HTTP {}); controleer account of wacht een minuut",r.status());let data:LoginResponse=r.json()?;Ok(Session{url,token:data.token,vault_id:data.vault_id})
}
pub fn synchronize(state:&mut LocalState,session:&Session)->Result<()> {
    ensure!(state.conflicts.is_empty(),"Los eerst de conflicten op");state.bind(session.url.clone(),session.vault_id)?;
    let r=client()?.post(format!("{}api/sync",session.url)).bearer_auth(&session.token).json(&state.request()?).send()?;
    ensure!(r.status().is_success(),"Synchronisatie mislukt (HTTP {}). Bij 401 opnieuw aanmelden",r.status());let response:SyncResponse=r.json()?;state.merge(response)
}
pub fn logout(session:&Session){if let Ok(c)=client(){let _=c.post(format!("{}api/logout",session.url)).bearer_auth(&session.token).send();}}
pub fn send_mail(session:&Session,request:&crate::mail::Request)->Result<crate::mail::Outcome>{
    let c=Client::builder().timeout(std::time::Duration::from_secs(45)).redirect(reqwest::redirect::Policy::none()).build()?;
    let r=c.post(format!("{}api/mail/send",session.url)).bearer_auth(&session.token).json(request).send()?;
    let status=r.status();let data:serde_json::Value=r.json()?;
    ensure!(status.is_success(),"{}",data.get("error").and_then(|e|e.as_str()).unwrap_or("Verzending niet bevestigd; controleer ontvangst voordat je opnieuw probeert"));
    Ok(serde_json::from_value(data)?)
}
