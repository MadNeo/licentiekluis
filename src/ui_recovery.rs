use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use licentiekluis_native::recovery::{Recovery, TextFile, Photo, MAX_PHOTO, MAX_TEXT_FILE};

pub(super) struct RecoveryUi {
    pub window:nwg::Window, pub preview:nwg::Window,
    list:nwg::ListBox<String>,search:nwg::TextInput,filter:nwg::ComboBox<String>,
    name:nwg::TextInput,category:nwg::ComboBox<String>,text:nwg::TextBox,
    file_label:nwg::Label,photo_label:nwg::Label,status:nwg::Label,
    buttons:Vec<nwg::Button>,preview_close:nwg::Button,preview_text:nwg::TextBox,photo:nwg::ImageFrame,
    bitmap:RefCell<Option<nwg::Bitmap>>,ids:RefCell<Vec<Uuid>>,draft:RefCell<Option<Entry>>,
    dirty:Cell<bool>,loading:Cell<bool>,revealed:Cell<Option<Instant>>,preview_at:Cell<Option<Instant>>,
    _labels:Vec<nwg::Label>,handlers:Vec<nwg::RawEventHandler>,
}
impl RecoveryUi {
 pub fn build(parent:&nwg::Window,theme:Rc<theme::Theme>)->Result<Self>{
    let mut window=nwg::Window::default();nwg::Window::builder().parent(Some(parent)).size((980,740)).position((60,25)).title("Herstelsleutels · LicentieKluis").flags(nwg::WindowFlags::WINDOW).build(&mut window)?;
    let mut preview=nwg::Window::default();nwg::Window::builder().parent(Some(&window)).size((800,700)).position((140,40)).title("Herstelgegevens · 30 seconden zichtbaar").flags(nwg::WindowFlags::WINDOW).build(&mut preview)?;
    let mut labels=vec![];let mut label=|text:&str,x,y,w,h|->Result<()>{let mut l=nwg::Label::default();nwg::Label::builder().parent(&window).text(text).position((x,y)).size((w,h)).build(&mut l)?;labels.push(l);Ok(())};
    for (t,x,y,w,h) in [("Herstelsleutels",24,20,650,30),("Authenticator of crypto-wallet · kies tekst, een tekstbestand en/of een foto",24,56,910,24),("Je herstelgegevens",24,105,274,24),("Naam",330,108,600,22),("Categorie",330,171,600,22),("01 · Handmatig invoeren (optioneel, maximaal 20.000 tekens)",330,236,620,24),("02 · Tekstbestand (optioneel, UTF-8 .txt, maximaal 64 KB)",330,402,620,24),("03 · Foto (optioneel, PNG of JPEG, maximaal 2 MB)",330,508,620,24),("De inhoud blijft verborgen totdat je deze opent.",24,668,900,24)]{label(t,x,y,w,h)?;}drop(label);
    let mut list=nwg::ListBox::default();nwg::ListBox::builder().parent(&window).position((24,220)).size((274,380)).build(&mut list)?;
    let mut search=nwg::TextInput::default();nwg::TextInput::builder().parent(&window).position((24,139)).size((274,28)).placeholder_text(Some("Zoek op naam…")).build(&mut search)?;
    let mut filter=nwg::ComboBox::default();nwg::ComboBox::builder().parent(&window).position((24,181)).size((274,160)).collection(vec!["Alle categorieën".into(),"Authenticator".into(),"Crypto-wallet".into()]).selected_index(Some(0)).build(&mut filter)?;
    let mut name=nwg::TextInput::default();nwg::TextInput::builder().parent(&window).position((330,135)).size((620,28)).build(&mut name)?;
    let mut category=nwg::ComboBox::default();nwg::ComboBox::builder().parent(&window).position((330,198)).size((620,140)).collection(vec!["Authenticator".into(),"Crypto-wallet".into()]).selected_index(Some(0)).build(&mut category)?;
    let mut text=nwg::TextBox::default();nwg::TextBox::builder().parent(&window).position((330,268)).size((620,80)).flags(nwg::TextBoxFlags::VISIBLE|nwg::TextBoxFlags::TAB_STOP|nwg::TextBoxFlags::VSCROLL|nwg::TextBoxFlags::AUTOVSCROLL).build(&mut text)?;
    let mut file_label=nwg::Label::default();nwg::Label::builder().parent(&window).position((330,432)).size((620,24)).text("Geen tekstbestand toegevoegd").build(&mut file_label)?;
    let mut photo_label=nwg::Label::default();nwg::Label::builder().parent(&window).position((330,538)).size((620,24)).text("Geen foto toegevoegd").build(&mut photo_label)?;
    let mut status=nwg::Label::default();nwg::Label::builder().parent(&window).position((24,704)).size((926,28)).build(&mut status)?;
    let mut buttons=vec![];
    for(t,x,y,w) in [("+ Nieuwe herstelsleutel",24,616,274),("Opslaan",800,616,150),("Verwijderen",330,616,145),("Bekijken",490,616,145),("Tekst tonen / bewerken",330,358,245),("Tekstbestand kiezen",330,465,245),("Tekstbestand wissen",590,465,220),("Foto kiezen",330,571,245),("Foto wissen",590,571,220),("Sluiten",800,20,150)]{let mut b=nwg::Button::default();nwg::Button::builder().parent(&window).position((x,y)).size((w,30)).text(t).build(&mut b)?;buttons.push(b);}
    let mut preview_text=nwg::TextBox::default();nwg::TextBox::builder().parent(&preview).position((24,24)).size((752,220)).readonly(true).flags(nwg::TextBoxFlags::VISIBLE|nwg::TextBoxFlags::TAB_STOP|nwg::TextBoxFlags::VSCROLL|nwg::TextBoxFlags::AUTOVSCROLL).build(&mut preview_text)?;
    let mut photo=nwg::ImageFrame::default();nwg::ImageFrame::builder().parent(&preview).position((24,260)).size((752,360)).build(&mut photo)?;
    let mut preview_close=nwg::Button::default();nwg::Button::builder().parent(&preview).position((530,646)).size((246,30)).text("Verbergen en sluiten").build(&mut preview_close)?;
    let mut handlers=vec![];
    for(w,size) in [(&window,(980,740)),(&preview,(800,700))]{render::fit_children(w,size);handlers.push(render::bind(w,vec![(0,0,size.0,size.1,[243,246,250])],size,theme.clone())?);theme::prepare(w);theme::repaint(w,theme.dark.get());}
    for l in labels.iter().chain([&file_label,&photo_label,&status]){handlers.push(render::bind_label(l,theme.clone())?);}
    Ok(Self{window,preview,list,search,filter,name,category,text,file_label,photo_label,status,buttons,preview_close,preview_text,photo,bitmap:RefCell::new(None),ids:RefCell::new(vec![]),draft:RefCell::new(None),dirty:Cell::new(false),loading:Cell::new(false),revealed:Cell::new(None),preview_at:Cell::new(None),_labels:labels,handlers})
 }
 pub fn hide_preview(&self){self.preview.set_visible(false);self.preview_text.set_text("");self.photo.set_bitmap(None);self.bitmap.borrow_mut().take();self.preview_at.set(None);}
 pub fn clear(&self){self.hide_preview();self.loading.set(true);self.text.set_text("");self.name.set_text("");self.file_label.set_text("");self.photo_label.set_text("");self.draft.borrow_mut().take();self.list.clear();self.ids.borrow_mut().clear();self.revealed.set(None);self.dirty.set(false);self.loading.set(false);}
 pub fn tick(&self){if self.preview_at.get().is_some_and(|t|t.elapsed()>Duration::from_secs(30)){self.hide_preview();}if self.revealed.get().is_some_and(|t|t.elapsed()>Duration::from_secs(30)){self.text.set_visible(false);self.revealed.set(None);self.buttons[4].set_text("Tekst tonen / bewerken");}}
}
impl Drop for RecoveryUi {fn drop(&mut self){for h in &self.handlers{let _=nwg::unbind_raw_event_handler(h);}}}
impl App {
 pub(super) fn recovery_open(&self)->Result<()>{self.snapshot()?;ensure!(!self.dirty.get(),"Sla eerst de licentie-invoer op");self.recovery_refresh();self.recovery_fill(None);self.recovery.window.set_visible(true);self.window.set_enabled(false);self.modal.set(true);self.recovery.window.set_focus();Ok(())}
 fn recovery_discard(&self)->bool{!self.recovery.dirty.get()||self.confirm("Niet-opgeslagen herstelgegevens verwerpen?")}
 fn recovery_close(&self){if !self.recovery_discard(){return;}self.recovery.clear();self.recovery.window.set_visible(false);self.window.set_enabled(true);self.modal.set(false);self.window.set_focus();}
 pub(super) fn recovery_refresh(&self){
    let ui=&self.recovery;ui.loading.set(true);let selected=ui.draft.borrow().as_ref().map(|e|e.id);let s=self.state.borrow();
    if let Some(s)=s.as_ref(){let q=ui.search.text().to_lowercase();let filter=ui.filter.selection().unwrap_or(0);let mut rows:Vec<_>=s.entries.iter().filter(|e|e.kind=="recovery"&&!e.deleted&&e.name.to_lowercase().contains(&q)&&e.recovery.as_ref().is_some_and(|r|filter==0||r.category==if filter==1{"authenticator"}else{"wallet"})).collect();rows.sort_by_key(|e|e.name.to_lowercase());*ui.ids.borrow_mut()=rows.iter().map(|e|e.id).collect();ui.list.set_collection(rows.iter().map(|e|format!("{} · {}{}",if e.recovery.as_ref().unwrap().category=="wallet"{"Wallet"}else{"Auth"},e.name,if s.pending.contains_key(&e.id){" *"}else{""})).collect());ui.list.set_selection(selected.and_then(|id|ui.ids.borrow().iter().position(|x|*x==id)));ui.status.set_text(&format!("{} herstelsleutels · {} wachtende wijzigingen · {} conflicten",rows.len(),s.pending.len(),s.conflicts.len()));}ui.loading.set(false);
 }
 fn recovery_fill(&self,id:Option<Uuid>){
    let ui=&self.recovery;ui.hide_preview();ui.loading.set(true);let e=id.and_then(|id|self.state.borrow().as_ref()?.entries.iter().find(|e|e.id==id).cloned()).unwrap_or_else(||{let mut e=Entry::new("recovery","");e.recovery=Some(Recovery{category:"authenticator".into(),..Default::default()});e});let r=e.recovery.as_ref().unwrap();ui.name.set_text(&e.name);ui.category.set_selection(Some(if r.category=="wallet"{1}else{0}));ui.text.set_text(&r.text);ui.text.set_visible(r.text.is_empty());ui.buttons[4].set_text(if r.text.is_empty(){"Tekst verbergen"}else{"Tekst tonen / bewerken"});ui.file_label.set_text(r.text_file.as_ref().map_or("Geen tekstbestand toegevoegd",|f|f.name.as_str()));ui.photo_label.set_text(r.photo.as_ref().map_or("Geen foto toegevoegd",|p|p.name.as_str()));*ui.draft.borrow_mut()=Some(e);ui.dirty.set(false);ui.revealed.set(None);ui.loading.set(false);
 }
 pub(super) fn recovery_event(&self,event:nwg::Event,handle:nwg::ControlHandle){
    let ui=&self.recovery;if ui.loading.get(){return;}if event==nwg::Event::OnWindowClose{if handle==ui.preview.handle{ui.hide_preview();}else{self.recovery_close();}return;}
    let result=(||->Result<()>{
      if event==nwg::Event::OnButtonClick && handle==ui.preview_close.handle{ui.hide_preview();return Ok(());}
      if event==nwg::Event::OnTextInput&&handle==ui.search.handle||event==nwg::Event::OnComboxBoxSelection&&handle==ui.filter.handle{self.recovery_refresh();return Ok(());}
      if event==nwg::Event::OnListBoxSelect&&handle==ui.list.handle{if self.recovery_discard(){let id=ui.list.selection().and_then(|i|ui.ids.borrow().get(i).copied());self.recovery_fill(id);}else{self.recovery_refresh();}return Ok(());}
      if event==nwg::Event::OnTextInput&&(handle==ui.name.handle||handle==ui.text.handle)||event==nwg::Event::OnComboxBoxSelection&&handle==ui.category.handle{ui.dirty.set(true);if handle==ui.text.handle{ui.revealed.set(Some(Instant::now()));}}
      if event!=nwg::Event::OnButtonClick{return Ok(());}let Some(i)=ui.buttons.iter().position(|b|b.handle==handle)else{return Ok(());};
      match i {
       0=>if self.recovery_discard(){self.recovery_fill(None);ui.list.set_selection(None);},
       1=>self.recovery_save()?,2=>self.recovery_delete()?,3=>self.recovery_view()?,
       4=>{let show=!ui.text.visible();ui.text.set_visible(show);ui.buttons[4].set_text(if show{"Tekst verbergen"}else{"Tekst tonen / bewerken"});ui.revealed.set(if show{Some(Instant::now())}else{None});},
       5|7=>self.recovery_attach(i==7)?,
       6|8=>{if let Some(e)=ui.draft.borrow_mut().as_mut(){let r=e.recovery.as_mut().unwrap();if i==6{r.text_file=None;ui.file_label.set_text("Geen tekstbestand toegevoegd");}else{r.photo=None;ui.photo_label.set_text("Geen foto toegevoegd");}ui.dirty.set(true);}},
       9=>self.recovery_close(),_=>{}
      }Ok(())
    })();if let Err(e)=result{ui.status.set_text(&format!("{e:#}"));}
 }
 fn recovery_save(&self)->Result<()>{
    let ui=&self.recovery;let mut e=ui.draft.borrow().clone().context("Maak of selecteer een herstelsleutel")?;let mut s=self.snapshot()?;
    ensure!(!s.entries.iter().any(|x|x.id==e.id&&x.mutation_id!=e.mutation_id),"Deze herstelsleutel is ondertussen gewijzigd. Open de actuele versie opnieuw");
    e.name=ui.name.text().trim().into();let r=e.recovery.as_mut().unwrap();r.category=if ui.category.selection()==Some(1){"wallet"}else{"authenticator"}.into();r.text=ui.text.text();let id=e.id;s.edit(e)?;self.commit(s)?;self.recovery_fill(Some(id));self.recovery_refresh();ui.status.set_text("Versleuteld opgeslagen. Synchronisatie hervat zodra je dit venster sluit.");Ok(())
 }
 fn recovery_delete(&self)->Result<()>{let mut s=self.snapshot()?;let id=self.recovery.draft.borrow().as_ref().map(|e|e.id).context("Selecteer een herstelsleutel")?;let mut e=s.entries.iter().find(|e|e.id==id).context("Sla de nieuwe herstelsleutel eerst op")?.clone();if !self.confirm("Deze herstelsleutel met tekst en bijlagen verwijderen, ook op de server na synchronisatie?"){return Ok(());}e.deleted=true;e.recovery=Some(Recovery{category:e.recovery.as_ref().unwrap().category.clone(),..Default::default()});s.edit(e)?;self.commit(s)?;self.recovery_fill(None);self.recovery_refresh();Ok(())}
 fn recovery_attach(&self,photo:bool)->Result<()>{
    let ui=&self.recovery;let mut dialog=nwg::FileDialog::default();nwg::FileDialog::builder().title(if photo{"Foto kiezen (maximaal 2 MB)"}else{"UTF-8-tekstbestand kiezen (maximaal 64 KB)"}).action(nwg::FileDialogAction::Open).filters(if photo{"Foto(*.png;*.jpg;*.jpeg)"}else{"Tekst(*.txt)"}).build(&mut dialog)?;if !dialog.run(Some(&ui.window)){return Ok(());}let path=PathBuf::from(dialog.get_selected_item()?);let max=if photo{MAX_PHOTO}else{MAX_TEXT_FILE};ensure!(std::fs::metadata(&path)?.len()<=max as u64,"Bestand te groot");
    use std::io::Read;let mut bytes=Zeroizing::new(Vec::new());std::fs::File::open(&path)?.take((max+1)as u64).read_to_end(&mut bytes)?;ensure!(bytes.len()<=max,"Bestand te groot");let name=path.file_name().context("Ongeldige bestandsnaam")?.to_string_lossy().to_string();let mut draft=ui.draft.borrow_mut();let e=draft.as_mut().context("Maak of selecteer een herstelsleutel")?;let mut r=e.recovery.as_ref().unwrap().clone();
    if photo{r.photo=Some(Photo{name:name.clone(),media_type:licentiekluis_native::recovery::photo_type(&bytes)?.into(),data:STANDARD.encode(&bytes)});}else{let text=std::str::from_utf8(&bytes).context("Gebruik een UTF-8-tekstbestand")?.trim_start_matches('\u{feff}').to_string();r.text_file=Some(TextFile{name:name.clone(),text});}r.validate(true)?;e.recovery=Some(r);if photo{ui.photo_label.set_text(&name);}else{ui.file_label.set_text(&name);}ui.dirty.set(true);Ok(())
 }
 fn recovery_view(&self)->Result<()>{
    let ui=&self.recovery;ensure!(!ui.dirty.get(),"Sla wijzigingen eerst op om ze te bekijken");let draft=ui.draft.borrow();let e=draft.as_ref().context("Selecteer een herstelsleutel")?;let r=e.recovery.as_ref().unwrap();r.validate(false)?;ui.hide_preview();
    let mut text=Zeroizing::new(String::new());if !r.text.is_empty(){text.push_str("HANDMATIGE TEKST\r\n");text.push_str(&r.text);text.push_str("\r\n\r\n");}if let Some(f)=&r.text_file{text.push_str(&f.name);text.push_str("\r\n");text.push_str(&f.text);}ui.preview_text.set_text(&text);
    if let Some(photo)=&r.photo{let bytes=Zeroizing::new(STANDARD.decode(&photo.data)?);let decoder=nwg::ImageDecoder::new()?;let source=decoder.from_stream(&bytes)?;let frame=source.frame(0)?;let(w,h)=frame.size();ensure!(w>0&&h>0&&w<=20000&&h<=20000&&(w as u64*h as u64)<=32000000,"Afbeelding heeft ongeldige afmetingen");let ratio=(752.0/w as f64).min(360.0/h as f64).min(1.0);let image=decoder.resize_image(&frame,[(w as f64*ratio).max(1.0)as u32,(h as f64*ratio).max(1.0)as u32])?;let bitmap=image.as_bitmap()?;ui.photo.set_bitmap(Some(&bitmap));*ui.bitmap.borrow_mut()=Some(bitmap);}
    ui.preview_at.set(Some(Instant::now()));ui.preview.set_visible(true);ui.preview.set_focus();Ok(())
 }
}

