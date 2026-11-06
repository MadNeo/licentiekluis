use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    #[serde(alias="Platform")] pub platform: String,
    #[serde(alias="Device")] pub device: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    #[serde(alias="Id")] pub id: Uuid,
    #[serde(alias="Kind")] pub kind: String,
    #[serde(alias="Name")] pub name: String,
    #[serde(alias="OwnerId")] pub owner_id: Option<Uuid>,
    #[serde(alias="Code")] pub code: String,
    #[serde(alias="SoftwareVersion")] pub software_version: String,
    #[serde(alias="Notes")] pub notes: String,
    #[serde(alias="Expires")] pub expires: Option<chrono::NaiveDate>,
    #[serde(alias="Installations")] pub installations: Vec<Installation>,
    #[serde(alias="Deleted")] pub deleted: bool,
    #[serde(alias="Revision")] pub revision: i64,
    #[serde(alias="MutationId")] pub mutation_id: Uuid,
}
impl Entry {
    pub fn new(kind: &str, name: &str) -> Self { Self { id: Uuid::new_v4(), kind: kind.into(), name:name.into(), owner_id:None, code:String::new(), software_version:String::new(), notes:String::new(), expires:None, installations:vec![], deleted:false, revision:0, mutation_id:Uuid::new_v4() } }
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.id.is_nil() && !self.mutation_id.is_nil() && self.revision>=0, "Ongeldige recordidentiteit");
        ensure!(self.kind=="person" || self.kind=="license", "Ongeldig recordtype");
        ensure!(!self.name.trim().is_empty() && self.name.encode_utf16().count()<=200,"Vul een naam in (maximaal 200 tekens)");
        ensure!(self.code.encode_utf16().count()<=10000 && self.notes.encode_utf16().count()<=20000 && self.software_version.encode_utf16().count()<=200,"Tekst te lang");
        if self.kind=="license" { ensure!(self.owner_id.is_some(),"Selecteer een gebruiker"); }
        ensure!(self.installations.len()<=100,"Maximaal 100 installaties per licentie");
        for i in &self.installations { ensure!(["Windows","Linux","macOS"].contains(&i.platform.as_str()) && i.device.encode_utf16().count()<=200,"Ongeldig platform of apparaatnaam te lang"); }
        Ok(())
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct LocalState {
    #[serde(alias="ServerUrl",default)] pub server_url:String,
    #[serde(alias="VaultId",default)] pub vault_id:Option<Uuid>,
    #[serde(alias="Entries",default)] pub entries:Vec<Entry>,
    #[serde(alias="Pending",default)] pub pending:BTreeMap<Uuid,i64>,
    #[serde(alias="Conflicts",default)] pub conflicts:BTreeMap<Uuid,Entry>,
    #[serde(alias="LastSync",default)] pub last_sync:Option<chrono::DateTime<chrono::FixedOffset>>,
}
impl LocalState {
    pub fn validate(&self)->Result<()> {
        let mut ids=std::collections::HashSet::new();
        for e in &self.entries { e.validate()?; ensure!(ids.insert(e.id),"Dubbele recordidentiteit"); }
        for (id,rev) in &self.pending {ensure!(*rev>=0 && ids.contains(id),"Ongeldige synchronisatiegegevens");}
        for (id,e) in &self.conflicts {e.validate()?;ensure!(*id==e.id && self.pending.contains_key(id),"Ongeldig conflict");}
        for e in self.entries.iter().filter(|e|e.kind=="license"&&!e.deleted) {ensure!(self.entries.iter().any(|p|Some(p.id)==e.owner_id&&p.kind=="person"),"Licentie zonder gebruiker");}
        Ok(())
    }
    pub fn edit(&mut self,mut e:Entry)->Result<()> {
        e.validate()?;
        let revision=self.entries.iter().find(|x|x.id==e.id).map_or(0,|x|x.revision);
        self.pending.entry(e.id).or_insert(revision);
        e.revision=revision;e.mutation_id=Uuid::new_v4();
        self.entries.retain(|x|x.id!=e.id);self.entries.push(e);Ok(())
    }
    pub fn request(&self)->Result<SyncRequest> {
        let vault_id=self.vault_id.ok_or_else(||anyhow::anyhow!("Geen server gekoppeld"))?;
        let mut changes=vec![];
        for (id,base) in &self.pending {let e=self.entries.iter().find(|e|e.id==*id).ok_or_else(||anyhow::anyhow!("Ontbrekende wijziging"))?;changes.push(Change{value:e.clone(),base_revision:*base});}
        Ok(SyncRequest{vault_id,changes})
    }
    pub fn merge(&mut self,response:SyncResponse)->Result<()> {
        for e in &response.entries {e.validate()?;}
        for id in response.applied {self.pending.remove(&id);self.conflicts.remove(&id);}
        for remote in response.entries {
            if self.pending.contains_key(&remote.id){self.conflicts.insert(remote.id,remote);}
            else {self.entries.retain(|e|e.id!=remote.id);self.entries.push(remote);}
        }
        self.last_sync=Some(chrono::Local::now().fixed_offset());self.validate()
    }
    pub fn resolve(&mut self,id:Uuid,local:bool)->Result<()> {
        let remote=self.conflicts.get(&id).ok_or_else(||anyhow::anyhow!("Conflict niet gevonden"))?.clone();
        if local {let e=self.entries.iter_mut().find(|e|e.id==id).ok_or_else(||anyhow::anyhow!("Record ontbreekt"))?;e.revision=remote.revision;e.mutation_id=Uuid::new_v4();self.pending.insert(id,remote.revision);}
        else {self.entries.retain(|e|e.id!=id);self.entries.push(remote);self.pending.remove(&id);}
        self.conflicts.remove(&id);Ok(())
    }
    pub fn bind(&mut self,url:String,id:Uuid)->Result<()> {if self.vault_id.is_some_and(|v|v!=id){bail!("Deze lokale kluis hoort bij een andere server");}self.server_url=url;self.vault_id=Some(id);Ok(())}
}
#[derive(Serialize,Deserialize)] #[serde(rename_all="camelCase")]
pub struct Change {pub value:Entry,pub base_revision:i64}
#[derive(Serialize,Deserialize)] #[serde(rename_all="camelCase")]
pub struct SyncRequest {pub vault_id:Uuid,pub changes:Vec<Change>}
#[derive(Serialize,Deserialize)] #[serde(rename_all="camelCase")]
pub struct SyncResponse {pub entries:Vec<Entry>,pub applied:Vec<Uuid>,pub conflicts:Vec<Uuid>}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn offline_edit_preserves_base_revision(){let mut s=LocalState::default();let mut p=Entry::new("person","A");p.revision=3;s.entries.push(p.clone());p.name="B".into();s.edit(p.clone()).unwrap();p.name="C".into();s.edit(p).unwrap();assert_eq!(s.pending.values().next(),Some(&3));}
    #[test] fn conflict_keeps_both_and_resolves(){let mut s=LocalState::default();let p=Entry::new("person","Local");s.edit(p.clone()).unwrap();let mut r=p.clone();r.name="Remote".into();r.revision=1;s.merge(SyncResponse{entries:vec![r],applied:vec![],conflicts:vec![p.id]}).unwrap();assert_eq!(s.entries[0].name,"Local");assert_eq!(s.conflicts[&p.id].name,"Remote");s.resolve(p.id,true).unwrap();assert_eq!(s.pending[&p.id],1);assert!(s.conflicts.is_empty());}
    #[test] fn accepted_mutation_clears_pending(){let mut s=LocalState::default();let mut p=Entry::new("person","A");s.edit(p.clone()).unwrap();p.revision=1;s.merge(SyncResponse{entries:vec![p.clone()],applied:vec![p.id],conflicts:vec![]}).unwrap();assert!(s.pending.is_empty());}
    #[test] fn server_identity_cannot_change(){let mut s=LocalState::default();s.bind("https://a/".into(),Uuid::new_v4()).unwrap();assert!(s.bind("https://b/".into(),Uuid::new_v4()).is_err());}
    #[test] fn invalid_platform_rejected(){let mut p=Entry::new("license","A");p.owner_id=Some(Uuid::new_v4());p.installations.push(Installation{platform:"bad".into(),device:"PC".into()});assert!(p.validate().is_err());}
}

