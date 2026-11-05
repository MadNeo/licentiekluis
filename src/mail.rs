use anyhow::{ensure,Result};
use serde::{Serialize,Deserialize};
use uuid::Uuid;
use crate::model::Entry;

#[derive(Clone,Serialize)] #[serde(rename_all="camelCase")]
pub struct Selection {pub id:Uuid,pub revision:i64}
#[derive(Clone,Serialize)] #[serde(rename_all="camelCase")]
pub struct Request {pub request_id:Uuid,pub recipient:String,pub subject:String,pub licenses:Vec<Selection>}
#[derive(Deserialize)] pub struct Outcome {pub status:String,pub message:String}
#[derive(Clone)] pub struct Draft {pub request:Request,pub body:String}
pub fn compose(recipient:&str,subject:&str,entries:&[Entry])->Result<Draft>{
    let recipient=recipient.trim();let subject=subject.trim();
    ensure!(recipient.len()<=254 && recipient.matches('@').count()==1 && !recipient.chars().any(|c|c.is_whitespace()||"<>,;\r\n".contains(c)),"Vul precies een e-mailadres in");
    let (local,domain)=recipient.split_once('@').ok_or_else(||anyhow::anyhow!("Ongeldig adres"))?;ensure!(!local.is_empty()&&domain.contains('.')&&!domain.starts_with('.')&&!domain.ends_with('.'),"Ongeldig adres");
    ensure!(!subject.is_empty()&&subject.chars().count()<=150&&!subject.chars().any(|c|c=='\r'||c=='\n'),"Ongeldig onderwerp");
    ensure!(!entries.is_empty()&&entries.len()<=50,"Selecteer 1 tot 50 licenties");
    let mut ids=std::collections::HashSet::new();let mut body="Beste ontvanger,\r\n\r\nHierbij de geselecteerde softwarelicenties.\r\n\r\n".to_string();let mut selections=vec![];
    for (i,e) in entries.iter().enumerate(){ensure!(ids.insert(e.id)&&e.kind=="license"&&!e.deleted&&!e.code.trim().is_empty(),"Selectie bevat een dubbele, verwijderde of lege licentie");body.push_str(&format!("{}. Software: {}\r\n",i+1,e.name));if !e.software_version.is_empty(){body.push_str(&format!("Versie: {}\r\n",e.software_version));}body.push_str(&format!("Licentiecode: {}\r\n\r\n",e.code));selections.push(Selection{id:e.id,revision:e.revision});}
    body.push_str("Bewaar deze gegevens zorgvuldig.\r\nVerstuurd vanuit LicentieKluis.\r\n");
    Ok(Draft{request:Request{request_id:Uuid::new_v4(),recipient:recipient.into(),subject:subject.into(),licenses:selections},body})
}
pub fn mailto(draft:&Draft)->Result<String>{
    // Encode headers/body via URL API; never interpolate into a shell command.
    let mut url=reqwest::Url::parse("mailto:placeholder@example.invalid")?;
    url.set_path(&draft.request.recipient);url.query_pairs_mut().append_pair("subject",&draft.request.subject).append_pair("body",&draft.body);
    let url=url.to_string();ensure!(url.len()<=7000,"De e-mail is te lang voor je mailprogramma-koppeling. Kies minder licenties of gebruik Verzenden via server.");Ok(url)
}
#[cfg(test)] mod tests {
 use super::*;
 fn sample()->Entry{let mut e=Entry::new("license","Software A");e.code="AAA-BBB".into();e.software_version="2026".into();e.notes="PRIVATE NOTE".into();e}
 #[test] fn multiple_software_code_pairs(){let a=sample();let mut b=sample();b.name="Software B".into();b.code="CCC-DDD".into();let d=compose("test@example.org","Licenties",&[a,b]).unwrap();assert!(d.body.contains("Software A")&&d.body.contains("AAA-BBB")&&d.body.contains("Software B")&&d.body.contains("CCC-DDD"));assert!(!d.body.contains("PRIVATE NOTE"));}
 #[test] fn reject_header_injection(){assert!(compose("test@example.org\r\nBcc: other@example.org","x",&[sample()]).is_err());assert!(compose("test@example.org","x\r\nBcc: bad",&[sample()]).is_err());}
 #[test] fn encode_special_characters(){let mut e=sample();e.code="A&B # ?".into();let d=compose("test@example.org","Licenties",&[e]).unwrap();let u=reqwest::Url::parse(&mailto(&d).unwrap()).unwrap();assert!(u.query_pairs().any(|(k,v)|k=="body"&&v.contains("A&B # ?")));}
}
