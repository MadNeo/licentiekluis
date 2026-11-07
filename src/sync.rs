use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::time::Duration;
use crate::model::{LocalState,SyncResponse};
use serde::Deserialize;
use reqwest::blocking::Client;
use uuid::Uuid;

#[derive(Clone)] pub struct Session {pub url:String,pub token:String,pub vault_id:Uuid, server_ca:Option<Vec<u8>>, verify_certificate:bool}
#[derive(Deserialize)] #[serde(rename_all="camelCase")] struct LoginResponse {token:String,vault_id:Uuid}
pub fn server_url(input: &str) -> Result<String> {
    let url = reqwest::Url::parse(input.trim())?;
    ensure!(url.scheme() == "https" && url.username().is_empty() && url.password().is_none()
        && url.path() == "/" && url.query().is_none() && url.fragment().is_none(),
        "Gebruik een HTTPS-adres zonder pad of accountgegevens");
    Ok(url.to_string())
}

pub fn parse_server_ca(bytes: &[u8]) -> Result<(Vec<u8>, String)> {
    let certificate = if bytes.starts_with(b"-----BEGIN CERTIFICATE-----") {
        native_tls::Certificate::from_pem(bytes)
    } else {
        native_tls::Certificate::from_der(bytes)
    }.context("Kies het publieke CA-certificaat van je eigen server (.crt, .cer of .pem)")?;
    let der = certificate.to_der()?;
    let fingerprint = Sha256::digest(&der).iter()
        .map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(":");
    Ok((der, fingerprint))
}

/// Temporary unverified TLS is limited to direct local addresses.
pub fn certificate_verification(url: &str, requested: bool) -> bool {
    let local = reqwest::Url::parse(url).ok().filter(|url| url.scheme() == "https")
        .and_then(|url| url.host_str().map(str::to_owned))
        .is_some_and(|host| {
            if host == "localhost" { return true; }
            match host.trim_start_matches('[').trim_end_matches(']').parse::<std::net::IpAddr>() {
                Ok(std::net::IpAddr::V4(ip)) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
                Ok(std::net::IpAddr::V6(ip)) => ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local(),
                Err(_) => false,
            }
        });
    requested || !local
}

fn client(url: &str, server_ca: Option<&[u8]>, verify_certificate: bool, timeout: u64) -> Result<Client> {
    let verify = certificate_verification(url, verify_certificate);
    let mut builder = Client::builder().connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(timeout)).redirect(reqwest::redirect::Policy::none());
    if !verify {
        builder = builder.danger_accept_invalid_certs(true).danger_accept_invalid_hostnames(true).no_proxy();
    } else if let Some(der) = server_ca {
        builder = builder.tls_built_in_root_certs(false)
            .add_root_certificate(reqwest::Certificate::from_der(der)?);
    }
    Ok(builder.build()?)
}

fn request_error(error: reqwest::Error) -> anyhow::Error {
    let error = anyhow::Error::new(error);
    let detail = format!("{error:#}");
    if ["-2146762487", "-2146762486"].iter().any(|code| detail.contains(code)) {
        return anyhow::anyhow!("Server-CA niet vertrouwd. Kies bij Instellingen het CA-certificaat van je eigen server.");
    }
    if detail.contains("-2146762481") {
        return anyhow::anyhow!("Het serveradres komt niet overeen met het certificaat. Gebruik het HTTPS-adres dat bij de installatie is ingesteld.");
    }
    error
}

pub fn login(url: &str, user: &str, password: &str, server_ca: Option<&[u8]>, verify_certificate: bool) -> Result<Session> {
    let url = server_url(url)?;
    let response = client(&url, server_ca, verify_certificate, 15)?.post(format!("{url}api/login"))
        .json(&serde_json::json!({"username":user,"password":password})).send().map_err(request_error)?;
    ensure!(response.status().is_success(),
        "Aanmelden mislukt (HTTP {}); controleer account of wacht een minuut", response.status());
    let data: LoginResponse = response.json()?;
    let verify_certificate = certificate_verification(&url, verify_certificate);
    Ok(Session { url, token: data.token, vault_id: data.vault_id, server_ca: server_ca.map(<[u8]>::to_vec), verify_certificate })
}
pub fn synchronize(state:&mut LocalState,session:&Session)->Result<()> {
    ensure!(state.conflicts.is_empty(),"Los eerst de conflicten op");let mut next=state.clone();next.bind(session.url.clone(),session.vault_id)?;
    let r=client(&session.url, session.server_ca.as_deref(), session.verify_certificate, 15)?.post(format!("{}api/sync",session.url)).bearer_auth(&session.token).json(&next.request()?).send().map_err(request_error)?;
    ensure!(r.status().is_success(),"Synchronisatie mislukt (HTTP {}). Bij 401 opnieuw aanmelden",r.status());let response:SyncResponse=r.json()?;next.merge(response)?;*state=next;Ok(())
}
pub fn logout(session:&Session){if let Ok(c)=client(&session.url, session.server_ca.as_deref(), session.verify_certificate, 15){let _=c.post(format!("{}api/logout",session.url)).bearer_auth(&session.token).send();}}
pub fn send_mail(session:&Session,request:&crate::mail::Request)->Result<crate::mail::Outcome>{
    let c=client(&session.url, session.server_ca.as_deref(), session.verify_certificate, 45)?;
    let r=c.post(format!("{}api/mail/send",session.url)).bearer_auth(&session.token).json(request).send().map_err(request_error)?;
    let status=r.status();let data:serde_json::Value=r.json()?;
    ensure!(status.is_success(),"{}",data.get("error").and_then(|e|e.as_str()).unwrap_or("Verzending niet bevestigd; controleer ontvangst voordat je opnieuw probeert"));
    Ok(serde_json::from_value(data)?)
}

#[cfg(test)] mod tests {
    use super::*;
    use std::{net::TcpListener,io::{Read,Write}};
    #[test] fn temporary_mode_is_limited_to_local_addresses() {
        for url in ["https://192.168.178.198/", "https://10.0.0.1/", "https://172.16.0.1/",
            "https://127.0.0.1/", "https://[::1]/", "https://[fd00::1]/", "https://localhost/"] {
            assert!(!certificate_verification(url, false));
            assert!(certificate_verification(url, true));
        }
        for url in ["https://example.com/", "https://1.1.1.1/", "https://172.32.0.1/",
            "https://[2606:4700:4700::1111]/", "http://192.168.178.198/", "https://localhost.example.com/", "not a URL"] {
            assert!(certificate_verification(url, false));
        }
    }

    #[test] fn validates_connection_and_rejects_invalid_ca() {
        assert_eq!(server_url(" https://192.168.178.198 ").unwrap(), "https://192.168.178.198/");
        for url in ["http://host.test", "https://host.test/api", "https://user@host.test", "https://host.test/?query=1"] {
            assert!(server_url(url).is_err());
        }
        assert!(parse_server_ca(b"not a certificate").is_err());
    }

    #[test] fn outage_preserves_local_database_and_recovery_syncs_pending() {
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("vault.db");
        let mut state=LocalState::default();
        let person=crate::model::Entry::new("person","Offline user");state.edit(person.clone()).unwrap();
        crate::database::save(&path,&state,"local-password").unwrap();
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();let address=listener.local_addr().unwrap();drop(listener);
        let session=Session{url:format!("http://{address}/"),token:"test-only".into(),vault_id:Uuid::new_v4(),server_ca:None,verify_certificate:true};
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

