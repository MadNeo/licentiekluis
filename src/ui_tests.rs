//! Native Windows rendering regression. Uses a temporary path, never the user's vault.
use super::*;
use std::{mem, ptr};
use winapi::shared::windef::{HWND, POINT, RECT};
use winapi::um::winuser::*;
use winapi::um::wingdi::*;

unsafe fn paint_now(window: &nwg::Window) {
    let hwnd = window.handle.hwnd().unwrap();
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    RedrawWindow(hwnd, ptr::null(), ptr::null_mut(),
        RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW | RDW_FRAME);
    // Give newly shown owned windows time to paint before capturing them.
    for _ in 0..20 {
        let mut msg: MSG = mem::zeroed();
        while PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg); DispatchMessageW(&msg);
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

unsafe fn reachable(handle: &nwg::ControlHandle) {
    let hwnd = handle.hwnd().unwrap();
    assert_ne!(IsWindowVisible(hwnd), 0, "Control must be visible");
    let parent = GetParent(hwnd);
    let mut rect: RECT = mem::zeroed();
    assert_ne!(GetWindowRect(hwnd, &mut rect), 0);
    let mut point = POINT { x: (rect.left+rect.right)/2, y: (rect.top+rect.bottom)/2 };
    ScreenToClient(parent, &mut point);
    let hit = ChildWindowFromPointEx(parent, point, CWP_SKIPINVISIBLE | CWP_SKIPTRANSPARENT);
    let mut parent_rect: RECT = mem::zeroed(); GetClientRect(parent, &mut parent_rect);
    let mut caption=[0u16;256];let len=GetWindowTextW(hwnd,caption.as_mut_ptr(),256);
    assert_eq!(hit, hwnd, "Control {:?} at ({},{}) outside/covered; parent client {}x{}",String::from_utf16_lossy(&caption[..len as usize]),point.x,point.y,parent_rect.right,parent_rect.bottom);
}

unsafe fn snapshot(hwnd: HWND, path: &std::path::Path, dark: bool) {
    let mut rect: RECT = mem::zeroed(); GetWindowRect(hwnd, &mut rect);
    let width = rect.right-rect.left; let height = rect.bottom-rect.top;
    let screen = GetDC(hwnd); let dc = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, width, height);
    let old = SelectObject(dc, bitmap as _);
    assert_ne!(PrintWindow(hwnd, dc, 0), 0, "Native window capture failed");
    SelectObject(dc, old);
    let mut info: BITMAPINFO = mem::zeroed();
    info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = width; info.bmiHeader.biHeight = -height;
    info.bmiHeader.biPlanes = 1; info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    let mut pixels = vec![0u8; (width*height*4) as usize];
    let lines = GetDIBits(dc, bitmap, 0, height as u32, pixels.as_mut_ptr() as _, &mut info, DIB_RGB_COLORS);
    DeleteObject(bitmap as _); DeleteDC(dc); ReleaseDC(hwnd, screen);
    assert_eq!(lines, height, "Pixel capture incomplete");
    let bright = pixels.chunks_exact(4).filter(|p| p[0] > 180 && p[1] > 180 && p[2] > 180).count();
    if dark {let dark_pixels=pixels.chunks_exact(4).filter(|p|p[0]<90&&p[1]<90&&p[2]<90).count();assert!(dark_pixels>(width*height/3) as usize,"Dark backgrounds must be rendered");assert!(bright>100,"Dark mode must retain visible text");}else{assert!(bright > (width*height/3) as usize, "Window capture must show its light interface, not a blank black frame");}
    let mut out = Vec::new();
    out.extend_from_slice(b"BM"); out.extend_from_slice(&(54+pixels.len() as u32).to_le_bytes());
    out.extend_from_slice(&[0;4]); out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes()); out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&(-height).to_le_bytes()); out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes()); out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(pixels.len() as u32).to_le_bytes()); out.extend_from_slice(&[0;16]);
    out.extend_from_slice(&pixels); std::fs::write(path,out).unwrap();
}

