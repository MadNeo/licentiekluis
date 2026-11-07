use anyhow::{ensure,Context,Result};
use native_windows_gui as nwg;
#[path = "ui_render.rs"] mod render;
#[path = "ui_recovery.rs"] mod recovery_ui;
#[path = "ui_theme.rs"] mod theme;
#[cfg(test)] #[path = "ui_tests.rs"] mod tests;
use licentiekluis_native::{model::{Entry,Installation,LocalState},vault,database,sync::{self,Session},mail};
use std::{cell::{Cell,RefCell},path::PathBuf,rc::Rc,sync::{Arc,Mutex},time::{Duration,Instant}};
use uuid::Uuid;
use zeroize::Zeroizing;

const PW:usize=0;const REPEAT:usize=1;const SEARCH:usize=2;const PERSON:usize=3;const NAME:usize=4;const CODE:usize=5;const VERSION:usize=6;const EXPIRY:usize=7;const DEVICE:usize=8;const URL:usize=9;const USER:usize=10;const SERVERPW:usize=11;const BACKUPPW:usize=12;
const MAIL:usize=18;
const SETTINGS:usize=19;
const IMPORT_CA:usize=20;
const REMOVE_CA:usize=21;
const TLS_CHECK:usize=22;
const SERVER_SETTINGS:usize=23;
const RECOVERY:usize=24;
const UNLOCK:usize=0;const NEW:usize=1;const SAVE:usize=2;const DELETE:usize=3;const COPY:usize=4;const SHOW:usize=5;const ADDPERSON:usize=6;const RENAME:usize=7;const DELPERSON:usize=8;const ADDINSTALL:usize=9;const DELINSTALL:usize=10;const LOGIN:usize=11;const SYNC:usize=12;const LOGOUT:usize=13;const CONFLICT:usize=14;const BACKUP:usize=15;const RESTORE:usize=16;const LOCK:usize=17;
type WorkResult=(u64,LocalState,std::result::Result<(LocalState,Session),String>);

