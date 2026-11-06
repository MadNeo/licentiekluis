//! Backgrounds belong to the parent; direct child controls fit the available client area.
use native_windows_gui as nwg;
use super::theme::Theme;
use std::rc::Rc;
use winapi::shared::{windef::{HDC, HWND, POINT, RECT}, minwindef::{BOOL, LPARAM}};
use winapi::um::wingdi::{SetBkMode, TRANSPARENT};
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

pub fn bind(window: &nwg::Window, panels: Vec<Panel>, design: (i32, i32), theme: Rc<Theme>)
    -> Result<nwg::RawEventHandler, nwg::NwgError>
{
    nwg::bind_raw_event_handler(&window.handle, 0x4C4B0001, move |hwnd, msg, w, l| {
        if msg == WM_DRAWITEM { if unsafe { super::theme::draw_item(l,&theme) } { return Some(1); } }
        if [WM_CTLCOLORSTATIC,WM_CTLCOLOREDIT,WM_CTLCOLORLISTBOX,WM_CTLCOLORBTN,WM_CTLCOLORDLG].contains(&msg) {
            unsafe {
                let dc=w as HDC;SetBkMode(dc,TRANSPARENT as i32);winapi::um::wingdi::SetTextColor(dc,theme.ink());winapi::um::wingdi::SetBkColor(dc,theme.background());
                let mut color=[255,255,255];
                if msg==WM_CTLCOLORSTATIC && super::theme::class(l as HWND)=="Static" {
                    let mut r:RECT=std::mem::zeroed();GetWindowRect(l as HWND,&mut r);let mut point=POINT{x:r.left,y:r.top};ScreenToClient(hwnd,&mut point);
                    let mut client:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut client);
                    let x=point.x*design.0/client.right.max(1);let y=point.y*design.1/client.bottom.max(1);
                    if let Some(p)=panels.iter().rev().find(|p|x>=p.0&&y>=p.1&&x<p.0+p.2&&y<p.1+p.3){color=p.4;}
                }
                return Some(theme.brush(color) as isize);
            }
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
                let brush = theme.brush(rgb);
                if !brush.is_null() {
                    FillRect(dc, &rect, brush);

                }
            }
        }
        Some(1)
    })
}

