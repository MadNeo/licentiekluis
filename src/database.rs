//! SQLite contains only authenticated ciphertext, never plaintext licence records.
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use zeroize::Zeroizing;
use crate::{model::LocalState, vault};

#[derive(Clone, Serialize, Deserialize)]
pub struct Credentials { pub url:String, pub user:String, pub password:String }
impl Drop for Credentials { fn drop(&mut self) { use zeroize::Zeroize; self.password.zeroize(); } }

fn connect(path:&Path)->Result<Connection> {
    std::fs::create_dir_all(path.parent().context("Ongeldig databasepad")?)?;
    let db=Connection::open(path)?;
    db.busy_timeout(std::time::Duration::from_secs(5))?;
    db.execute_batch("PRAGMA synchronous=FULL; PRAGMA secure_delete=ON;
        CREATE TABLE IF NOT EXISTS encrypted_store (name TEXT PRIMARY KEY, payload BLOB NOT NULL);")?;
    Ok(db)
}
fn write<T:Serialize>(path:&Path,name:&str,value:&T,password:&str)->Result<()> {
    let plain=Zeroizing::new(serde_json::to_vec(value)?);
    let cipher=vault::encrypt(&plain,password)?;
    let mut db=connect(path)?; let tx=db.transaction()?;
    if name=="vault" {
        tx.execute("INSERT OR REPLACE INTO encrypted_store(name,payload) SELECT 'previous',payload FROM encrypted_store WHERE name='vault'",[])?;
    }
    tx.execute("INSERT OR REPLACE INTO encrypted_store(name,payload) VALUES (?1,?2)",params![name,cipher])?;
    tx.commit()?; Ok(())
}
fn read<T:serde::de::DeserializeOwned>(path:&Path,name:&str,password:&str)->Result<Option<T>> {
    let db=connect(path)?;
    let cipher:Option<Vec<u8>>=db.query_row("SELECT payload FROM encrypted_store WHERE name=?1",[name],|r|r.get(0)).optional()?;
    cipher.map(|c|Ok(serde_json::from_slice(&vault::decrypt(&c,password)?)?)).transpose()
}
pub fn exists(path:&Path)->Result<bool> {
    if !path.exists(){return Ok(false);}
    Ok(connect(path)?.query_row("SELECT EXISTS(SELECT 1 FROM encrypted_store WHERE name='vault')",[],|r|r.get(0))?)
}
pub fn load(path:&Path,password:&str)->Result<LocalState> {
    let state:LocalState=read(path,"vault",password)?.context("Geen lokale kluis in database")?;
    state.validate()?;Ok(state)
}
pub fn save(path:&Path,state:&LocalState,password:&str)->Result<()> {
    state.validate()?;write(path,"vault",state,password)
}
pub fn credentials(path:&Path,password:&str)->Result<Option<Credentials>> {read(path,"connection",password)}
pub fn remember(path:&Path,value:&Credentials,password:&str)->Result<()> {write(path,"connection",value,password)}
pub fn forget(path:&Path)->Result<()> {
    connect(path)?.execute("DELETE FROM encrypted_store WHERE name='connection'",[])?;Ok(())
}
/// Import only after successful decrypt; leave the original encrypted file intact.
pub fn migrate(path:&Path,legacy:&Path,password:&str)->Result<()> {
    if !exists(path)? && legacy.exists() {
        let state=vault::load(legacy,password)?;
        save(path,&state,password)?;
    } Ok(())
}

#[cfg(test)] mod tests {
 use super::*;
 #[test] fn encrypted_database_roundtrip_and_restart() {
    let d=tempfile::tempdir().unwrap();let p=d.path().join("vault.db");
    let mut s=LocalState::default();s.edit(crate::model::Entry::new("person","SECRET-PERSON-123")).unwrap();
    save(&p,&s,"test-password").unwrap();
    remember(&p,&Credentials{url:"https://example.test/".into(),user:"account".into(),password:"SECRET-SERVER-PASSWORD".into()},"test-password").unwrap();
    assert_eq!(load(&p,"test-password").unwrap().entries[0].name,"SECRET-PERSON-123");
    assert_eq!(credentials(&p,"test-password").unwrap().unwrap().user,"account");
    let bytes=std::fs::read(&p).unwrap();
    for secret in ["SECRET-PERSON-123","SECRET-SERVER-PASSWORD"] {assert!(!bytes.windows(secret.len()).any(|w|w==secret.as_bytes()));}
    assert!(load(&p,"wrong").is_err());
    assert_eq!(load(&p,"test-password").unwrap().pending.len(),1);
    forget(&p).unwrap();assert!(credentials(&p,"test-password").unwrap().is_none());
 }
 #[test] fn migration_keeps_legacy_and_never_overwrites_database() {
    let d=tempfile::tempdir().unwrap();let old=d.path().join("vault.lkv");let db=d.path().join("vault.db");
    let mut s=LocalState::default();s.edit(crate::model::Entry::new("person","Original")).unwrap();
    vault::save(&old,&s,"test-password").unwrap();
    assert!(migrate(&db,&old,"wrong").is_err());assert!(!exists(&db).unwrap());
    migrate(&db,&old,"test-password").unwrap();assert!(old.exists());
    save(&db,&LocalState::default(),"test-password").unwrap();
    migrate(&db,&old,"test-password").unwrap();assert!(load(&db,"test-password").unwrap().entries.is_empty());
    assert_eq!(vault::load(&old,"test-password").unwrap().entries.len(),1);
 }
}