struct App {
    recovery:recovery_ui::RecoveryUi,
    theme:Rc<theme::Theme>,theme_path:PathBuf,theme_button:nwg::Button,tabs:nwg::TabsContainer,_tabs:Vec<nwg::Tab>,changelog:nwg::TextBox,body_children:Vec<winapi::shared::windef::HWND>,
    sync_busy:Cell<bool>,epoch:Cell<u64>,connection:RefCell<Option<database::Credentials>>,
    paint_handlers:Vec<nwg::RawEventHandler>,settings_window:nwg::Window,_fonts:Vec<nwg::Font>,
    mail_window:nwg::Window,mail_licenses:nwg::ListBox<String>,mail_to:nwg::TextInput,mail_subject:nwg::TextInput,mail_preview:nwg::TextBox,mail_buttons:Vec<nwg::Button>,mail_entries:RefCell<Vec<Entry>>,mail_draft:RefCell<Option<mail::Draft>>,mail_result:Arc<Mutex<Option<std::result::Result<mail::Outcome,String>>>>,
    window:nwg::Window, labels:Vec<nwg::Label>, fields:Vec<nwg::TextInput>, buttons:Vec<nwg::Button>,
    people:nwg::ListBox<String>, licenses:nwg::ListBox<String>, owner:nwg::ComboBox<String>, platform:nwg::ComboBox<String>, filter:nwg::ComboBox<String>,
    installations:nwg::ListBox<String>,notes:nwg::TextBox,license_count:nwg::Label,status:nwg::Label,notice:nwg::Notice,timer:nwg::AnimationTimer,
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
    let app=Rc::new(App::build(dir.join("vault.db"))?);let weak=Rc::downgrade(&app);
    let handler=nwg::full_bind_event_handler(&app.window.handle,move|event,_,handle|{if let Some(app)=weak.upgrade(){app.event(event,handle);}});
    app.status.set_text("Vergrendeld. Nieuwe kluis: kies minimaal 14 tekens en herhaal het wachtwoord.");
    let weak=Rc::downgrade(&app);let mail_handler=nwg::full_bind_event_handler(&app.mail_window.handle,move|event,_,handle|{if let Some(app)=weak.upgrade(){app.mail_event(event,handle);}});
    let weak=Rc::downgrade(&app);let settings_handler=nwg::full_bind_event_handler(&app.settings_window.handle,move|event,_,handle|{if let Some(app)=weak.upgrade(){if event==nwg::Event::OnWindowClose{app.settings_window.set_visible(false);}else{app.event(event,handle);}}});
    let weak=Rc::downgrade(&app);let recovery_handler=nwg::full_bind_event_handler(&app.recovery.window.handle,move|event,_,handle|{if let Some(app)=weak.upgrade(){app.recovery_event(event,handle);}});
    let weak=Rc::downgrade(&app);let preview_handler=nwg::full_bind_event_handler(&app.recovery.preview.handle,move|event,_,handle|{if let Some(app)=weak.upgrade(){app.recovery_event(event,handle);}});
    nwg::dispatch_thread_events();nwg::unbind_event_handler(&preview_handler);nwg::unbind_event_handler(&recovery_handler);nwg::unbind_event_handler(&settings_handler);nwg::unbind_event_handler(&mail_handler);nwg::unbind_event_handler(&handler);drop(app);drop(lock);Ok(())
}
impl App {
    fn build(path:PathBuf)->Result<Self> {
        let mut body=nwg::Font::default();nwg::Font::builder().family("Segoe UI").size(16).build(&mut body)?;
        nwg::Font::set_global_default(Some(body));
        let mut title=nwg::Font::default();nwg::Font::builder().family("Segoe UI").size(28).weight(650).build(&mut title)?;
        let mut heading=nwg::Font::default();nwg::Font::builder().family("Segoe UI").size(18).weight(600).build(&mut heading)?;
        let mut small=nwg::Font::default();nwg::Font::builder().family("Segoe UI").size(13).build(&mut small)?;
        let mut strong=nwg::Font::default();nwg::Font::builder().family("Segoe UI").size(15).weight(600).build(&mut strong)?;
        let mut mono=nwg::Font::default();nwg::Font::builder().family("Consolas").size(15).build(&mut mono)?;
        let mut window=nwg::Window::default();nwg::Window::builder().size((1180,780)).position((30,20)).title(concat!("LicentieKluis ",env!("CARGO_PKG_VERSION"))).flags(nwg::WindowFlags::WINDOW|nwg::WindowFlags::VISIBLE|nwg::WindowFlags::MINIMIZE_BOX).build(&mut window)?;
        let mut settings_window=nwg::Window::default();nwg::Window::builder().size((720,620)).position((180,90)).title("Instellingen · LicentieKluis").parent(Some(&window)).flags(nwg::WindowFlags::WINDOW).build(&mut settings_window)?;
        let mut mail_window=nwg::Window::default();nwg::Window::builder().size((920,680)).position((100,50)).title("Licenties delen · LicentieKluis").parent(Some(&window)).flags(nwg::WindowFlags::WINDOW).build(&mut mail_window)?;
        let mut labels=vec![];
        let mut decorations:Vec<(nwg::ControlHandle,render::Panel)>=vec![];
        let bg=[243,246,250];let white=[255,255,255];let mint=[228,245,241];let teal=[20,139,125];
        let mut label=|parent:&nwg::Window,text:&str,x,y,w,h,color,font:&nwg::Font|->Result<()>{
            if text.is_empty(){decorations.push((parent.handle.clone(),(x,y,w,h,color)));return Ok(());}
            let mut l=nwg::Label::default();nwg::Label::builder().parent(parent).text(text).position((x,y)).size((w,h)).background_color(Some(color)).font(Some(font)).build(&mut l)?;labels.push(l);Ok(())
        };
        label(&window,"",0,0,1180,780,bg,&small)?;
        label(&window,"",0,0,1180,80,white,&small)?;
        label(&window,"",0,0,6,80,teal,&small)?;
        label(&window,"LicentieKluis",24,13,440,34,white,&title)?;
        label(&window,"WERKRUIMTE  /  SOFTWARELICENTIES",26,50,425,20,white,&small)?;
        label(&window,"",20,96,1140,72,mint,&small)?;
        label(&window,"",20,96,4,72,teal,&small)?;
        label(&window,"Kluiswachtwoord",36,106,204,20,mint,&small)?;
        label(&window,"Herhalen bij een nieuwe kluis",254,106,228,20,mint,&small)?;
        label(&window,"Lokaal versleuteld opgeslagen",666,108,300,20,mint,&strong)?;
        label(&window,"Je kunt ook zonder server blijven werken.",666,133,340,22,mint,&small)?;
        for (x, w) in [(20, 198), (232, 360), (606, 554)] {
            label(&window, "", x, 184, w, 540, white, &small)?;
        }
        label(&window,"Gebruikers",36,200,166,26,white,&heading)?;
        label(&window,"Filter op licentiehouder",36,231,166,20,white,&small)?;
        label(&window,"Gebruikers beheren",36,532,166,23,white,&strong)?;
        label(&window,"Licenties",248,200,162,26,white,&heading)?;
        label(&window,"Zoek op software, versie of apparaat",248,246,328,20,white,&small)?;
        label(&window,"Licentiegegevens",624,200,518,26,white,&heading)?;
        for (text, x, y, w) in [
            ("Software",624,240,248), ("Toegewezen aan",888,240,254),
            ("Licentiecode",624,300,518), ("Versie",624,360,160),
            ("Geldig tot (jjjj-mm-dd)",800,360,230),
            ("Installaties",624,422,518),
            ("Platform",624,449,144), ("Apparaat",784,449,230),
            ("Opmerkingen",624,586,518),
        ] {
            let font = if text == "Installaties" { &strong } else { &small };
            label(&window,text,x,y,w,20,white,font)?;
        }
        label(&window,"",20,738,1140,34,mint,&small)?;
        label(&settings_window,"",0,0,720,620,bg,&small)?;
        label(&settings_window,"Instellingen",24,18,440,36,bg,&title)?;
        label(&settings_window,"Automatische sync elke 30 seconden. Zonder server werk je lokaal.",24,59,664,24,bg,&small)?;
        label(&settings_window,"",20,98,680,328,white,&small)?;
        label(&settings_window,"Serververbinding",36,112,640,26,white,&heading)?;
        for (text,x,y,w) in [("HTTPS-serveradres",36,149,648),("Serveraccount",36,215,252),("Serverwachtwoord",304,215,236)] {label(&settings_window,text,x,y,w,22,white,&small)?;}
        label(&settings_window,"CA geldt alleen voor dit adres · E-mail instellen opent je browser",36,396,648,21,white,&small)?;
        label(&settings_window,"",20,442,680,150,white,&small)?;
        label(&settings_window,"Back-up & herstel",36,456,648,26,white,&heading)?;
        label(&settings_window,"Wachtwoord van de back-up (alleen bij herstellen)",36,494,420,22,white,&small)?;
        label(&settings_window,"Bewaar je back-up ook buiten deze computer.",36,561,620,22,white,&small)?;
        let positions=[(36,130,204),(254,130,228),(248,270,328),(36,562,166),(624,262,248),(624,322,334),(624,382,160),(800,382,244),(784,472,216),(36,175,648),(36,241,252),(304,241,236),(36,520,330)];
        let placeholders=["Wachtwoord","Nogmaals (alleen eerste keer)","Zoeken…","Naam gebruiker","Bijvoorbeeld Microsoft Office","Licentiecode","Bijvoorbeeld 2024","Leeg = geen einddatum","Bijvoorbeeld laptop kantoor","https://192.168.178.100","Gebruikersnaam","Serverwachtwoord","Wachtwoord van de back-up"];
        let mut fields=vec![];
        for (i,(x,y,w)) in positions.into_iter().enumerate(){let mut f=nwg::TextInput::default();let parent=if i>=URL{&settings_window}else{&window};nwg::TextInput::builder().parent(parent).position((x,y)).size((w,28)).placeholder_text(Some(placeholders[i])).password(if [PW,REPEAT,CODE,SERVERPW,BACKUPPW].contains(&i){Some('●')}else{None}).build(&mut f)?;if i==CODE{f.set_font(Some(&mono));}fields.push(f);}
        let specifications = [
            ("Ontgrendelen",494,128,140), ("+ Nieuwe licentie",420,199,156),
            ("Opslaan",986,674,156), ("Verwijderen",624,674,140),
            ("Kopiëren",968,322,82), ("Tonen",1060,322,82),
            ("Toevoegen",36,600,166), ("Hernoemen",36,638,166),
            ("Verwijderen",36,676,166), ("Voeg toe",1008,470,80),
            ("−",1094,470,48), ("Aanmelden",556,239,128),
            ("Synchroniseren",36,283,184), ("Afmelden",232,283,132),
            ("Conflicten",376,283,132), ("Back-up maken",382,518,146),
            ("Herstellen",540,518,144), ("Vergrendelen",1018,128,126),
            ("Licenties delen",248,674,328), ("Instellingen",920,25,224),
            ("CA-certificaat kiezen",36,363,224), ("CA-certificaat wissen",274,363,224),
            ("Certificaatcontrole instellen",36,319,648),
            ("E-mail instellen",520,283,164),
            ("Herstelsleutels",704,25,200),
        ];
        let mut buttons=vec![];
        for (i,(text,x,y,w)) in specifications.into_iter().enumerate(){let mut b=nwg::Button::default();let parent=if [LOGIN,SYNC,LOGOUT,CONFLICT,BACKUP,RESTORE,IMPORT_CA,REMOVE_CA,TLS_CHECK,SERVER_SETTINGS].contains(&i){&settings_window}else{&window};nwg::Button::builder().text(text).parent(parent).position((x,y)).size((w,30)).font(Some(&strong)).build(&mut b)?;buttons.push(b);}
        let mut people=nwg::ListBox::default();nwg::ListBox::builder().parent(&window).position((36,258)).size((166,256)).build(&mut people)?;
        let mut licenses=nwg::ListBox::default();nwg::ListBox::builder().parent(&window).position((248,373)).size((328,285)).build(&mut licenses)?;
        let mut owner=nwg::ComboBox::default();nwg::ComboBox::builder().parent(&window).position((888,262)).size((254,200)).build(&mut owner)?;
        let mut platform=nwg::ComboBox::default();nwg::ComboBox::builder().parent(&window).position((624,472)).size((144,200)).collection(vec!["Windows".to_string(),"Linux".into(),"macOS".into()]).selected_index(Some(0)).build(&mut platform)?;
        let mut filter=nwg::ComboBox::default();nwg::ComboBox::builder().parent(&window).position((248,310)).size((328,200)).collection(vec!["Alle platforms".to_string(),"Windows".into(),"Linux".into(),"macOS".into()]).selected_index(Some(0)).build(&mut filter)?;
        let mut installations=nwg::ListBox::default();nwg::ListBox::builder().parent(&window).position((624,511)).size((518,66)).build(&mut installations)?;
        let mut notes=nwg::TextBox::default();nwg::TextBox::builder().parent(&window).position((624,609)).size((518,52)).flags(nwg::TextBoxFlags::VISIBLE|nwg::TextBoxFlags::TAB_STOP|nwg::TextBoxFlags::VSCROLL|nwg::TextBoxFlags::AUTOVSCROLL).build(&mut notes)?;
        let mut license_count = nwg::Label::default();
        nwg::Label::builder().parent(&window).position((248,344)).size((328,22))
            .text("Ontgrendel de kluis om licenties te bekijken").font(Some(&small))
            .build(&mut license_count)?;
        let mut status=nwg::Label::default();nwg::Label::builder().parent(&window).position((36,741)).size((1108,28)).background_color(Some(mint)).font(Some(&small)).build(&mut status)?;
        let mut notice=nwg::Notice::default();nwg::Notice::builder().parent(&window).build(&mut notice)?;
        let mut timer=nwg::AnimationTimer::default();nwg::AnimationTimer::builder().parent(&window).interval(Duration::from_secs(1)).active(true).build(&mut timer)?;
        label(&mail_window,"",0,0,920,680,bg,&small)?;
        label(&mail_window,"Licenties delen",24,18,872,36,bg,&title)?;
        label(&mail_window,"Selecteer licenties, controleer het voorbeeld en kies hoe je wilt versturen.",24,61,872,22,bg,&small)?;
        label(&mail_window,"",20,102,330,480,white,&small)?;label(&mail_window,"",366,102,534,480,white,&small)?;
        label(&mail_window,"Selectie",36,116,298,26,white,&heading)?;
        label(&mail_window,"Meerdere licenties mogelijk (maximaal 50)",36,150,298,22,white,&small)?;
        let mut mail_licenses=nwg::ListBox::default();nwg::ListBox::builder().parent(&mail_window).position((36,184)).size((298,336)).flags(nwg::ListBoxFlags::VISIBLE|nwg::ListBoxFlags::MULTI_SELECT|nwg::ListBoxFlags::TAB_STOP).build(&mut mail_licenses)?;
        for (text,y) in [("Ontvanger",116),("Onderwerp",188),("Voorbeeld van de e-mail",260)]{label(&mail_window,text,382,y,502,22,white,&small)?;}
        let mut mail_to=nwg::TextInput::default();nwg::TextInput::builder().parent(&mail_window).position((382,144)).size((502,28)).placeholder_text(Some("naam@voorbeeld.nl")).build(&mut mail_to)?;
        let mut mail_subject=nwg::TextInput::default();nwg::TextInput::builder().parent(&mail_window).position((382,216)).size((502,28)).text("Softwarelicenties").build(&mut mail_subject)?;
        let mut mail_preview=nwg::TextBox::default();nwg::TextBox::builder().parent(&mail_window).position((382,288)).size((502,276)).readonly(true).flags(nwg::TextBoxFlags::VISIBLE|nwg::TextBoxFlags::TAB_STOP|nwg::TextBoxFlags::VSCROLL|nwg::TextBoxFlags::AUTOVSCROLL).build(&mut mail_preview)?;
        let mut mail_buttons=vec![];for (text,x,y,w) in [("Voorbeeld maken",20,604,160),("Via server versturen",192,604,184),("Open mailprogramma",388,604,200),("Sluiten",764,604,136),("Alles selecteren",36,538,144),("Selectie wissen",190,538,144),("E-mail instellen",602,604,146)]{let mut b=nwg::Button::default();nwg::Button::builder().parent(&mail_window).position((x,y)).size((w,32)).text(text).font(Some(&strong)).build(&mut b)?;mail_buttons.push(b);}
        drop(label);
        let theme_path=path.parent().unwrap().join("appearance.txt");let theme=Rc::new(theme::Theme::new(&theme_path));
        let body_children=theme::body_children(&window);
        let mut theme_button=nwg::Button::default();nwg::Button::builder().parent(&settings_window).position((490,22)).size((194,30)).text(if theme.dark.get(){"Lichte modus"}else{"Donkere modus"}).build(&mut theme_button)?;
        let mut tabs=nwg::TabsContainer::default();nwg::TabsContainer::builder().parent(&window).position((480,22)).size((214,34)).build(&mut tabs)?;
        let mut tab_main=nwg::Tab::default();nwg::Tab::builder().parent(&tabs).text("Kluis").build(&mut tab_main)?;
        let mut tab_changes=nwg::Tab::default();nwg::Tab::builder().parent(&tabs).text("Changelog").build(&mut tab_changes)?;
        let mut changelog=nwg::TextBox::default();nwg::TextBox::builder().parent(&window).position((24,100)).size((1120,620)).readonly(true).text(include_str!("../CHANGELOG.md")).flags(nwg::TextBoxFlags::TAB_STOP|nwg::TextBoxFlags::VSCROLL|nwg::TextBoxFlags::AUTOVSCROLL).build(&mut changelog)?;
        let mut paint_handlers=vec![];
        for (parent,design) in [(&window,(1180,780)),(&settings_window,(720,620)),(&mail_window,(920,680))] {
            let panels=decorations.iter().filter(|(handle,_)|*handle==parent.handle).map(|(_,panel)|*panel).collect();
            render::fit_children(parent,design);
            paint_handlers.push(render::bind(parent,panels,design,theme.clone())?);
            theme::prepare(parent);
            for l in labels.iter().filter(|l| unsafe {winapi::um::winuser::GetParent(l.handle.hwnd().unwrap())==parent.handle.hwnd().unwrap()}) {
                paint_handlers.push(render::bind_label(l,theme.clone())?);
            }
            if parent.handle == window.handle {
                for label in [&status, &license_count] {
                    paint_handlers.push(render::bind_label(label, theme.clone())?);
                }
            }
            theme::repaint(parent,theme.dark.get());
        }
        paint_handlers.push(render::bind_tabs(&tabs,theme.clone())?);
        for tab in [&tab_main,&tab_changes]{paint_handlers.push(render::bind_tab_page(&tab.handle,theme.clone())?);unsafe{winapi::um::winuser::ShowWindow(tab.handle.hwnd().unwrap(),winapi::um::winuser::SW_HIDE);}}
        theme::repaint(&window,theme.dark.get());
        let recovery=recovery_ui::RecoveryUi::build(&window,theme.clone())?;
        Ok(Self{recovery,theme,theme_path,theme_button,tabs,_tabs:vec![tab_main,tab_changes],changelog,body_children,sync_busy:Cell::new(false),epoch:Cell::new(0),connection:RefCell::new(None),paint_handlers,settings_window,_fonts:vec![title,heading,small,strong,mono],mail_window,mail_licenses,mail_to,mail_subject,mail_preview,mail_buttons,mail_entries:RefCell::new(vec![]),mail_draft:RefCell::new(None),mail_result:Arc::new(Mutex::new(None)),window,labels,fields,buttons,people,licenses,owner,platform,filter,installations,notes,license_count,status,notice,timer,state:RefCell::new(None),password:RefCell::new(None),session:RefCell::new(None),selected:Cell::new(None),people_ids:RefCell::new(vec![]),license_ids:RefCell::new(vec![]),draft_installs:RefCell::new(vec![]),loading:Cell::new(false),dirty:Cell::new(false),busy:Cell::new(false),modal:Cell::new(false),path,result:Arc::new(Mutex::new(None)),last_sync:Cell::new(Instant::now()),copied:RefCell::new(None),revealed:Cell::new(None)})
    }
    fn error(&self,e:anyhow::Error){let was_modal=self.modal.replace(true);nwg::modal_error_message(&self.window,"Bewerking niet voltooid",&format!("{e:#}"));self.modal.set(was_modal);}
    fn confirm(&self,text:&str)->bool{let was_modal=self.modal.replace(true);let result=nwg::modal_message(&self.window,&nwg::MessageParams{title:"LicentieKluis",content:text,buttons:nwg::MessageButtons::YesNo,icons:nwg::MessageIcons::Warning});self.modal.set(was_modal);result==nwg::MessageChoice::Yes}
    fn snapshot(&self)->Result<LocalState>{self.state.borrow().clone().context("Ontgrendel eerst de kluis")}
    fn commit(&self,next:LocalState)->Result<()>{let p=self.password.borrow();database::save(&self.path,&next,p.as_ref().context("Kluis vergrendeld")?)?;drop(p);*self.state.borrow_mut()=Some(next);self.dirty.set(false);self.refresh();Ok(())}
    fn set_theme(&self,dark:bool)->Result<()> {self.theme.dark.set(dark);self.theme_button.set_text(if dark{"Lichte modus"}else{"Donkere modus"});for window in [&self.window,&self.settings_window,&self.mail_window,&self.recovery.window,&self.recovery.preview]{theme::repaint(window,dark);}self.theme.save(&self.theme_path)}
    fn select_tab(&self){for tab in &self._tabs{unsafe{winapi::um::winuser::ShowWindow(tab.handle.hwnd().unwrap(),winapi::um::winuser::SW_HIDE);}}let changes=self.tabs.selected_tab()==1;theme::show_body(&self.body_children,!changes);self.changelog.set_visible(changes);if changes{self.changelog.set_focus();}theme::repaint(&self.window,self.theme.dark.get());}
    fn event(&self,event:nwg::Event,handle:nwg::ControlHandle){
        if event==nwg::Event::OnButtonClick && handle==self.theme_button.handle {if let Err(e)=self.set_theme(!self.theme.dark.get()){self.error(e);}return;}
        if event==nwg::Event::TabsContainerChanged && handle==self.tabs.handle {self.select_tab();return;}

        if event==nwg::Event::OnNotice && handle==self.notice.handle {if self.mail_result.lock().map(|s|s.is_some()).unwrap_or(false){self.finish_mail();}else{self.finish_work();}return;}
        if event==nwg::Event::OnTimerTick && handle==self.timer.handle {self.tick();return;}
        if self.busy.get()||self.loading.get(){return;}
        if event==nwg::Event::OnWindowClose {if !self.dirty.get()||self.confirm("Niet-opgeslagen invoer verwerpen en sluiten?"){self.clear_clipboard();nwg::stop_thread_dispatch();}return;}
        let result=(||->Result<()>{
            if event==nwg::Event::OnButtonClick {if let Some(i)=self.buttons.iter().position(|b|b.handle==handle){return self.action(i);}}
            if event==nwg::Event::OnListBoxSelect && handle==self.people.handle {self.render_list();return Ok(());}
            if event==nwg::Event::OnListBoxSelect && handle==self.licenses.handle {if self.discard(){let id=self.licenses.selection().and_then(|i|self.license_ids.borrow().get(i).copied());self.fill(id);}else{self.licenses.set_selection(self.selected.get().and_then(|id|self.license_ids.borrow().iter().position(|x|*x==id)));}return Ok(());}
            if event==nwg::Event::OnTextInput && handle==self.fields[SEARCH].handle {self.render_list();return Ok(());}
            if event==nwg::Event::OnComboxBoxSelection && handle==self.filter.handle {self.render_list();return Ok(());}
            if (event==nwg::Event::OnTextInput && (self.fields[NAME..=DEVICE].iter().any(|x|x.handle==handle)||handle==self.notes.handle)) || (event==nwg::Event::OnComboxBoxSelection && handle==self.owner.handle){if self.state.borrow().is_some(){self.dirty.set(true);}}
            Ok(())
        })();if let Err(e)=result{self.error(e);}
    }
    fn discard(&self)->bool{!self.dirty.get()||self.confirm("Niet-opgeslagen licentie-invoer verwerpen?")}
    fn action(&self,i:usize)->Result<()> {
        if i==SETTINGS{self.refresh_tls_button();self.settings_window.set_visible(true);self.settings_window.set_focus();return Ok(());}
        if i==UNLOCK{return self.unlock();}
        ensure!(self.state.borrow().is_some(),"Ontgrendel eerst de kluis");
        match i {
            RECOVERY=>self.recovery_open()?,
            MAIL=>self.open_mail()?,
            NEW=>{if self.discard(){self.fill(None);}},SAVE=>self.save_license()?,DELETE=>self.delete_license()?,
            COPY=>{let code=self.fields[CODE].text();clipboard_win::set_clipboard_string(&code).map_err(|e|anyhow::anyhow!("Kopiëren naar klembord mislukt: {e:?}"))?;*self.copied.borrow_mut()=Some((Zeroizing::new(code),Instant::now()));},
            SHOW=>{self.fields[CODE].set_password_char(None);self.revealed.set(Some(Instant::now()));},
            ADDPERSON|RENAME|DELPERSON=>{ensure!(!self.dirty.get(),"Sla eerst de licentie-invoer op");self.edit_person(i)?;},
            ADDINSTALL=>{let p=self.platform.selection_string().context("Kies een platform")?;self.draft_installs.borrow_mut().push(Installation{platform:p,device:self.fields[DEVICE].text()});self.fields[DEVICE].set_text("");self.render_installs();self.dirty.set(true);},
            DELINSTALL=>{if let Some(index)=self.installations.selection(){self.draft_installs.borrow_mut().remove(index);self.render_installs();self.dirty.set(true);}},
            LOGIN=>self.begin_work(true)?,SYNC=>self.begin_work(false)?,
            IMPORT_CA => self.update_server_ca(true)?,
            REMOVE_CA => self.update_server_ca(false)?,
            TLS_CHECK => self.toggle_certificate_verification()?,
            SERVER_SETTINGS => self.open_server_settings()?,
            LOGOUT=>{database::forget(&self.path)?;*self.connection.borrow_mut()=None;self.epoch.set(self.epoch.get()+1);if let Some(s)=self.session.borrow_mut().take(){std::thread::spawn(move||sync::logout(&s));}self.status.set_text("Afgemeld. Lokaal werken blijft beschikbaar.");},
            CONFLICT=>{ensure!(!self.dirty.get(),"Sla eerst de licentie-invoer op");self.resolve()?;},BACKUP=>self.backup(false)?,RESTORE=>{ensure!(!self.dirty.get(),"Sla eerst de licentie-invoer op");self.backup(true)?;},
            LOCK=>{if self.discard(){self.recovery.clear();self.recovery.window.set_visible(false);self.epoch.set(self.epoch.get()+1);*self.connection.borrow_mut()=None;self.clear_clipboard();self.fields[CODE].set_text("");self.fields[SERVERPW].set_text("");self.fields[BACKUPPW].set_text("");*self.state.borrow_mut()=None;*self.password.borrow_mut()=None;if let Some(s)=self.session.borrow_mut().take(){std::thread::spawn(move||sync::logout(&s));}self.fill(None);self.people.clear();self.licenses.clear();self.license_count.set_text("Ontgrendel de kluis om licenties te bekijken");self.owner.set_collection(vec![]);self.fields[PERSON].set_text("");self.status.set_text("Vergrendeld");}},_=>{}
        }Ok(())
    }
    fn unlock(&self)->Result<()> {
        ensure!(self.state.borrow().is_none(),"De kluis is al geopend");let password=Zeroizing::new(self.fields[PW].text());
        database::migrate(&self.path,&self.path.with_extension("lkv"),&password)?;
        let state=if database::exists(&self.path)?{database::load(&self.path,&password)?}else{ensure!(password.chars().count()>=14 && *password==self.fields[REPEAT].text(),"Kies minimaal 14 tekens en herhaal hetzelfde wachtwoord");let s=LocalState::default();database::save(&self.path,&s,&password)?;s};
        self.fields[PW].set_text("");self.fields[REPEAT].set_text("");self.fields[URL].set_text(&state.server_url);
        let config=database::credentials(&self.path,&password)?;
        if let Some(c)=&config{self.fields[URL].set_text(&c.url);self.fields[USER].set_text(&c.user);}
        *self.connection.borrow_mut()=config;*self.password.borrow_mut()=Some(password);*self.state.borrow_mut()=Some(state);self.refresh();
        self.last_sync.set(Instant::now()-Duration::from_secs(31));
        self.status.set_text("Lokale database geopend. Automatische synchronisatie zodra de server bereikbaar is.");Ok(())
    }
    fn refresh(&self){
        self.loading.set(true);let selected_person=self.people.selection().filter(|i|*i>0).and_then(|i|self.people_ids.borrow().get(i-1).copied());if let Some(s)=self.state.borrow().as_ref(){let mut people:Vec<_>=s.entries.iter().filter(|e|e.kind=="person"&&!e.deleted).collect();people.sort_by_key(|p|p.name.to_lowercase());*self.people_ids.borrow_mut()=people.iter().map(|e|e.id).collect();let names:Vec<String>=people.iter().map(|p|p.name.clone()).collect();let mut list=vec!["Alle gebruikers".into()];list.extend(names.clone());let selected=selected_person.and_then(|id|self.people_ids.borrow().iter().position(|p|*p==id)).map(|i|i+1).unwrap_or(0);self.people.set_collection(list);self.people.set_selection(Some(selected));self.owner.set_collection(names);self.status.set_text(&format!("{} wijzigingen · {} conflicten · Laatste sync: {}",s.pending.len(),s.conflicts.len(),s.last_sync.map(|d|d.format("%d-%m-%Y %H:%M").to_string()).unwrap_or_else(||"nog niet".into())));}
        if self.connection.borrow().as_ref().is_some_and(|config|
            !sync::certificate_verification(&config.url, config.verify_certificate)) {
            self.status.set_text(&format!("{} · Certificaatcontrole UIT", self.status.text()));
        }
        self.loading.set(false);self.render_list();
    }
    fn render_list(&self){
        self.loading.set(true);
        if let Some(state) = self.state.borrow().as_ref() {
            let owner = self.people.selection().filter(|i| *i > 0)
                .and_then(|i| self.people_ids.borrow().get(i - 1).copied());
            let query = self.fields[SEARCH].text().trim().to_lowercase();
            let platform = self.filter.selection_string().unwrap_or_default();
            let active: Vec<_> = state.entries.iter()
                .filter(|entry| entry.kind == "license" && !entry.deleted).collect();
            let mut matches: Vec<_> = active.iter().copied().filter(|entry| {
                (owner.is_none() || entry.owner_id == owner)
                    && (platform.is_empty() || platform == "Alle platforms"
                        || entry.installations.iter().any(|item| item.platform == platform))
                    && (query.is_empty() || entry.name.to_lowercase().contains(&query)
                        || entry.software_version.to_lowercase().contains(&query)
                        || entry.installations.iter().any(|item| item.device.to_lowercase().contains(&query)))
            }).collect();
            matches.sort_by_key(|entry| entry.name.to_lowercase());
            let count = match (matches.len(), active.len()) {
                (_, 0) => "Nog geen licenties".to_owned(),
                (0, total) => format!("Geen resultaten · {total} in de kluis"),
                (visible, total) => format!("{visible} van {total} zichtbaar"),
            };
            self.license_count.set_text(&count);
            *self.license_ids.borrow_mut() = matches.iter().map(|entry| entry.id).collect();
            self.licenses.set_collection(matches.iter().map(|entry| {
                let warning = if state.conflicts.contains_key(&entry.id) { "! " } else { "" };
                if entry.software_version.is_empty() {
                    format!("{warning}{}", entry.name)
                } else {
                    format!("{warning}{}  ·  {}", entry.name, entry.software_version)
                }
            }).collect());
        }
        self.loading.set(false);let id=self.selected.get().filter(|id|self.license_ids.borrow().contains(id));if let Some(id)=id{self.licenses.set_selection(self.license_ids.borrow().iter().position(|x|*x==id));}if !self.dirty.get(){self.fill(id);}
    }
    fn fill(&self,id:Option<Uuid>){
        self.loading.set(true);self.selected.set(id);let e=self.state.borrow().as_ref().and_then(|s|s.entries.iter().find(|e|Some(e.id)==id).cloned());
        for f in &self.fields[NAME..=EXPIRY]{f.set_text("");}self.notes.set_text("");self.fields[DEVICE].set_text("");self.fields[CODE].set_password_char(Some('*'));self.revealed.set(None);self.draft_installs.borrow_mut().clear();self.owner.set_selection(Some(0));
        if let Some(e)=e {self.fields[NAME].set_text(&e.name);self.fields[CODE].set_text(&e.code);self.fields[VERSION].set_text(&e.software_version);self.fields[EXPIRY].set_text(&e.expires.map(|d|d.to_string()).unwrap_or_default());self.notes.set_text(&e.notes);self.owner.set_selection(self.people_ids.borrow().iter().position(|p|Some(*p)==e.owner_id));*self.draft_installs.borrow_mut()=e.installations;}
        self.render_installs();self.dirty.set(false);self.loading.set(false);
    }
    fn render_installs(&self){self.installations.set_collection(self.draft_installs.borrow().iter().map(|i|format!("{}  |  {}",i.platform,i.device)).collect());}
    fn save_license(&self)->Result<()>{ensure!(self.fields[DEVICE].text().trim().is_empty(),"Klik eerst op Voeg toe om het ingevoerde apparaat toe te voegen, of maak het apparaatveld leeg");let mut s=self.snapshot()?;let mut e=self.selected.get().and_then(|id|s.entries.iter().find(|e|e.id==id).cloned()).unwrap_or_else(||Entry::new("license",""));e.name=self.fields[NAME].text().trim().into();e.code=self.fields[CODE].text();e.software_version=self.fields[VERSION].text();e.owner_id=self.owner.selection().and_then(|i|self.people_ids.borrow().get(i).copied()).or(e.owner_id);let expiry=self.fields[EXPIRY].text();e.expires=if expiry.trim().is_empty(){None}else{Some(chrono::NaiveDate::parse_from_str(expiry.trim(),"%Y-%m-%d").context("Gebruik jjjj-mm-dd")?)};e.notes=self.notes.text();e.installations=self.draft_installs.borrow().clone();let id=e.id;s.edit(e)?;self.selected.set(Some(id));self.commit(s)}
    fn delete_license(&self)->Result<()>{let mut s=self.snapshot()?;let id=self.selected.get().context("Selecteer een licentie")?;if !self.confirm("Deze licentie verwijderen, ook op de server na synchronisatie?"){return Ok(());}let mut e=s.entries.iter().find(|e|e.id==id).context("Record ontbreekt")?.clone();e.deleted=true;s.edit(e)?;self.selected.set(None);self.commit(s)}
    fn edit_person(&self,action:usize)->Result<()>{let mut s=self.snapshot()?;let id=self.people.selection().filter(|i|*i>0).and_then(|i|self.people_ids.borrow().get(i-1).copied());let mut p=if action==ADDPERSON{Entry::new("person","")}else{s.entries.iter().find(|e|Some(e.id)==id).context("Selecteer eerst een gebruiker")?.clone()};if action==DELPERSON{ensure!(!s.entries.iter().any(|e|e.kind=="license"&&!e.deleted&&e.owner_id==Some(p.id)),"Verplaats of verwijder eerst de licenties van deze gebruiker");if !self.confirm("Gebruiker verwijderen?"){return Ok(());}p.deleted=true;}else{p.name=self.fields[PERSON].text().trim().into();ensure!(!s.entries.iter().any(|e|e.kind=="person"&&!e.deleted&&e.id!=p.id&&e.name.to_lowercase()==p.name.to_lowercase()),"Deze gebruiker bestaat al");}s.edit(p)?;self.commit(s)}
    fn open_server_settings(&self) -> Result<()> {
        let url = sync::server_url(&self.fields[URL].text()).context("Vul bij Instellingen eerst je HTTPS-serveradres in")?;
        open::that(format!("{url}settings#emailSettings"))
            .context("De serverinstellingen konden niet in de browser worden geopend")?;
        Ok(())
    }

