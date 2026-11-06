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
                if (msg==WM_CTLCOLORSTATIC && super::theme::class(l as HWND).eq_ignore_ascii_case("static")) || msg==WM_CTLCOLORBTN {
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
                    if x>0 && width>150 && height>60 {
                        use winapi::um::wingdi::*;
                        let pen=CreatePen(PS_SOLID as i32,1,theme.border());let old_pen=SelectObject(dc,pen as _);let old_brush=SelectObject(dc,brush as _);
                        RoundRect(dc,rect.left,rect.top,rect.right,rect.bottom,16,16);
                        SelectObject(dc,old_pen);SelectObject(dc,old_brush);DeleteObject(pen as _);
                    } else {FillRect(dc, &rect, brush);}

                }
            }
        }
        Some(1)
    })
}


/// NWG's label subclass paints fixed-color non-client strips. Own the whole
/// label client area so both text and margins follow the active theme.
pub fn bind_label(label:&nwg::Label, theme:Rc<Theme>)->Result<nwg::RawEventHandler,nwg::NwgError> {
    let handler=nwg::bind_raw_event_handler(&label.handle,0x4C4B0002,move|hwnd,msg,_,_| {
        if msg==WM_NCCALCSIZE||msg==WM_NCPAINT{return Some(0);}
        if msg!=WM_PAINT{return None;}
        unsafe {
            use winapi::um::wingdi::*;
            let mut ps:PAINTSTRUCT=std::mem::zeroed();let dc=BeginPaint(hwnd,&mut ps);
            let mut rect:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rect);
            let brush=SendMessageW(GetParent(hwnd),WM_CTLCOLORSTATIC,dc as usize,hwnd as isize) as winapi::shared::windef::HBRUSH;
            FillRect(dc,&rect,brush);SetBkMode(dc,TRANSPARENT as i32);SetTextColor(dc,theme.ink());
            let font=SendMessageW(hwnd,WM_GETFONT,0,0);let old=SelectObject(dc,font as _);
            let len=GetWindowTextLengthW(hwnd);let mut text=vec![0u16;len as usize+1];GetWindowTextW(hwnd,text.as_mut_ptr(),len+1);
            DrawTextW(dc,text.as_ptr(),len,&mut rect,DT_LEFT|DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS|DT_NOPREFIX);
            SelectObject(dc,old);EndPaint(hwnd,&ps);
        }
        Some(0)
    })?;
    unsafe{SetWindowPos(label.handle.hwnd().unwrap(),std::ptr::null_mut(),0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE|SWP_FRAMECHANGED);}
    Ok(handler)
}

/// Preserve the native tab keyboard/click behavior while painting its full
/// client area; the classic control otherwise leaves a white strip in dark mode.
pub fn bind_tabs(tabs:&nwg::TabsContainer,theme:Rc<Theme>)->Result<nwg::RawEventHandler,nwg::NwgError>{
    nwg::bind_raw_event_handler(&tabs.handle,0x4C4B0003,move|hwnd,msg,w,_|{
        if msg!=WM_PAINT&&msg!=WM_ERASEBKGND{return None;}
        unsafe{
            use winapi::um::{wingdi::*,commctrl::*};
            let mut ps:PAINTSTRUCT=std::mem::zeroed();let dc=if msg==WM_PAINT{BeginPaint(hwnd,&mut ps)}else{w as HDC};
            let mut rect:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rect);FillRect(dc,&rect,theme.brush([255,255,255]));
            if msg==WM_PAINT{
                let font=SendMessageW(hwnd,WM_GETFONT,0,0);let old_font=SelectObject(dc,font as _);SetBkMode(dc,TRANSPARENT as i32);
                let selected=SendMessageW(hwnd,TCM_GETCURSEL,0,0);let count=SendMessageW(hwnd,TCM_GETITEMCOUNT,0,0);
                for i in 0..count{
                    let mut item:RECT=std::mem::zeroed();SendMessageW(hwnd,TCM_GETITEMRECT,i as usize,&mut item as *mut _ as isize);
                    let active=i==selected;let brush=theme.brush(if active{[20,139,125]}else{[255,255,255]});
                    let old_brush=SelectObject(dc,brush as _);let pen=CreatePen(PS_SOLID as i32,1,theme.border());let old_pen=SelectObject(dc,pen as _);
                    RoundRect(dc,item.left,item.top,item.right,item.bottom,10,10);SelectObject(dc,old_brush);SelectObject(dc,old_pen);DeleteObject(pen as _);
                    let mut text=[0u16;128];let mut info:TCITEMW=std::mem::zeroed();info.mask=TCIF_TEXT;info.pszText=text.as_mut_ptr();info.cchTextMax=128;SendMessageW(hwnd,TCM_GETITEMW,i as usize,&mut info as *mut _ as isize);
                    SetTextColor(dc,if active{RGB(255,255,255)}else{theme.ink()});DrawTextW(dc,text.as_ptr(),-1,&mut item,DT_CENTER|DT_VCENTER|DT_SINGLELINE);
                }
                SelectObject(dc,old_font);EndPaint(hwnd,&ps);
            }
        }
        Some(1)
    })
}
