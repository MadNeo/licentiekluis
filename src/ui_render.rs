//! Paint backgrounds in the parent window, never with overlapping child controls.
use native_windows_gui as nwg;
use winapi::shared::windef::{HDC, RECT};
use winapi::um::wingdi::{CreateSolidBrush, DeleteObject, RGB};
use winapi::um::winuser::{FillRect, GetClientRect, WM_ERASEBKGND};

pub type Panel = (i32, i32, i32, i32, [u8; 3]);

pub fn bind(window: &nwg::Window, panels: Vec<Panel>, design: (i32, i32))
    -> Result<nwg::RawEventHandler, nwg::NwgError>
{
    nwg::bind_raw_event_handler(&window.handle, 0x4C4B0001, move |hwnd, msg, w, _l| {
        if msg != WM_ERASEBKGND { return None; }
        unsafe {
            let mut client: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut client);
            let dc = w as HDC;
            // The same design coordinates scale with the native window's DPI.
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