    fn refresh_tls_button(&self) {
        let label = match self.connection.borrow().as_ref() {
            Some(config) if !sync::certificate_verification(&config.url, config.verify_certificate) =>
                "Certificaatcontrole UIT · Klik om aan te zetten",
            Some(config) if sync::certificate_verification(&config.url, false) =>
                "Certificaatcontrole AAN · Publiek serveradres",
            Some(_) => "Certificaatcontrole AAN · Klik om uit te zetten",
            None => "Certificaatcontrole instellen na aanmelden",
        };
        self.buttons[TLS_CHECK].set_text(label);
    }

    fn toggle_certificate_verification(&self) -> Result<()> {
        ensure!(!self.sync_busy.get(), "Wacht tot de huidige synchronisatie klaar is");
        let mut config = self.connection.borrow().clone().context("Meld je eerst aan op de server")?;
        ensure!(sync::server_url(&self.fields[URL].text())? == sync::server_url(&config.url)?,
            "Meld je eerst aan op het gewijzigde serveradres");
        ensure!(!sync::certificate_verification(&config.url, false),
            "Voor een publiek serveradres blijft certificaatcontrole ingeschakeld");
        config.verify_certificate = !config.verify_certificate;
        database::remember(&self.path, &config, self.password.borrow().as_ref().context("Vergrendeld")?)?;
        *self.connection.borrow_mut() = Some(config);
        *self.session.borrow_mut() = None;
        self.refresh_tls_button();
        self.begin_work(false)
    }

