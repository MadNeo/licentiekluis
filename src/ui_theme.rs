//! Theme preferences never contain vault data or credentials.
use std::{cell::Cell,path::Path,ptr};
use native_windows_gui as nwg;
use winapi::{shared::{minwindef::{BOOL,LPARAM},windef::{HWND,HBRUSH}},um::{winuser::*,wingdi::*,commctrl::*}};
#[link(name="uxtheme")] extern "system" {fn SetWindowTheme(hwnd:HWND,sub:*const u16,list:*const u16)->i32;}
#[link(name="dwmapi")] extern "system" {fn DwmSetWindowAttribute(hwnd:HWND,attribute:u32,value:*const i32,size:u32)->i32;}
pub struct Theme {pub dark:Cell<bool>,brushes:Vec<HBRUSH>}
impl Theme {
 pub fn new(path:&Path)->Self {let dark=std::fs::read_to_string(path).map(|s|s.trim()=="dark").unwrap_or(false);let brushes=[[243,245,249],[255,255,255],[231,244,239],[20,110,101],[13,21,34],[23,34,52],[24,60,57],[21,121,108]].iter().map(|c|unsafe{CreateSolidBrush(RGB(c[0],c[1],c[2]))}).collect();Self{dark:Cell::new(dark),brushes}}
 pub fn save(&self,path:&Path)->anyhow::Result<()> {use std::io::Write;let mut file=tempfile::NamedTempFile::new_in(path.parent().unwrap())?;file.write_all(if self.dark.get(){b"dark\n"}else{b"light\n"})?;file.as_file().sync_all()?;file.persist(path).map_err(|e|e.error)?;Ok(())}
 pub fn brush(&self,color:[u8;3])->HBRUSH {let index=match color{[255,255,255]=>1,[228,245,241]=>2,[20,139,125]=>3,_=>0};self.brushes[index+if self.dark.get(){4}else{0}]}
 pub fn ink(&self)->u32 {if self.dark.get(){RGB(232,239,247)}else{RGB(22,52,72)}}
 pub fn border(&self)->u32 {if self.dark.get(){RGB(51,69,93)}else{RGB(220,227,237)}}
 pub fn background(&self)->u32 {if self.dark.get(){RGB(23,34,52)}else{RGB(255,255,255)}}
}
impl Drop for Theme {fn drop(&mut self){for brush in &self.brushes{unsafe{DeleteObject(*brush as _);}}}}
pub unsafe fn class(hwnd:HWND)->String {let mut text=[0u16;64];let n=GetClassNameW(hwnd,text.as_mut_ptr(),64);String::from_utf16_lossy(&text[..n as usize])}
pub fn prepare(window:&nwg::Window){
 unsafe extern "system" fn child(hwnd:HWND,_:LPARAM)->BOOL {let empty=[0u16];SetWindowTheme(hwnd,empty.as_ptr(),empty.as_ptr());let name=class(hwnd);let style=GetWindowLongW(hwnd,GWL_STYLE);if name=="Button"{SetWindowLongW(hwnd,GWL_STYLE,(style & !15)|BS_OWNERDRAW as i32);}if name=="SysTabControl32"{SetWindowLongW(hwnd,GWL_STYLE,style|TCS_OWNERDRAWFIXED as i32|TCS_BUTTONS as i32|TCS_FLATBUTTONS as i32);}1}
 unsafe {EnumChildWindows(window.handle.hwnd().unwrap(),Some(child),0);}
}
pub fn repaint(window:&nwg::Window,dark:bool){unsafe{let hwnd=window.handle.hwnd().unwrap();let value=if dark{1}else{0};DwmSetWindowAttribute(hwnd,20,&value,4);RedrawWindow(hwnd,ptr::null(),ptr::null_mut(),RDW_INVALIDATE|RDW_ERASE|RDW_ALLCHILDREN|RDW_UPDATENOW|RDW_FRAME);}}
pub unsafe fn draw_item(l:LPARAM,theme:&Theme)->bool {
 let item=&*(l as *const DRAWITEMSTRUCT);if item.CtlType!=ODT_BUTTON&&item.CtlType!=ODT_TAB{return false;}
 let dc=item.hDC;let selected=item.itemState&ODS_SELECTED!=0;
 let mut text=[0u16;512];if item.CtlType==ODT_TAB{let mut info:TCITEMW=std::mem::zeroed();info.mask=TCIF_TEXT;info.pszText=text.as_mut_ptr();info.cchTextMax=512;SendMessageW(item.hwndItem,TCM_GETITEMW,item.itemID as usize,&mut info as *mut _ as LPARAM);}else{GetWindowTextW(item.hwndItem,text.as_mut_ptr(),512);}
 let caption=String::from_utf16_lossy(&text[..text.iter().position(|c|*c==0).unwrap_or(text.len())]);
 let primary=item.CtlType==ODT_BUTTON&&["Opslaan","Ontgrendelen","Nieuwe licentie","Aanmelden","Voorbeeld maken"].contains(&caption.as_str());
 let color=if primary{[20,139,125]}else if selected{[228,245,241]}else{[255,255,255]};
 let brush=theme.brush(color);let pen=CreatePen(PS_SOLID,1,if primary{if theme.dark.get(){RGB(21,121,108)}else{RGB(20,110,101)}}else{theme.border()});
 let old_brush=SelectObject(dc,brush as _);let old_pen=SelectObject(dc,pen as _);
 let radius=(12*GetDpiForWindow(item.hwndItem)/96) as i32;
 RoundRect(dc,item.rcItem.left,item.rcItem.top,item.rcItem.right,item.rcItem.bottom,radius,radius);
 SelectObject(dc,old_brush);SelectObject(dc,old_pen);DeleteObject(pen as _);
 let font=SendMessageW(item.hwndItem,WM_GETFONT,0,0);let old=if font!=0{SelectObject(dc,font as _)}else{ptr::null_mut()};SetBkMode(dc,TRANSPARENT as i32);SetTextColor(dc,if item.itemState&ODS_DISABLED!=0{RGB(124,139,152)}else if primary{RGB(255,255,255)}else{theme.ink()});let mut rect=item.rcItem;DrawTextW(dc,text.as_ptr(),-1,&mut rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS);if !old.is_null(){SelectObject(dc,old);}if item.itemState&ODS_FOCUS!=0{InflateRect(&mut rect,-3,-3);DrawFocusRect(dc,&rect);}true
}
pub fn body_children(window:&nwg::Window)->Vec<HWND>{
 struct Collect{parent:HWND,children:Vec<HWND>,threshold:i32}
 unsafe extern "system" fn child(hwnd:HWND,data:LPARAM)->BOOL{let c=&mut *(data as *mut Collect);if GetParent(hwnd)==c.parent{let mut rect=std::mem::zeroed();GetWindowRect(hwnd,&mut rect);let mut p=winapi::shared::windef::POINT{x:rect.left,y:rect.top};ScreenToClient(c.parent,&mut p);if p.y>=c.threshold{c.children.push(hwnd);}}1}
 unsafe{let hwnd=window.handle.hwnd().unwrap();let mut c=Collect{parent:hwnd,children:vec![],threshold:80*(GetDpiForWindow(hwnd) as i32).max(96)/96};EnumChildWindows(hwnd,Some(child),&mut c as *mut _ as LPARAM);c.children}
}
pub fn show_body(children:&[HWND],visible:bool){for hwnd in children{unsafe{ShowWindow(*hwnd,if visible{SW_SHOW}else{SW_HIDE});}}}