/// Keep edits made while the network request was in flight. If the server
/// accepted the sent version, base the next mutation on that accepted revision.
/// Otherwise preserve the existing base revision and conflict.
pub fn reconcile(sent:&LocalState,current:&LocalState,mut received:LocalState)->Result<LocalState> {
    for local in &current.entries {
        let previous=sent.entries.iter().find(|e|e.id==local.id);
        if previous.is_some_and(|e|e.mutation_id==local.mutation_id){continue;}
        let mut entry=local.clone();
        let base=if received.conflicts.contains_key(&local.id) {
            *current.pending.get(&local.id).unwrap_or(&local.revision)
        } else {
            received.entries.iter().find(|e|e.id==local.id).map_or(local.revision,|e|e.revision)
        };
        entry.revision=base;
        received.entries.retain(|e|e.id!=local.id);received.entries.push(entry);
        received.pending.insert(local.id,base);
    }
    received.validate()?;Ok(received)
}
#[cfg(test)] mod background_tests {
 use super::*;
 #[test] fn edits_during_sync_survive_and_use_accepted_revision() {
    let mut sent=LocalState::default();let p=Entry::new("person","Sent");sent.edit(p.clone()).unwrap();
    let mut current=sent.clone();let mut edited=current.entries[0].clone();edited.name="New offline edit".into();current.edit(edited).unwrap();
    let mut received=sent.clone();let mut remote=sent.entries[0].clone();remote.revision=7;
    received.merge(SyncResponse{entries:vec![remote],applied:vec![p.id],conflicts:vec![]}).unwrap();
    let result=reconcile(&sent,&current,received).unwrap();
    assert_eq!(result.entries[0].name,"New offline edit");assert_eq!(result.pending[&p.id],7);
    assert_eq!(result.entries[0].mutation_id,current.entries[0].mutation_id);
 }
 #[test] fn concurrent_edit_keeps_server_conflict() {
    let mut sent=LocalState::default();let p=Entry::new("person","Sent");sent.edit(p.clone()).unwrap();
    let mut current=sent.clone();let mut edit=current.entries[0].clone();edit.name="Latest local".into();current.edit(edit).unwrap();
    let mut received=sent.clone();let mut remote=p.clone();remote.name="Other client".into();remote.revision=3;
    received.merge(SyncResponse{entries:vec![remote],applied:vec![],conflicts:vec![p.id]}).unwrap();
    let result=reconcile(&sent,&current,received).unwrap();
    assert_eq!(result.entries[0].name,"Latest local");assert_eq!(result.conflicts[&p.id].name,"Other client");
 }
}

