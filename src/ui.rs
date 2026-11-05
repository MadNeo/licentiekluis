use anyhow::{ensure,Context,Result};
use native_windows_gui as nwg;
use licentiekluis_native::{model::{Entry,Installation,LocalState},vault,sync::{self,Session},mail};
use std::{cell::{Cell,RefCell},path::PathBuf,rc::Rc,sync::{Arc,Mutex},time::{Duration,Instant}};
use uuid::Uuid;
use zeroize::Zeroizing;

const PW:usize=0;const REPEAT:usize=1;const SEARCH:usize=2;const PERSON:usize=3;const NAME:usize=4;const CODE:usize=5;const VERSION:usize=6;const EXPIRY:usize=7;const DEVICE:usize=8;const URL:usize=9;const USER:usize=10;const SERVERPW:usize=11;const BACKUPPW:usize=12;
const MAIL:usize=18;
const UNLOCK:usize=0;const NEW:usize=1;const SAVE:usize=2;const DELETE:usize=3;const COPY:usize=4;const SHOW:usize=5;const ADDPERSON:usize=6;const RENAME:usize=7;const DELPERSON:usize=8;const ADDINSTALL:usize=9;const DELINSTALL:usize=10;const LOGIN:usize=11;const SYNC:usize=12;const LOGOUT:usize=13;const CONFLICT:usize=14;const BACKUP:usize=15;const RESTORE:usize=16;const LOCK:usize=17;
type WorkResult=std::result::Result<(LocalState,Session),String>;