    fn update_server_ca(&self, import: bool) -> Result<()> {
        use std::io::Read;

        ensure!(!self.sync_busy.get(), "Wacht tot de huidige synchronisatie klaar is");
        let mut config = self.connection.borrow().clone()
            .context("Meld je eerst aan met je serveraccount om de verbinding op te slaan")?;
        let address = sync::server_url(&self.fields[URL].text())?;
        ensure!(address == sync::server_url(&config.url)?,
            "Meld je eerst aan op dit adres voordat je het CA-certificaat wijzigt");
        config.url = address;

        if import {
            let mut dialog = nwg::FileDialog::default();
            nwg::FileDialog::builder().title("Kies het CA-certificaat van je eigen server")
                .action(nwg::FileDialogAction::Open)
                .filters("CA-certificaat(*.crt;*.cer;*.pem)").build(&mut dialog)?;
            self.modal.set(true);
            let accepted = dialog.run(Some(&self.settings_window));
            self.modal.set(false);
            if !accepted { return Ok(()); }

            let file = std::fs::File::open(PathBuf::from(dialog.get_selected_item()?))?;
            let mut bytes = Vec::new();
            const MAX_CERT_SIZE: u64 = 64 * 1024;
            file.take(MAX_CERT_SIZE + 1).read_to_end(&mut bytes)?;
            ensure!(bytes.len() as u64 <= MAX_CERT_SIZE, "Het CA-certificaat is te groot");
            let (der, fingerprint) = sync::parse_server_ca(&bytes)?;
            let prompt = format!(
                "Server: {}\n\nSHA-256-vingerafdruk:\n{}\n\nVergelijk deze met de vingerafdruk uit je Proxmox-console. Komt die overeen?",
                config.url, fingerprint,
            );
            self.modal.set(true);
            let answer = nwg::modal_message(&self.settings_window, &nwg::MessageParams {
                title: "CA-certificaat controleren", content: &prompt,
                buttons: nwg::MessageButtons::YesNo, icons: nwg::MessageIcons::Warning,
            });
            self.modal.set(false);
            if answer != nwg::MessageChoice::Yes { return Ok(()); }
            config.server_ca = Some(der);
            config.verify_certificate = true;
        } else {
            config.server_ca = None;
        }

        database::remember(&self.path, &config, self.password.borrow().as_ref().context("Vergrendeld")?)?;
        *self.connection.borrow_mut() = Some(config);
        self.refresh_tls_button();
        *self.session.borrow_mut() = None;
        self.settings_window.set_visible(false);
        self.begin_work(false)
    }

