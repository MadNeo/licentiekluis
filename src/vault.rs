use aes_gcm::{Aes256Gcm,Nonce,aead::{Aead,KeyInit,Payload}};
use anyhow::{ensure,Context,Result};
use rand::RngCore;
use std::{io::Write,path::Path};
use zeroize::Zeroizing;
use crate::model::LocalState;

pub fn encrypt(plain:&[u8],password:&str)->Result<Vec<u8>> {
    let mut salt=[0u8;16];let mut nonce=[0u8;12];rand::rngs::OsRng.fill_bytes(&mut salt);rand::rngs::OsRng.fill_bytes(&mut nonce);
    let mut key=Zeroizing::new([0u8;32]);pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(),&salt,600000,&mut *key);
    let mut header=b"LKV1".to_vec();header.extend(salt);
    let cipher=Aes256Gcm::new_from_slice(&*key).map_err(|_|anyhow::anyhow!("Sleutelfout"))?;
    let encrypted=cipher.encrypt(Nonce::from_slice(&nonce),Payload{msg:plain,aad:&header}).map_err(|_|anyhow::anyhow!("Versleuteling mislukt"))?;
    let split=encrypted.len()-16;header.extend(nonce);header.extend(&encrypted[split..]);header.extend(&encrypted[..split]);Ok(header)
}
pub fn decrypt(data:&[u8],password:&str)->Result<Zeroizing<Vec<u8>>> {
    ensure!(data.len()>=48 && &data[..4]==b"LKV1","Geen ondersteunde kluis");
    let mut key=Zeroizing::new([0u8;32]);pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(),&data[4..20],600000,&mut *key);
    let cipher=Aes256Gcm::new_from_slice(&*key).map_err(|_|anyhow::anyhow!("Sleutelfout"))?;
    let mut body=data[48..].to_vec();body.extend(&data[32..48]);
    let plain=cipher.decrypt(Nonce::from_slice(&data[20..32]),Payload{msg:&body,aad:&data[..20]}).map_err(|_|anyhow::anyhow!("Wachtwoord onjuist of bestand beschadigd"))?;
    Ok(Zeroizing::new(plain))
}
pub fn load(path:&Path,password:&str)->Result<LocalState> {
    let data=std::fs::read(path).context("Kluis lezen mislukt")?;let plain=decrypt(&data,password)?;
    let state:LocalState=serde_json::from_slice(&plain)?;state.validate()?;Ok(state)
}
pub fn save(path:&Path,state:&LocalState,password:&str)->Result<()> {
    state.validate()?;let plain=Zeroizing::new(serde_json::to_vec(state)?);let encrypted=encrypt(&plain,password)?;
    let parent=path.parent().context("Ongeldig pad")?;std::fs::create_dir_all(parent)?;
    let mut temp=tempfile::NamedTempFile::new_in(parent)?;temp.write_all(&encrypted)?;temp.as_file().sync_all()?;
    if path.exists(){std::fs::copy(path,path.with_extension("lkv.bak"))?;}
    temp.persist(path).map_err(|e|e.error).context("Kluis opslaan mislukt")?;Ok(())
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn crypto_roundtrip_and_tamper(){let mut c=encrypt(b"TEST","long-test-password").unwrap();assert_eq!(&**decrypt(&c,"long-test-password").unwrap(),b"TEST");assert!(decrypt(&c,"wrong").is_err());let last=c.len()-1;c[last]^=1;assert!(decrypt(&c,"long-test-password").is_err());}
    #[test] fn randomized_encryption(){assert_ne!(encrypt(b"TEST","pass").unwrap(),encrypt(b"TEST","pass").unwrap());}
    #[test] fn import_dotnet_fixture(){let bytes=include_bytes!("../fixtures/dotnet-vault.lkv");let plain=decrypt(bytes,"test-only-long-passphrase").unwrap();let s:LocalState=serde_json::from_slice(&plain).unwrap();s.validate().unwrap();assert!(s.entries.iter().any(|e|e.name=="Testgebruiker"));assert!(s.entries.iter().any(|e|e.code=="TEST-NOT-REAL"));}
    #[test] fn save_replace_backup(){let d=tempfile::tempdir().unwrap();let p=d.path().join("test.lkv");let mut s=LocalState::default();save(&p,&s,"pass").unwrap();s.edit(crate::model::Entry::new("person","Test")).unwrap();save(&p,&s,"pass").unwrap();assert_eq!(load(&p,"pass").unwrap().entries.len(),1);assert!(p.with_extension("lkv.bak").exists());}
}