struct App {
    mail_window:nwg::Window,mail_licenses:nwg::ListBox<String>,mail_to:nwg::TextInput,mail_subject:nwg::TextInput,mail_preview:nwg::TextBox,mail_buttons:Vec<nwg::Button>,mail_entries:RefCell<Vec<Entry>>,mail_draft:RefCell<Option<mail::Draft>>,mail_result:Arc<Mutex<Option<std::result::Result<mail::Outcome,String>>>>,
    window:nwg::Window, labels:Vec<nwg::Label>, fields:Vec<nwg::TextInput>, buttons:Vec<nwg::Button>,
    people:nwg::ListBox<String>, licenses:nwg::ListBox<String>, owner:nwg::ComboBox<String>, platform:nwg::ComboBox<String>, filter:nwg::ComboBox<String>,
    installations:nwg::ListBox<String>,notes:nwg::TextBox,status:nwg::Label,notice:nwg::Notice,timer:nwg::AnimationTimer,
    state:RefCell<Option<LocalState>>,password:RefCell<Option<Zeroizing<String>>>,session:RefCell<Option<Session>>,
    selected:Cell<Option<Uuid>>,people_ids:RefCell<Vec<Uuid>>,license_ids:RefCell<Vec<Uuid>>,draft_installs:RefCell<Vec<Installation>>,
    loading:Cell<bool>,dirty:Cell<bool>,busy:Cell<bool>,modal:Cell<bool>,path:PathBuf,
    result:Arc<Mutex<Option<WorkResult>>>,last_sync:Cell<Instant>,copied:RefCell<Option<(Zeroizing<String>,Instant)>>,revealed:Cell<Option<Instant>>,
}
pub fn run()->Result<()> {
    nwg::init()?;nwg::Font::set_global_family("Segoe UI")?;
    let dir=PathBuf::from(std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA ontbreekt")?).join("LicentieKluisRust");std::fs::create_dir_all(&dir)?;
    let lock=std::fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(dir.join("instance.lock"))?;
    fs2::FileExt::try_lock_exclusive(&lock).context("LicentieKluis Rust is al geopend")?;
    let app=Rc::new(App::build(dir.join("vault.lkv"))?);let weak=Rc::downgrade(&app);
    let handler=nwg::full_bind_event_handler(&app.window.handle,move|event,_,handle|{if let Some(app)=weak.upgrade(){app.event(event,handle);}});
    app.status.set_text("Vergrendeld. Nieuwe kluis: kies minimaal 14 tekens en herhaal het wachtwoord.");
    let weak=Rc::downgrade(&app);let mail_handler=nwg::full_bind_event_handler(&app.mail_window.handle,move|event,_,handle|{if let Some(app)=weak.upgrade(){app.mail_event(event,handle);}});
    nwg::dispatch_thread_events();nwg::unbind_event_handler(&mail_handler);nwg::unbind_event_handler(&handler);drop(app);drop(lock);Ok(())
}
impl App {
    fn build(path:PathBuf)->Result<Self> {
        let mut window=nwg::Window::default();nwg::Window::builder().size((1200,800)).position((80,40)).title("LicentieKluis Rust · native Windows · 0.3.0 broncodepreview").build(&mut window)?;
        let mut labels=vec![];let mut label=|text:&str,x,y,w|->Result<()>{let mut l=nwg::Label::default();nwg::Label::builder().text(text).parent(&window).position((x,y)).size((w,22)).build(&mut l)?;labels.push(l);Ok(())};
        label("LICENTIEKLUIS   /   Lokaal werken, veilig synchroniseren",20,14,900)?;
        label("Lokaal kluiswachtwoord",20,45,200)?;label("Herhaal (alleen nieuwe kluis)",240,45,210)?;
        label("GEBRUIKERS",20,124,220)?;label("LICENTIES",260,124,360)?;label("LICENTIE BEWERKEN",650,124,500)?;
        label("Gebruikersnaam toevoegen / wijzigen",20,474,230)?;
        for (text,y) in [("Software",150),("Licentiecode",210),("Versie",270),("Einddatum (jjjj-mm-dd, of leeg)",330)]{label(text,650,y,250)?;}
        label("Toegewezen aan",920,150,250)?;label("Installaties: platform en apparaat",650,390,500)?;label("Opmerkingen",650,548,500)?;
        label("HTTPS-serveradres",20,630,380)?;label("Serveraccount",420,630,210)?;label("Serverwachtwoord",650,630,250)?;
        label("Wachtwoord back-up (alleen voor herstellen)",650,727,340)?;
        let positions=[(20,68,205),(240,68,205),(260,95,230),(20,500,220),(650,174,250),(650,234,400),(650,294,250),(650,354,250),(820,417,225),(20,654,380),(420,654,210),(650,654,250),(650,752,250)];
        let mut fields=vec![];
        for (i,(x,y,w)) in positions.into_iter().enumerate(){let mut f=nwg::TextInput::default();nwg::TextInput::builder().parent(&window).position((x,y)).size((w,26)).password(if [PW,REPEAT,CODE,SERVERPW,BACKUPPW].contains(&i){Some('*')}else{None}).build(&mut f)?;fields.push(f);}
        let specifications=[("Ontgrendelen",465,67,140),("Nieuwe licentie",260,586,115),("Opslaan",650,600,100),("Verwijderen",385,586,110),("Kopiëren",1060,234,110),("Code tonen",1060,270,110),("Toevoegen",20,538,105),("Hernoemen",135,538,105),("Verwijderen",20,578,105),("+",1055,417,50),("−",1115,417,50),("Aanmelden",920,654,125),("Synchroniseren",20,700,145),("Afmelden",175,700,110),("Conflicten",295,700,110),("Back-up",700,67,110),("Herstellen",820,67,110),("Vergrendelen",940,67,150),("Licenties mailen",420,700,185)];
        let mut buttons=vec![];for (text,x,y,w) in specifications{let mut b=nwg::Button::default();nwg::Button::builder().text(text).parent(&window).position((x,y)).size((w,29)).build(&mut b)?;buttons.push(b);}
        let mut people=nwg::ListBox::default();nwg::ListBox::builder().parent(&window).position((20,150)).size((220,310)).build(&mut people)?;
        let mut licenses=nwg::ListBox::default();nwg::ListBox::builder().parent(&window).position((260,150)).size((360,420)).build(&mut licenses)?;
        let mut owner=nwg::ComboBox::default();nwg::ComboBox::builder().parent(&window).position((920,174)).size((250,200)).build(&mut owner)?;
        let mut platform=nwg::ComboBox::default();nwg::ComboBox::builder().parent(&window).position((650,417)).size((155,200)).collection(vec!["Windows".to_string(),"Linux".into(),"macOS".into()]).selected_index(Some(0)).build(&mut platform)?;
        let mut filter=nwg::ComboBox::default();nwg::ComboBox::builder().parent(&window).position((500,95)).size((120,200)).collection(vec!["Alle platforms".to_string(),"Windows".into(),"Linux".into(),"macOS".into()]).selected_index(Some(0)).build(&mut filter)?;
        let mut installations=nwg::ListBox::default();nwg::ListBox::builder().parent(&window).position((650,454)).size((520,82)).build(&mut installations)?;
        let mut notes=nwg::TextBox::default();nwg::TextBox::builder().parent(&window).position((650,571)).size((395,27)).build(&mut notes)?;
        let mut status=nwg::Label::default();nwg::Label::builder().parent(&window).position((20,751)).size((600,40)).build(&mut status)?;
        let mut notice=nwg::Notice::default();nwg::Notice::builder().parent(&window).build(&mut notice)?;
        let mut timer=nwg::AnimationTimer::default();nwg::AnimationTimer::builder().parent(&window).interval(Duration::from_secs(1)).active(true).build(&mut timer)?;
        let mut mail_window=nwg::Window::default();nwg::Window::builder().size((900,700)).position((140,70)).title("Licenties per e-mail delen").build(&mut mail_window)?;mail_window.set_visible(false);
        let mut mail_licenses=nwg::ListBox::default();nwg::ListBox::builder().parent(&mail_window).position((20,45)).size((340,500)).flags(nwg::ListBoxFlags::VISIBLE | nwg::ListBoxFlags::MULTI_SELECT | nwg::ListBoxFlags::TAB_STOP).build(&mut mail_licenses)?;
        for (text,x,y,w) in [("Klik meerdere licenties aan (maximaal 50)",20,15,350),("Ontvanger (een e-mailadres)",390,15,470),("Onderwerp",390,85,470),("Voorbeeld van de e-mail",390,155,470)]{let mut l=nwg::Label::default();nwg::Label::builder().text(text).parent(&mail_window).position((x,y)).size((w,24)).build(&mut l)?;labels.push(l);}
        let mut mail_to=nwg::TextInput::default();nwg::TextInput::builder().parent(&mail_window).position((390,45)).size((470,28)).build(&mut mail_to)?;
        let mut mail_subject=nwg::TextInput::default();nwg::TextInput::builder().parent(&mail_window).position((390,115)).size((470,28)).text("Softwarelicenties").build(&mut mail_subject)?;
        let mut mail_preview=nwg::TextBox::default();nwg::TextBox::builder().parent(&mail_window).position((390,185)).size((470,360)).readonly(true).build(&mut mail_preview)?;
        let mut mail_buttons=vec![];for (text,x,y,w) in [("Voorbeeld maken",20,570,170),("Via server versturen",200,570,200),("Open eigen mailprogramma",410,570,260),("Sluiten",690,570,170),("Alles selecteren",20,610,170),("Selectie wissen",200,610,170)]{let mut b=nwg::Button::default();nwg::Button::builder().parent(&mail_window).position((x,y)).size((w,32)).text(text).build(&mut b)?;mail_buttons.push(b);}
        Ok(Self{mail_window,mail_licenses,mail_to,mail_subject,mail_preview,mail_buttons,mail_entries:RefCell::new(vec![]),mail_draft:RefCell::new(None),mail_result:Arc::new(Mutex::new(None)),window,labels,fields,buttons,people,licenses,owner,platform,filter,installations,notes,status,notice,timer,state:RefCell::new(None),password:RefCell::new(None),session:RefCell::new(None),selected:Cell::new(None),people_ids:RefCell::new(vec![]),license_ids:RefCell::new(vec![]),draft_installs:RefCell::new(vec![]),loading:Cell::new(false),dirty:Cell::new(false),busy:Cell::new(false),modal:Cell::new(false),path,result:Arc::new(Mutex::new(None)),last_sync:Cell::new(Instant::now()),copied:RefCell::new(None),revealed:Cell::new(None)})
    }
    fn error(&self,e:anyhow::Error){self.modal.set(true);nwg::modal_error_message(&self.window,"Bewerking niet voltooid",&format!("{e:#}"));self.modal.set(false);}
    fn confirm(&self,text:&str)->bool{self.modal.set(true);let result=nwg::modal_message(&self.window,&nwg::MessageParams{title:"LicentieKluis",content:text,buttons:nwg::MessageButtons::YesNo,icons:nwg::MessageIcons::Warning});self.modal.set(false);result==nwg::MessageChoice::Yes}
    fn snapshot(&self)->Result<LocalState>{self.state.borrow().clone().context("Ontgrendel eerst de kluis")}
    fn commit(&self,next:LocalState)->Result<()>{let p=self.password.borrow();vault::save(&self.path,&next,p.as_ref().context("Kluis vergrendeld")?)?;drop(p);*self.state.borrow_mut()=Some(next);self.refresh();Ok(())}
    fn event(&self,event:nwg::Event,handle:nwg::ControlHandle){
        if event==nwg::Event::OnNotice && handle==self.notice.handle {if self.mail_result.lock().map(|s|s.is_some()).unwrap_or(false){self.finish_mail();}else{self.finish_work();}return;}
        if event==nwg::Event::OnTimerTick && handle==self.timer.handle {self.tick();return;}
        if self.busy.get()||self.loading.get(){return;}
        if event==nwg::Event::OnWindowClose {if !self.dirty.get()||self.confirm("Niet-opgeslagen invoer verwerpen en sluiten?"){self.clear_clipboard();nwg::stop_thread_dispatch();}return;}
        let result=(||->Result<()>{
            if event==nwg::Event::OnButtonClick {if let Some(i)=self.buttons.iter().position(|b|b.handle==handle){return self.action(i);}}
            if event==nwg::Event::OnListBoxSelect && handle==self.people.handle {if self.discard(){self.render_list();}return Ok(());}
            if event==nwg::Event::OnListBoxSelect && handle==self.licenses.handle {if self.discard(){let id=self.licenses.selection().and_then(|i|self.license_ids.borrow().get(i).copied());self.fill(id);}return Ok(());}
            if event==nwg::Event::OnTextInput && handle==self.fields[SEARCH].handle {if !self.dirty.get(){self.render_list();}return Ok(());}
            if event==nwg::Event::OnComboxBoxSelection && handle==self.filter.handle {if self.discard(){self.render_list();}return Ok(());}
            if (event==nwg::Event::OnTextInput && (self.fields[NAME..=EXPIRY].iter().any(|x|x.handle==handle)||handle==self.notes.handle)) || (event==nwg::Event::OnComboxBoxSelection && handle==self.owner.handle){if self.state.borrow().is_some(){self.dirty.set(true);}}
            Ok(())
        })();if let Err(e)=result{self.error(e);}
    }
    fn discard(&self)->bool{!self.dirty.get()||self.confirm("Niet-opgeslagen licentie-invoer verwerpen?")}
    fn action(&self,i:usize)->Result<()> {
        if i==UNLOCK{return self.unlock();}
        ensure!(self.state.borrow().is_some(),"Ontgrendel eerst de kluis");
        match i {
            MAIL=>self.open_mail()?,
            NEW=>{if self.discard(){self.fill(None);}},SAVE=>self.save_license()?,DELETE=>self.delete_license()?,
            COPY=>{let code=self.fields[CODE].text();clipboard_win::set_clipboard_string(&code)?;*self.copied.borrow_mut()=Some((Zeroizing::new(code),Instant::now()));},
            SHOW=>{self.fields[CODE].set_password_char(None);self.revealed.set(Some(Instant::now()));},
            ADDPERSON|RENAME|DELPERSON=>{ensure!(!self.dirty.get(),"Sla eerst de licentie-invoer op");self.edit_person(i)?;},
            ADDINSTALL=>{let p=self.platform.selection_string().context("Kies een platform")?;self.draft_installs.borrow_mut().push(Installation{platform:p,device:self.fields[DEVICE].text()});self.render_installs();self.dirty.set(true);},
            DELINSTALL=>{if let Some(index)=self.installations.selection(){self.draft_installs.borrow_mut().remove(index);self.render_installs();self.dirty.set(true);}},
            LOGIN=>self.begin_work(true)?,SYNC=>self.begin_work(false)?,
            LOGOUT=>{if let Some(s)=self.session.borrow_mut().take(){std::thread::spawn(move||sync::logout(&s));}self.status.set_text("Afgemeld. Lokaal werken blijft beschikbaar.");},
            CONFLICT=>{ensure!(!self.dirty.get(),"Sla eerst de licentie-invoer op");self.resolve()?;},BACKUP=>self.backup(false)?,RESTORE=>{ensure!(!self.dirty.get(),"Sla eerst de licentie-invoer op");self.backup(true)?;},
            LOCK=>{if self.discard(){self.clear_clipboard();self.fields[CODE].set_text("");self.fields[SERVERPW].set_text("");*self.state.borrow_mut()=None;*self.password.borrow_mut()=None;if let Some(s)=self.session.borrow_mut().take(){std::thread::spawn(move||sync::logout(&s));}self.fill(None);self.people.clear();self.licenses.clear();self.owner.set_collection(vec![]);self.fields[PERSON].set_text("");self.status.set_text("Vergrendeld");}},_=>{}
        }Ok(())
    }
    fn unlock(&self)->Result<()> {
        ensure!(self.state.borrow().is_none(),"De kluis is al geopend");let password=Zeroizing::new(self.fields[PW].text());
        let state=if self.path.exists(){vault::load(&self.path,&password)?}else{ensure!(password.chars().count()>=14 && *password==self.fields[REPEAT].text(),"Kies minimaal 14 tekens en herhaal hetzelfde wachtwoord");let s=LocalState::default();vault::save(&self.path,&s,&password)?;s};
        self.fields[PW].set_text("");self.fields[REPEAT].set_text("");self.fields[URL].set_text(&state.server_url);*self.password.borrow_mut()=Some(password);*self.state.borrow_mut()=Some(state);self.refresh();Ok(())
    }
    fn refresh(&self){
        self.loading.set(true);if let Some(s)=self.state.borrow().as_ref(){let mut people:Vec<_>=s.entries.iter().filter(|e|e.kind=="person"&&!e.deleted).collect();people.sort_by_key(|p|p.name.to_lowercase());*self.people_ids.borrow_mut()=people.iter().map(|e|e.id).collect();let names:Vec<String>=people.iter().map(|p|p.name.clone()).collect();let mut list=vec!["Alle gebruikers".into()];list.extend(names.clone());let selected=self.people.selection();self.people.set_collection(list);self.people.set_selection(selected.or(Some(0)));self.owner.set_collection(names);self.status.set_text(&format!("{} wijzigingen · {} conflicten · Laatste sync: {}",s.pending.len(),s.conflicts.len(),s.last_sync.map(|d|d.format("%d-%m-%Y %H:%M").to_string()).unwrap_or_else(||"nog niet".into())));}
        self.loading.set(false);self.render_list();
    }
    fn render_list(&self){
        self.loading.set(true);if let Some(s)=self.state.borrow().as_ref(){let owner=self.people.selection().filter(|i|*i>0).and_then(|i|self.people_ids.borrow().get(i-1).copied());let q=self.fields[SEARCH].text().to_lowercase();let filter=self.filter.selection_string().unwrap_or_default();let mut licenses:Vec<_>=s.entries.iter().filter(|e|e.kind=="license"&&!e.deleted&&(owner.is_none()||e.owner_id==owner)&&(filter=="Alle platforms"||filter.is_empty()||e.installations.iter().any(|i|i.platform==filter))&&format!("{} {}",e.name,e.installations.iter().map(|i|i.device.as_str()).collect::<Vec<_>>().join(" ")).to_lowercase().contains(&q)).collect();licenses.sort_by_key(|e|e.name.to_lowercase());*self.license_ids.borrow_mut()=licenses.iter().map(|e|e.id).collect();self.licenses.set_collection(licenses.iter().map(|e|format!("{}{}  ·  {}",if s.conflicts.contains_key(&e.id){"! "}else{""},e.name,e.installations.iter().map(|i|i.platform.as_str()).collect::<Vec<_>>().join(", "))).collect());}
        self.loading.set(false);let id=self.selected.get().filter(|id|self.license_ids.borrow().contains(id));if let Some(id)=id{self.licenses.set_selection(self.license_ids.borrow().iter().position(|x|*x==id));}self.fill(id);
    }
    fn fill(&self,id:Option<Uuid>){
        self.loading.set(true);self.selected.set(id);let e=self.state.borrow().as_ref().and_then(|s|s.entries.iter().find(|e|Some(e.id)==id).cloned());
        for f in &self.fields[NAME..=EXPIRY]{f.set_text("");}self.notes.set_text("");self.fields[CODE].set_password_char(Some('*'));self.revealed.set(None);self.draft_installs.borrow_mut().clear();self.owner.set_selection(Some(0));
        if let Some(e)=e {self.fields[NAME].set_text(&e.name);self.fields[CODE].set_text(&e.code);self.fields[VERSION].set_text(&e.software_version);self.fields[EXPIRY].set_text(&e.expires.map(|d|d.to_string()).unwrap_or_default());self.notes.set_text(&e.notes);self.owner.set_selection(self.people_ids.borrow().iter().position(|p|Some(*p)==e.owner_id));*self.draft_installs.borrow_mut()=e.installations;}
        self.render_installs();self.dirty.set(false);self.loading.set(false);
    }
    fn render_installs(&self){self.installations.set_collection(self.draft_installs.borrow().iter().map(|i|format!("{}  |  {}",i.platform,i.device)).collect());}
    fn save_license(&self)->Result<()>{let mut s=self.snapshot()?;let mut e=self.selected.get().and_then(|id|s.entries.iter().find(|e|e.id==id).cloned()).unwrap_or_else(||Entry::new("license",""));e.name=self.fields[NAME].text().trim().into();e.code=self.fields[CODE].text();e.software_version=self.fields[VERSION].text();e.owner_id=self.owner.selection().and_then(|i|self.people_ids.borrow().get(i).copied());let expiry=self.fields[EXPIRY].text();e.expires=if expiry.trim().is_empty(){None}else{Some(chrono::NaiveDate::parse_from_str(expiry.trim(),"%Y-%m-%d").context("Gebruik jjjj-mm-dd")?)};e.notes=self.notes.text();e.installations=self.draft_installs.borrow().clone();let id=e.id;s.edit(e)?;self.selected.set(Some(id));self.commit(s)}
    fn delete_license(&self)->Result<()>{let mut s=self.snapshot()?;let id=self.selected.get().context("Selecteer een licentie")?;if !self.confirm("Deze licentie verwijderen, ook op de server na synchronisatie?"){return Ok(());}let mut e=s.entries.iter().find(|e|e.id==id).context("Record ontbreekt")?.clone();e.deleted=true;s.edit(e)?;self.selected.set(None);self.commit(s)}
    fn edit_person(&self,action:usize)->Result<()>{let mut s=self.snapshot()?;let id=self.people.selection().filter(|i|*i>0).and_then(|i|self.people_ids.borrow().get(i-1).copied());let mut p=if action==ADDPERSON{Entry::new("person","")}else{s.entries.iter().find(|e|Some(e.id)==id).context("Selecteer eerst een gebruiker")?.clone()};if action==DELPERSON{ensure!(!s.entries.iter().any(|e|e.kind=="license"&&!e.deleted&&e.owner_id==Some(p.id)),"Verplaats of verwijder eerst de licenties van deze gebruiker");if !self.confirm("Gebruiker verwijderen?"){return Ok(());}p.deleted=true;}else{p.name=self.fields[PERSON].text().trim().into();ensure!(!s.entries.iter().any(|e|e.kind=="person"&&!e.deleted&&e.id!=p.id&&e.name.to_lowercase()==p.name.to_lowercase()),"Deze gebruiker bestaat al");}s.edit(p)?;self.commit(s)}
    fn begin_work(&self,login:bool)->Result<()> {
        ensure!(!self.dirty.get(),"Sla eerst de licentie-invoer op voordat je synchroniseert");let state=self.snapshot()?;ensure!(state.conflicts.is_empty(),"Los eerst de conflicten op");
        if login&&state.vault_id.is_none()&&!self.confirm("Deze kluis koppelen? Alle lokale gebruikers en licenties worden naar deze server verzonden. Het beheeraccount kan de hele kluis zien."){return Ok(());}
        let current=self.session.borrow().clone();if !login{ensure!(current.is_some(),"Meld eerst aan");}
        let url=self.fields[URL].text().trim().trim_end_matches('/').to_string()+"/";let user=self.fields[USER].text();let password=Zeroizing::new(self.fields[SERVERPW].text());self.fields[SERVERPW].set_text("");
        let result=self.result.clone();let sender=self.notice.sender();self.busy.set(true);self.window.set_enabled(false);self.status.set_text("Verbinding maken en synchroniseren…");
        std::thread::spawn(move||{let work=(||->Result<(LocalState,Session)>{let session=if login{sync::login(&url,&user,&password)?}else{current.context("Geen sessie")?};let mut next=state;sync::synchronize(&mut next,&session)?;Ok((next,session))})();if let Ok(mut slot)=result.lock(){*slot=Some(work.map_err(|e|format!("{e:#}")));}sender.notice();});Ok(())
    }
    fn finish_work(&self){self.window.set_enabled(true);self.busy.set(false);self.last_sync.set(Instant::now());let result=self.result.lock().ok().and_then(|mut s|s.take());if let Some(result)=result {match result {Ok((s,session))=>{match self.commit(s){Ok(())=>{*self.session.borrow_mut()=Some(session);},Err(e)=>self.error(e)}},Err(e)=>{*self.session.borrow_mut()=None;self.status.set_text("Sync mislukt; lokaal opgeslagen wijzigingen blijven behouden. Meld opnieuw aan.");self.error(anyhow::anyhow!(e));}}}}
    fn resolve(&self)->Result<()>{let mut s=self.snapshot()?;let ids:Vec<_>=s.conflicts.keys().copied().collect();if ids.is_empty(){self.status.set_text("Geen conflicten");return Ok(());}for id in ids{let local=s.entries.iter().find(|e|e.id==id).context("Record ontbreekt")?;let remote=&s.conflicts[&id];let describe=|e:&Entry|format!("{}\nVersie: {}\nVerwijderd: {}\nInstallaties: {}\nOpmerkingen: {}",e.name,e.software_version,e.deleted,e.installations.iter().map(|i|format!("{} / {}",i.platform,i.device)).collect::<Vec<_>>().join(", "),e.notes);let text=format!("LOKAAL\n{}\n\nSERVER\n{}\n\nCode verschilt: {}\n\nJa: lokale versie behouden\nNee: serverversie overnemen\nAnnuleren: later beslissen",describe(local),describe(remote),local.code!=remote.code);self.modal.set(true);let answer=nwg::modal_message(&self.window,&nwg::MessageParams{title:"Synchronisatieconflict",content:&text,buttons:nwg::MessageButtons::YesNoCancel,icons:nwg::MessageIcons::Warning});self.modal.set(false);if answer==nwg::MessageChoice::Cancel{break;}s.resolve(id,answer==nwg::MessageChoice::Yes)?;}self.commit(s)}
    fn backup(&self,restore:bool)->Result<()>{let mut dialog=nwg::FileDialog::default();nwg::FileDialog::builder().title(if restore{"Back-up herstellen (ook .NET 0.1)"}else{"Versleutelde back-up opslaan"}).action(if restore{nwg::FileDialogAction::Open}else{nwg::FileDialogAction::Save}).filters("Kluis(*.lkv)").build(&mut dialog)?;self.modal.set(true);let accepted=dialog.run(Some(&self.window));self.modal.set(false);if !accepted{return Ok(());}let path=PathBuf::from(dialog.get_selected_item()?);ensure!(path!=self.path,"Kies een andere locatie dan de actieve kluis");if restore{let password=Zeroizing::new(self.fields[BACKUPPW].text());self.fields[BACKUPPW].set_text("");let s=vault::load(&path,&password)?;if !self.confirm("De huidige lokale kluis vervangen? Er wordt eerst een herstelkopie gemaakt. Gebruik daarna alleen deze client voor nieuwe wijzigingen."){return Ok(());}let before=self.path.with_file_name(format!("before-restore-{}.lkv",chrono::Local::now().format("%Y%m%d-%H%M%S")));vault::save(&before,&self.snapshot()?,self.password.borrow().as_ref().context("Vergrendeld")?)?;*self.session.borrow_mut()=None;self.fields[URL].set_text(&s.server_url);self.commit(s)?;}else{vault::save(&path,&self.snapshot()?,self.password.borrow().as_ref().context("Vergrendeld")?)?;self.status.set_text("Back-up opgeslagen; bewaar het lokale kluiswachtwoord voor herstel.");}Ok(())}
    fn open_mail(&self)->Result<()> {
        ensure!(!self.dirty.get(),"Sla eerst de licentie-invoer op");let s=self.snapshot()?;
        let owner=self.people.selection().filter(|i|*i>0).and_then(|i|self.people_ids.borrow().get(i-1).copied());
        let mut entries:Vec<_>=s.entries.into_iter().filter(|e|e.kind=="license"&&!e.deleted&&(owner.is_none()||e.owner_id==owner)).collect();entries.sort_by_key(|e|e.name.to_lowercase());ensure!(!entries.is_empty(),"Geen licenties bij deze gebruiker");
        self.mail_licenses.set_collection(entries.iter().map(|e|format!("{}  |  versie {}",e.name,e.software_version)).collect());*self.mail_entries.borrow_mut()=entries;*self.mail_draft.borrow_mut()=None;self.mail_preview.set_text("");self.modal.set(true);self.window.set_enabled(false);self.mail_window.set_visible(true);
    }
    fn close_mail(&self){self.mail_window.set_visible(false);self.window.set_enabled(true);self.modal.set(false);self.mail_preview.set_text("");self.mail_to.set_text("");self.mail_entries.borrow_mut().clear();self.mail_licenses.clear();*self.mail_draft.borrow_mut()=None;}
    fn mail_event(&self,event:nwg::Event,handle:nwg::ControlHandle){
        if self.busy.get(){return;}if event==nwg::Event::OnWindowClose{self.close_mail();return;}
        if event==nwg::Event::OnListBoxSelect||(event==nwg::Event::OnTextInput&&(handle==self.mail_to.handle||handle==self.mail_subject.handle)){*self.mail_draft.borrow_mut()=None;self.mail_preview.set_text("");}
        if event!=nwg::Event::OnButtonClick{return;}let Some(i)=self.mail_buttons.iter().position(|b|b.handle==handle)else{return;};
        let result=(||->Result<()>{match i{
            0=>{let entries=self.mail_entries.borrow();let chosen:Vec<_>=self.mail_licenses.multi_selection().iter().map(|i|entries[*i].clone()).collect();let draft=mail::compose(&self.mail_to.text(),&self.mail_subject.text(),&chosen)?;self.mail_preview.set_text(&format!("Aan: {}\r\nOnderwerp: {}\r\n\r\n{}",draft.request.recipient,draft.request.subject,draft.body));*self.mail_draft.borrow_mut()=Some(draft);},
            1=>self.send_mail()?,
            2=>{let draft=self.mail_draft.borrow().clone().context("Maak eerst een voorbeeld en controleer de ontvanger")?;let url=mail::mailto(&draft)?;open::that(url).context("Geen mailprogramma geopend; configureer de standaard mail-app")?;nwg::modal_info_message(&self.mail_window,"E-mail geopend","De e-mail is geopend in je eigen mailprogramma. Controleer de inhoud en klik daar op Verzenden. LicentieKluis kan de verzending niet controleren.");},
            3=>self.close_mail(),4=>{self.mail_licenses.select_all();*self.mail_draft.borrow_mut()=None;self.mail_preview.set_text("");},5=>{self.mail_licenses.unselect_all();*self.mail_draft.borrow_mut()=None;self.mail_preview.set_text("");},_=>{}}Ok(())})();if let Err(e)=result{nwg::modal_error_message(&self.mail_window,"E-mail niet verstuurd",&format!("{e:#}"));}
    }
    fn send_mail(&self)->Result<()> {
        let draft=self.mail_draft.borrow().clone().context("Maak eerst een voorbeeld en controleer de ontvanger")?;let session=self.session.borrow().clone().context("Meld eerst aan bij de server, of gebruik je eigen mailprogramma")?;let state=self.snapshot()?;
        for item in &draft.request.licenses{ensure!(!state.pending.contains_key(&item.id)&&!state.conflicts.contains_key(&item.id),"Synchroniseer eerst de geselecteerde licenties en los conflicten op");}
        let text=format!("{} licenties met volledige codes versturen naar {}?",draft.request.licenses.len(),draft.request.recipient);
        let answer=nwg::modal_message(&self.mail_window,&nwg::MessageParams{title:"Verzending bevestigen",content:&text,buttons:nwg::MessageButtons::YesNo,icons:nwg::MessageIcons::Warning});if answer!=nwg::MessageChoice::Yes{return Ok(());}
        self.busy.set(true);self.mail_window.set_enabled(false);let slot=self.mail_result.clone();let sender=self.notice.sender();
        std::thread::spawn(move||{let result=sync::send_mail(&session,&draft.request).map_err(|e|format!("{e:#}"));if let Ok(mut s)=slot.lock(){*s=Some(result);}sender.notice();});Ok(())
    }
    fn finish_mail(&self){
        self.busy.set(false);self.mail_window.set_enabled(true);let result=self.mail_result.lock().ok().and_then(|mut s|s.take());
        match result{Some(Ok(outcome))=>{*self.mail_draft.borrow_mut()=None;self.mail_preview.set_text(&outcome.message);nwg::modal_info_message(&self.mail_window,"Mailstatus",&outcome.message);},Some(Err(error))=>{nwg::modal_error_message(&self.mail_window,"Verzending niet bevestigd",&format!("{}\nControleer eerst de ontvangst. Bij opnieuw klikken blijft hetzelfde verzoeknummer behouden om dubbele verzending te voorkomen.",error));},None=>{}}
    }
    fn clear_clipboard(&self){if let Some((value,_))=self.copied.borrow_mut().take(){if clipboard_win::get_clipboard_string().ok().as_deref()==Some(value.as_str()){let _=clipboard_win::set_clipboard_string("");}}}
    fn tick(&self){let expire=self.copied.borrow().as_ref().is_some_and(|(_,t)|t.elapsed()>Duration::from_secs(30));if expire{self.clear_clipboard();}if self.revealed.get().is_some_and(|t|t.elapsed()>Duration::from_secs(30)){self.fields[CODE].set_password_char(Some('*'));self.revealed.set(None);}if !self.busy.get()&&!self.dirty.get()&&!self.modal.get()&&self.session.borrow().is_some()&&self.last_sync.get().elapsed()>Duration::from_secs(60){self.last_sync.set(Instant::now());if let Err(e)=self.begin_work(false){self.status.set_text(&e.to_string());}}}
}
