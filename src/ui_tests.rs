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
    let mut msg: MSG = mem::zeroed();
    while PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
        TranslateMessage(&msg); DispatchMessageW(&msg);
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

unsafe fn snapshot(hwnd: HWND, path: &std::path::Path) {
    let mut rect: RECT = mem::zeroed(); GetWindowRect(hwnd, &mut rect);
    let width = rect.right-rect.left; let height = rect.bottom-rect.top;
    let screen = GetDC(hwnd); let dc = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, width, height);
    let old = SelectObject(dc, bitmap as _);
    assert_ne!(PrintWindow(hwnd, dc, 2), 0, "Native window capture failed");
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
    let app = App::build(temp.path().join("never-created.lkv")).unwrap();
    app.status.set_text("Vergrendeld · Je gegevens blijven lokaal versleuteld.");
    let screenshots = PathBuf::from("target/ui-check");
    std::fs::create_dir_all(&screenshots).unwrap();
    unsafe {
        paint_now(&app.window);
        snapshot(app.window.handle.hwnd().unwrap(), &screenshots.join("00-before-check.bmp"));
        for field in &app.fields[..URL] { reachable(&field.handle); }
        for index in [UNLOCK,NEW,SAVE,DELETE,COPY,SHOW,ADDPERSON,RENAME,DELPERSON,ADDINSTALL,DELINSTALL,LOCK,MAIL,19] {
            reachable(&app.buttons[index].handle);
            assert!(!app.buttons[index].text().trim().is_empty());
        }
        for handle in [&app.people.handle,&app.licenses.handle,&app.owner.handle,
            &app.platform.handle,&app.filter.handle,&app.installations.handle,&app.notes.handle] { reachable(handle); }
        // Decorative rectangles must not be native child labels.
        assert!(app.labels.iter().all(|label| !label.text().trim().is_empty()));
        snapshot(app.window.handle.hwnd().unwrap(), &screenshots.join("01-main.bmp"));
        paint_now(&app.settings_window);
        for field in &app.fields[URL..] { reachable(&field.handle); }
        for index in [LOGIN,SYNC,LOGOUT,CONFLICT,BACKUP,RESTORE] { reachable(&app.buttons[index].handle); }
        snapshot(app.settings_window.handle.hwnd().unwrap(), &screenshots.join("02-settings.bmp"));
        app.settings_window.set_visible(false);
        paint_now(&app.mail_window);
        for button in &app.mail_buttons { reachable(&button.handle); }
        for handle in [&app.mail_licenses.handle,&app.mail_to.handle,&app.mail_subject.handle,&app.mail_preview.handle] { reachable(handle); }
        snapshot(app.mail_window.handle.hwnd().unwrap(), &screenshots.join("03-mail.bmp"));
    }
    assert!(!temp.path().join("never-created.lkv").exists());
}