#[test]
fn native_windows_show_text_and_unobstructed_controls() {
    nwg::init().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let app = App::build(temp.path().join("vault.db")).unwrap();
    app.status.set_text("Vergrendeld · Je gegevens blijven lokaal versleuteld.");
    let screenshots = PathBuf::from("target/ui-check");
    std::fs::create_dir_all(&screenshots).unwrap();
    unsafe {
        paint_now(&app.window);
        snapshot(app.window.handle.hwnd().unwrap(), &screenshots.join("00-before-check.bmp"),false);
        for field in &app.fields[..URL] { reachable(&field.handle); }
        for index in [UNLOCK,NEW,SAVE,DELETE,COPY,SHOW,ADDPERSON,RENAME,DELPERSON,ADDINSTALL,DELINSTALL,LOCK,MAIL,19] {
            reachable(&app.buttons[index].handle);
            assert!(!app.buttons[index].text().trim().is_empty());
        }
        for handle in [&app.people.handle,&app.licenses.handle,&app.owner.handle,
            &app.platform.handle,&app.filter.handle,&app.installations.handle,&app.notes.handle] { reachable(handle); }
        // Decorative rectangles must not be native child labels.
        assert!(app.labels.iter().all(|label| !label.text().trim().is_empty()));
        snapshot(app.window.handle.hwnd().unwrap(), &screenshots.join("01-main.bmp"),false);
        paint_now(&app.settings_window);
        for field in &app.fields[URL..] { reachable(&field.handle); }
        for index in [LOGIN,SYNC,LOGOUT,CONFLICT,BACKUP,RESTORE,IMPORT_CA,REMOVE_CA] { reachable(&app.buttons[index].handle); }
        snapshot(app.settings_window.handle.hwnd().unwrap(), &screenshots.join("02-settings.bmp"),false);
        app.event(nwg::Event::OnButtonClick,app.theme_button.handle.clone());
        assert!(app.theme.dark.get());assert!(theme::Theme::new(&app.theme_path).dark.get());
        paint_now(&app.settings_window);snapshot(app.settings_window.handle.hwnd().unwrap(),&screenshots.join("04-settings-dark.bmp"),true);
        app.settings_window.set_visible(false);
        paint_now(&app.window);
        let tab=app.tabs.handle.hwnd().unwrap();let mut tab_rect:RECT=mem::zeroed();GetClientRect(tab,&mut tab_rect);let tab_dc=GetDC(tab);
        assert_eq!(GetPixel(tab_dc,tab_rect.right-2,2),app.theme.background(),"Tab margins must follow dark theme");ReleaseDC(tab,tab_dc);
        let title=app.labels[0].handle.hwnd().unwrap();let dc=GetDC(title);
        assert_eq!(GetPixel(dc,1,1),app.theme.background(),"Label margins must follow dark theme, not retain white strips");ReleaseDC(title,dc);
        paint_now(&app.mail_window);
        for button in &app.mail_buttons { reachable(&button.handle); }
        for handle in [&app.mail_licenses.handle,&app.mail_to.handle,&app.mail_subject.handle,&app.mail_preview.handle] { reachable(handle); }
        snapshot(app.mail_window.handle.hwnd().unwrap(), &screenshots.join("05-mail-dark.bmp"),true);
        app.mail_window.set_visible(false);paint_now(&app.window);snapshot(app.window.handle.hwnd().unwrap(),&screenshots.join("06-main-dark.bmp"),true);
        reachable(&app.theme_button.handle);assert_eq!(app.tabs.tab_count(),2);
        app.tabs.set_selected_tab(1);app.event(nwg::Event::TabsContainerChanged,app.tabs.handle.clone());paint_now(&app.window);
        reachable(&app.changelog.handle);assert!(app.changelog.text().contains(env!("CARGO_PKG_VERSION")));assert_eq!(IsWindowVisible(app.fields[NAME].handle.hwnd().unwrap()),0);
        snapshot(app.window.handle.hwnd().unwrap(),&screenshots.join("07-changelog-dark.bmp"),true);
        app.tabs.set_selected_tab(0);app.select_tab();reachable(&app.fields[NAME].handle);
        app.set_theme(false).unwrap();paint_now(&app.window);snapshot(app.window.handle.hwnd().unwrap(),&screenshots.join("08-main-light.bmp"),false);
        assert!(!theme::Theme::new(&app.theme_path).dark.get());
    }
    // Draft preservation, version search, deleted-owner edits and device validation.
    let mut state=LocalState::default();let person=Entry::new("person","Voorbeeldgebruiker");state.edit(person.clone()).unwrap();
    let mut license=Entry::new("license","Microsoft Office");license.owner_id=Some(person.id);license.code="TEST-CODE-ONLY".into();license.software_version="2026".into();license.installations.push(Installation{platform:"Windows".into(),device:"Laptop kantoor".into()});state.edit(license.clone()).unwrap();
    *app.state.borrow_mut()=Some(state);*app.password.borrow_mut()=Some(Zeroizing::new("synthetic-test-password".into()));app.refresh();app.fill(Some(license.id));
    app.fields[SEARCH].set_text("2026");app.render_list();assert!(app.license_ids.borrow().contains(&license.id),"Search must include version");
    app.fields[NAME].set_text("Mijn onopgeslagen wijziging");app.dirty.set(true);app.fields[SEARCH].set_text("geen overeenkomst");app.render_list();
    assert_eq!(app.fields[NAME].text(),"Mijn onopgeslagen wijziging");assert_eq!(app.selected.get(),Some(license.id));assert!(app.dirty.get());
    app.fields[DEVICE].set_text("Nog toevoegen");assert!(app.save_license().is_err(),"Pending device must not be silently dropped");app.action(ADDINSTALL).unwrap();assert!(app.fields[DEVICE].text().is_empty());
    app.fields[SEARCH].set_text("");app.save_license().unwrap();assert!(!app.dirty.get());assert_eq!(database::load(&app.path,"synthetic-test-password").unwrap().entries.iter().find(|e|e.id==license.id).unwrap().installations.len(),2);
    app.state.borrow_mut().as_mut().unwrap().entries.iter_mut().find(|e|e.id==person.id).unwrap().deleted=true;
    app.refresh();app.fill(Some(license.id));assert_eq!(app.owner.selection(),None);app.fields[VERSION].set_text("2027");app.save_license().unwrap();
    assert_eq!(app.snapshot().unwrap().entries.iter().find(|e|e.id==license.id).unwrap().owner_id,Some(person.id));
    app.state.borrow_mut().as_mut().unwrap().entries.iter_mut().find(|e|e.id==person.id).unwrap().deleted=false;app.refresh();app.fill(Some(license.id));
    unsafe{paint_now(&app.window);snapshot(app.window.handle.hwnd().unwrap(),&screenshots.join("09-filled-light.bmp"),false);}
    app.set_theme(true).unwrap();unsafe{paint_now(&app.window);snapshot(app.window.handle.hwnd().unwrap(),&screenshots.join("10-filled-dark.bmp"),true);}
    app.fields[BACKUPPW].set_text("temporary-backup-password");app.action(LOCK).unwrap();assert!(app.fields[BACKUPPW].text().is_empty());assert!(app.fields[CODE].text().is_empty());
    assert!(!temp.path().join("never-created.lkv").exists());
}

