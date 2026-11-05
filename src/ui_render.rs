//! Backgrounds belong to the parent; direct child controls fit the available client area.
use native_windows_gui as nwg;
use winapi::shared::{windef::{HDC, HWND, POINT, RECT}, minwindef::{BOOL, LPARAM}};
use winapi::um::wingdi::{CreateSolidBrush, DeleteObject, RGB, SetBkMode, TRANSPARENT};
use winapi::um::winuser::*;

pub type Panel = (i32, i32, i32, i32, [u8; 3]);

// Windows can constrain initial top-level window dimensions to the monitor.
// Fit actual child bounds to that viewport, keeping text at a readable size.
pub fn fit_children(window: &nwg::Window, design: (i32, i32)) {
    struct Layout { parent: HWND, children: Vec<(HWND, RECT)> }
    unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
        let layout = &mut *(data as *mut Layout);
        if GetParent(hwnd) == layout.parent {
            let mut rect: RECT = std::mem::zeroed(); GetWindowRect(hwnd, &mut rect);
            let mut origin = POINT { x: rect.left, y: rect.top };
            ScreenToClient(layout.parent, &mut origin);
            rect.right -= rect.left; rect.bottom -= rect.top;
            rect.left = origin.x; rect.top = origin.y;
            layout.children.push((hwnd,rect));
        }
        1
    }
    unsafe {
        let hwnd = window.handle.hwnd().unwrap();
        let mut client: RECT = std::mem::zeroed(); GetClientRect(hwnd,&mut client);
        let dpi = (GetDpiForWindow(hwnd) as i32).max(96);
        let design_w = design.0*dpi/96; let design_h = design.1*dpi/96;
        let mut layout = Layout { parent:hwnd,children:vec![] };
        EnumChildWindows(hwnd,Some(collect),&mut layout as *mut _ as LPARAM);
        for (child,r) in layout.children {
            SetWindowPos(child,std::ptr::null_mut(),r.left*client.right/design_w,
                r.top*client.bottom/design_h,(r.right*client.right/design_w).max(1),
                (r.bottom*client.bottom/design_h).max(1),SWP_NOZORDER|SWP_NOACTIVATE);
        }
    }
}

pub fn bind(window: &nwg::Window, panels: Vec<Panel>, design: (i32, i32))
    -> Result<nwg::RawEventHandler, nwg::NwgError>
{
    nwg::bind_raw_event_handler(&window.handle, 0x4C4B0001, move |hwnd, msg, w, _l| {
        if msg == WM_CTLCOLORSTATIC {
            // Preserve the label's colored brush without white boxes behind its text.
            unsafe { SetBkMode(w as HDC, TRANSPARENT); }
            return None;
        }
        if msg != WM_ERASEBKGND { return None; }
        unsafe {
            let mut client: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut client);
            let dc = w as HDC;
            for &(x, y, width, height, rgb) in &panels {
                let rect = RECT {
                    left: x * client.right / design.0,
                    top: y * client.bottom / design.1,
                    right: (x + width) * client.right / design.0,
                    bottom: (y + height) * client.bottom / design.1,
                };
                let brush = CreateSolidBrush(RGB(rgb[0], rgb[1], rgb[2]));
                if !brush.is_null() {
                    FillRect(dc, &rect, brush);
                    DeleteObject(brush as _);
                }
            }
        }
        Some(1)
    })
}