#[cfg(test)] pub(super) fn verify_recovery_ui(app:&App){
 use super::tests::{paint_now,reachable,snapshot};
 let ui=&app.recovery;app.recovery_open().unwrap();
 ui.name.set_text("Authenticator test");ui.text.set_text("SYNTHETIC UI RECOVERY");app.recovery_save().unwrap();
 let id=ui.draft.borrow().as_ref().unwrap().id;
 assert!(!ui.text.visible(),"Saved recovery text stays hidden");
 let bytes=include_bytes!("../fixtures/recovery-test.png");
 {let mut draft=ui.draft.borrow_mut();let r=draft.as_mut().unwrap().recovery.as_mut().unwrap();r.text_file=Some(TextFile{name:"test.txt".into(),text:"SYNTHETIC FILE".into()});r.photo=Some(Photo{name:"test.png".into(),media_type:"image/png".into(),data:STANDARD.encode(bytes)});}
 app.recovery_save().unwrap();
 let reopened=database::load(&app.path,"synthetic-test-password").unwrap();assert!(reopened.entries.iter().find(|e|e.id==id).unwrap().recovery.as_ref().unwrap().photo.is_some());
 let shots=PathBuf::from("target/ui-check");
 unsafe{paint_now(&ui.window);for b in &ui.buttons{reachable(&b.handle);}for h in [&ui.list.handle,&ui.name.handle,&ui.category.handle,&ui.search.handle,&ui.filter.handle]{reachable(h);}snapshot(ui.window.handle.hwnd().unwrap(),&shots.join("11-recovery-dark.bmp"),true);}
 app.recovery_view().unwrap();assert!(ui.preview_text.text().contains("SYNTHETIC FILE"));assert!(ui.bitmap.borrow().is_some());
 unsafe{paint_now(&ui.preview);reachable(&ui.preview_close.handle);snapshot(ui.preview.handle.hwnd().unwrap(),&shots.join("12-recovery-preview.bmp"),true);}
 ui.preview_at.set(Some(Instant::now()-Duration::from_secs(31)));ui.tick();assert!(!ui.preview.visible());assert!(ui.preview_text.text().is_empty());assert!(ui.bitmap.borrow().is_none());
 app.set_theme(false).unwrap();unsafe{paint_now(&ui.window);snapshot(ui.window.handle.hwnd().unwrap(),&shots.join("13-recovery-light.bmp"),false);}
 app.recovery_close();assert!(ui.draft.borrow().is_none());assert!(ui.text.text().is_empty());assert!(app.window.enabled());
}
