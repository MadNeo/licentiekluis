//! Recovery material lives inside the existing encrypted record, never in a sidecar file.
use anyhow::{ensure, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

pub const MAX_PHOTO: usize = 2 * 1024 * 1024;
pub const MAX_TEXT_FILE: usize = 64 * 1024;
pub const MAX_TOTAL: usize = 8 * 1024 * 1024;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Recovery {
    #[serde(alias="Category")] pub category: String,
    #[serde(default,alias="Text")] pub text: String,
    #[serde(default,alias="TextFile")] pub text_file: Option<TextFile>,
    #[serde(default,alias="Photo")] pub photo: Option<Photo>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct TextFile {
    #[serde(alias="Name")] pub name: String,
    #[serde(alias="Text")] pub text: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Photo {
    #[serde(alias="Name")] pub name: String,
    #[serde(alias="MediaType")] pub media_type: String,
    #[serde(alias="Data")] pub data: String,
}
fn filename(name:&str)->bool { !name.trim().is_empty() && name.encode_utf16().count()<=200 && !name.chars().any(|c|c.is_control() || "/\\:".contains(c)) && name!="." && name!=".." }
pub fn photo_type(bytes:&[u8])->Result<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {return Ok("image/png");}
    if bytes.starts_with(&[0xff,0xd8,0xff]) {return Ok("image/jpeg");}
    anyhow::bail!("Kies een PNG- of JPEG-foto (maximaal 2 MB)")
}
impl Recovery {
    pub fn validate(&self,deleted:bool)->Result<()> {
        ensure!(["authenticator","wallet"].contains(&self.category.as_str()),"Kies Authenticator of Crypto-wallet");
        ensure!(self.text.encode_utf16().count()<=20000 && !self.text.contains('\0'),"Hersteltekst ongeldig (maximaal 20.000 tekens)");
        if let Some(f)=&self.text_file {
            ensure!(filename(&f.name)&&f.name.to_lowercase().ends_with(".txt") && f.text.len()<=MAX_TEXT_FILE && !f.text.trim().is_empty() && !f.text.contains('\0'),"Kies een niet-leeg UTF-8-tekstbestand (.txt, maximaal 64 KB)");
        }
        if let Some(p)=&self.photo {
            ensure!(filename(&p.name) && p.data.len()<=MAX_PHOTO.div_ceil(3)*4,"Foto ongeldig of groter dan 2 MB");
            let bytes=STANDARD.decode(&p.data)?;
            ensure!(bytes.len()<=MAX_PHOTO && photo_type(&bytes)?==p.media_type,"Kies een PNG- of JPEG-foto (maximaal 2 MB)");
        }
        ensure!(deleted || !self.text.trim().is_empty() || self.text_file.is_some() || self.photo.is_some(),"Voeg tekst, een tekstbestand of een foto toe. Eén vorm is voldoende");
        Ok(())
    }
    pub fn size(&self)->usize {self.text.len()+self.text_file.as_ref().map_or(0,|f|f.text.len())+self.photo.as_ref().map_or(0,|p|p.data.len())}
}
pub fn validate_total(entries:&[crate::model::Entry])->Result<()> {
    ensure!(entries.iter().filter_map(|e|e.recovery.as_ref()).map(Recovery::size).sum::<usize>()<=MAX_TOTAL,"Herstelgegevens samen maximaal 8 MB. Verwijder een ongebruikte foto of kies een kleinere foto");Ok(())
}

#[cfg(test)] mod tests {
 use super::*;
 use crate::model::{Entry,LocalState};
 fn entry(r:Recovery)->Entry{let mut e=Entry::new("recovery","Synthetic recovery");e.recovery=Some(r);e}
 #[test] fn each_input_alone_and_combination_are_valid(){
  let mut r=Recovery{category:"authenticator".into(),..Default::default()};assert!(r.validate(false).is_err());
  r.text="TEST ONLY 1234".into();r.validate(false).unwrap();r.text.clear();
  r.text_file=Some(TextFile{name:"recovery.txt".into(),text:"TEST ONLY file\nline two".into()});r.validate(false).unwrap();r.text_file=None;
  r.photo=Some(Photo{name:"photo.png".into(),media_type:"image/png".into(),data:STANDARD.encode(include_bytes!("../fixtures/recovery-test.png"))});r.validate(false).unwrap();
  r.text="TEST ONLY".into();r.text_file=Some(TextFile{name:"recovery.txt".into(),text:"TEST ONLY file".into()});r.validate(false).unwrap();
  let mut s=LocalState::default();s.edit(entry(r.clone())).unwrap();let temp=tempfile::tempdir().unwrap();let file=temp.path().join("vault.db");crate::database::save(&file,&s,"test-password-only").unwrap();let read=crate::database::load(&file,"test-password-only").unwrap();assert_eq!(read.entries[0].recovery,Some(r));assert!(!std::fs::read(file).unwrap().windows(9).any(|v|v==b"TEST ONLY"));
 }
 #[test] fn invalid_attachments_and_empty_records_are_rejected(){
  let r=Recovery{category:"wallet".into(),text:"test".into(),..Default::default()};
  for name in ["../secret.txt","x.html","bad\n.txt"]{let mut x=r.clone();x.text_file=Some(TextFile{name:name.into(),text:"test".into()});assert!(x.validate(false).is_err());}
  let mut x=r.clone();x.photo=Some(Photo{name:"photo.png".into(),media_type:"image/png".into(),data:STANDARD.encode(b"<svg onload=bad>")});assert!(x.validate(false).is_err());
  x=r.clone();x.text_file=Some(TextFile{name:"x.txt".into(),text:"x".repeat(MAX_TEXT_FILE+1)});assert!(x.validate(false).is_err());
  let mut person=Entry::new("person","person");person.recovery=Some(r);assert!(person.validate().is_err());
 }
 #[test] fn recovery_backup_conflict_and_tombstone_preserve_data(){
  let mut state=LocalState::default();let local=entry(Recovery{category:"wallet".into(),text:"LOCAL TEST".into(),..Default::default()});state.edit(local.clone()).unwrap();
  let mut remote=local.clone();remote.revision=1;remote.recovery.as_mut().unwrap().text="REMOTE TEST".into();state.merge(crate::model::SyncResponse{entries:vec![remote],applied:vec![],conflicts:vec![local.id]}).unwrap();
  let bytes=crate::vault::encrypt(&serde_json::to_vec(&state).unwrap(),"test-pass").unwrap();let plain=crate::vault::decrypt(&bytes,"test-pass").unwrap();let mut restored:LocalState=serde_json::from_slice(&plain).unwrap();restored.validate().unwrap();assert_eq!(restored.conflicts[&local.id].recovery.as_ref().unwrap().text,"REMOTE TEST");restored.resolve(local.id,false).unwrap();
  let mut deleted=restored.entries[0].clone();deleted.deleted=true;deleted.recovery.as_mut().unwrap().text.clear();restored.edit(deleted).unwrap();restored.validate().unwrap();
 }
}