    fn begin_work(&self,login:bool)->Result<()> {
        ensure!(!self.sync_busy.get(),"Synchronisatie is al bezig");
        let state=self.snapshot()?;ensure!(state.conflicts.is_empty(),"Los eerst de conflicten op");
        if login&&state.vault_id.is_none()&&!self.confirm("Deze kluis koppelen en automatisch synchroniseren? Alle lokale gebruikers en licenties worden naar deze server verzonden. De serveraanmelding wordt versleuteld in je lokale database bewaard."){return Ok(());}
        if login {
            let url = sync::server_url(&self.fields[URL].text())?;
            let server_ca = self.connection.borrow().as_ref()
                .filter(|saved| sync::server_url(&saved.url).ok().as_deref() == Some(url.as_str()))
                .and_then(|saved| saved.server_ca.clone());
            let config = database::Credentials { url: url.clone(), user: self.fields[USER].text(),
                password: self.fields[SERVERPW].text(), server_ca,
                verify_certificate: self.connection.borrow().as_ref()
                    .filter(|saved| sync::server_url(&saved.url).ok().as_deref() == Some(url.as_str()))
                    .is_some_and(|saved| saved.verify_certificate) };
            ensure!(!config.user.trim().is_empty()&&!config.password.is_empty(),"Vul serveraccount en serverwachtwoord in");
            database::remember(&self.path,&config,self.password.borrow().as_ref().context("Vergrendeld")?)?;
            *self.connection.borrow_mut()=Some(config);*self.session.borrow_mut()=None;self.fields[SERVERPW].set_text("");
        }
        let credentials=self.connection.borrow().clone().context("Stel eerst de serververbinding in")?;
        let current=self.session.borrow().clone();let epoch=self.epoch.get();
        let result=self.result.clone();let sender=self.notice.sender();self.sync_busy.set(true);
        self.last_sync.set(Instant::now());
        self.refresh_tls_button();
        self.status.set_text(if sync::certificate_verification(&credentials.url, credentials.verify_certificate) {
            "Synchroniseren op de achtergrond."
        } else {
            "Synchroniseren via HTTPS · certificaatcontrole tijdelijk UIT"
        });
        std::thread::spawn(move||{
            let work=(||->Result<(LocalState,Session)>{
                let session=match current {Some(s)=>s,None=>sync::login(&credentials.url,&credentials.user,&credentials.password,credentials.server_ca.as_deref(),credentials.verify_certificate)?};
                let mut next=state.clone();sync::synchronize(&mut next,&session)?;Ok((next,session))
            })();
            if let Ok(mut slot)=result.lock(){*slot=Some((epoch,state,work.map_err(|e|format!("{e:#}"))));}sender.notice();
        });Ok(())
    }
    fn finish_work(&self){
        self.sync_busy.set(false);self.last_sync.set(Instant::now());
        let result=self.result.lock().ok().and_then(|mut s|s.take());
        if let Some((epoch,sent,result))=result {
            if epoch!=self.epoch.get()||self.state.borrow().is_none(){return;}
            match result {
                Ok((next,session))=>{
                    let applied=(||->Result<()>{
                        let merged=licentiekluis_native::model::reconcile(&sent,&self.snapshot()?,next)?;
                        database::save(&self.path,&merged,self.password.borrow().as_ref().context("Vergrendeld")?)?;
                        *self.state.borrow_mut()=Some(merged);*self.session.borrow_mut()=Some(session);
                        if !self.dirty.get()&&!self.modal.get(){self.refresh();}
                        Ok(())
                    })();
                    if let Err(e)=applied{self.status.set_text(&format!("Lokale opslag niet bijgewerkt: {e}. Opnieuw proberen over 30 seconden."));}
                },
                Err(e)=>{*self.session.borrow_mut()=None;self.status.set_text(&format!("Lokaal werken; sync niet gelukt: {e}. Nieuwe poging over 30 seconden."));}
            }
        }
    }
    fn resolve(&self)->Result<()>{let mut s=self.snapshot()?;let ids:Vec<_>=s.conflicts.keys().copied().collect();if ids.is_empty(){self.status.set_text("Geen conflicten");return Ok(());}for id in ids{let local=s.entries.iter().find(|e|e.id==id).context("Record ontbreekt")?;let remote=&s.conflicts[&id];let describe=|e:&Entry|format!("{}\nVersie: {}\nVerwijderd: {}\nInstallaties: {}\nOpmerkingen: {}",e.name,e.software_version,e.deleted,e.installations.iter().map(|i|format!("{} / {}",i.platform,i.device)).collect::<Vec<_>>().join(", "),e.notes);let text=format!("LOKAAL\n{}\n\nSERVER\n{}\n\nCode of herstelinhoud verschilt: {}\n\nJa: lokale versie behouden\nNee: serverversie overnemen\nAnnuleren: later beslissen",describe(local),describe(remote),local.code!=remote.code || local.recovery!=remote.recovery);self.modal.set(true);let answer=nwg::modal_message(&self.window,&nwg::MessageParams{title:"Synchronisatieconflict",content:&text,buttons:nwg::MessageButtons::YesNoCancel,icons:nwg::MessageIcons::Warning});self.modal.set(false);if answer==nwg::MessageChoice::Cancel{break;}s.resolve(id,answer==nwg::MessageChoice::Yes)?;}self.commit(s)}
    fn backup(&self,restore:bool)->Result<()>{let mut dialog=nwg::FileDialog::default();nwg::FileDialog::builder().title(if restore{"Back-up herstellen (ook .NET 0.1)"}else{"Versleutelde back-up opslaan"}).action(if restore{nwg::FileDialogAction::Open}else{nwg::FileDialogAction::Save}).filters("Kluis(*.lkv)").build(&mut dialog)?;self.modal.set(true);let accepted=dialog.run(Some(&self.window));self.modal.set(false);if !accepted{return Ok(());}let path=PathBuf::from(dialog.get_selected_item()?);ensure!(path!=self.path,"Kies een andere locatie dan de actieve kluis");if restore{let password=Zeroizing::new(self.fields[BACKUPPW].text());self.fields[BACKUPPW].set_text("");let s=vault::load(&path,&password)?;if !self.confirm("De huidige lokale kluis vervangen? Er wordt eerst een herstelkopie gemaakt. Gebruik daarna alleen deze client voor nieuwe wijzigingen."){return Ok(());}let before=self.path.with_file_name(format!("before-restore-{}.lkv",chrono::Local::now().format("%Y%m%d-%H%M%S")));vault::save(&before,&self.snapshot()?,self.password.borrow().as_ref().context("Vergrendeld")?)?;*self.session.borrow_mut()=None;self.fields[URL].set_text(&s.server_url);self.commit(s)?;database::forget(&self.path)?;*self.connection.borrow_mut()=None;self.epoch.set(self.epoch.get()+1);}else{vault::save(&path,&self.snapshot()?,self.password.borrow().as_ref().context("Vergrendeld")?)?;self.status.set_text("Back-up opgeslagen; bewaar het lokale kluiswachtwoord voor herstel.");}Ok(())}
    fn open_mail(&self)->Result<()> {
        ensure!(!self.dirty.get(),"Sla eerst de licentie-invoer op");let s=self.snapshot()?;
        let owner=self.people.selection().filter(|i|*i>0).and_then(|i|self.people_ids.borrow().get(i-1).copied());
        let mut entries:Vec<_>=s.entries.into_iter().filter(|e|e.kind=="license"&&!e.deleted&&(owner.is_none()||e.owner_id==owner)).collect();entries.sort_by_key(|e|e.name.to_lowercase());ensure!(!entries.is_empty(),"Geen licenties bij deze gebruiker");
        self.mail_licenses.set_collection(entries.iter().map(|e|format!("{}  |  versie {}",e.name,e.software_version)).collect());*self.mail_entries.borrow_mut()=entries;*self.mail_draft.borrow_mut()=None;self.mail_preview.set_text("");self.modal.set(true);self.window.set_enabled(false);self.mail_window.set_visible(true);Ok(())
    }
    fn close_mail(&self){self.mail_window.set_visible(false);self.window.set_enabled(true);self.modal.set(false);self.mail_preview.set_text("");self.mail_to.set_text("");self.mail_entries.borrow_mut().clear();self.mail_licenses.clear();*self.mail_draft.borrow_mut()=None;}
    fn mail_event(&self,event:nwg::Event,handle:nwg::ControlHandle){
        if self.busy.get(){return;}if event==nwg::Event::OnWindowClose{self.close_mail();return;}
        if event==nwg::Event::OnListBoxSelect||(event==nwg::Event::OnTextInput&&(handle==self.mail_to.handle||handle==self.mail_subject.handle)){*self.mail_draft.borrow_mut()=None;self.mail_preview.set_text("");}
        if event!=nwg::Event::OnButtonClick{return;}let Some(i)=self.mail_buttons.iter().position(|b|b.handle==handle)else{return;};
        let result=(||->Result<()>{match i{
            6=>self.open_server_settings()?,
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
    fn tick(&self){self.recovery.tick();let expire=self.copied.borrow().as_ref().is_some_and(|(_,t)|t.elapsed()>Duration::from_secs(30));if expire{self.clear_clipboard();}if self.revealed.get().is_some_and(|t|t.elapsed()>Duration::from_secs(30)){self.fields[CODE].set_password_char(Some('*'));self.revealed.set(None);}if !self.busy.get()&&!self.sync_busy.get()&&!self.modal.get()&&self.state.borrow().is_some()&&self.connection.borrow().is_some()&&self.last_sync.get().elapsed()>Duration::from_secs(30){self.last_sync.set(Instant::now());if let Err(e)=self.begin_work(false){self.status.set_text(&e.to_string());}}}
}

impl Drop for App {
    fn drop(&mut self){for handler in &self.paint_handlers {let _=nwg::unbind_raw_event_handler(handler);}}
}

