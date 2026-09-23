//! Native DWM rounding must own the whole surface, including the backdrop.
//! A custom HRGN clips the client but leaves Acrylic's rectangular background.
use std::cell::Cell;
use tauri::WebviewWindow;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::{Dwm::*, Gdi::*},
    UI::{
        Shell::{DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};
const SUBCLASS_ID: usize = 0x50444652;
struct Shape {
    updating: Cell<bool>,
}
fn clean_style(style: u32) -> u32 {
    style & !(WS_CAPTION | WS_THICKFRAME)
}
fn clean_ex_style(style: u32) -> u32 {
    style & !(WS_EX_WINDOWEDGE | WS_EX_CLIENTEDGE | WS_EX_DLGMODALFRAME | WS_EX_STATICEDGE)
}
unsafe fn rounded_surface(hwnd: HWND, state: &Shape) -> Result<(), String> {
    if state.updating.replace(true) {
        return Ok(());
    }
    let result = (|| {
        // Window regions disable compositor rounding. Remove any old HRGN.
        let current = CreateRectRgn(0, 0, 0, 0);
        if !current.is_null() {
            if GetWindowRgn(hwnd, current) != 0 {
                SetWindowRgn(hwnd, std::ptr::null_mut(), 0);
            }
            DeleteObject(current);
        }
        let preference = DWMWCP_ROUND;
        let hr = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as _,
            &preference as *const _ as _,
            4,
        );
        if hr < 0 {
            return Err(
                "此 Windows 不支援整層玻璃圓角（需要 Windows 11）；尚未通過外觀驗收".into(),
            );
        }
        let border = 0xfffffffeu32; // DWMWA_COLOR_NONE
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR as _, &border as *const _ as _, 4);
        Ok(())
    })();
    state.updating.set(false);
    result
}
unsafe extern "system" fn frame_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    if msg == WM_NCPAINT {
        return 0;
    }
    if msg == WM_NCACTIVATE {
        return DefSubclassProc(hwnd, msg, wparam, -1);
    }
    if msg == WM_STYLECHANGING && lparam != 0 {
        let change = &mut *(lparam as *mut STYLESTRUCT);
        if wparam as i32 == GWL_STYLE {
            change.styleNew = clean_style(change.styleNew);
        }
        if wparam as i32 == GWL_EXSTYLE {
            change.styleNew = clean_ex_style(change.styleNew);
        }
    }
    if msg == WM_NCDESTROY {
        RemoveWindowSubclass(hwnd, Some(frame_proc), SUBCLASS_ID);
        drop(Box::from_raw(data as *mut Shape));
        return DefSubclassProc(hwnd, msg, wparam, lparam);
    }
    let result = DefSubclassProc(hwnd, msg, wparam, lparam);
    if matches!(
        msg,
        WM_SIZE | WM_WINDOWPOSCHANGED | WM_DPICHANGED | WM_STYLECHANGED | WM_DWMCOMPOSITIONCHANGED
    ) {
        let _ = rounded_surface(hwnd, &*(data as *const Shape));
    }
    result
}
pub fn install(window: &WebviewWindow) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as HWND;
    unsafe {
        let ptr = Box::into_raw(Box::new(Shape {
            updating: Cell::new(false),
        }));
        if SetWindowSubclass(hwnd, Some(frame_proc), SUBCLASS_ID, ptr as usize) == 0 {
            drop(Box::from_raw(ptr));
            return Err("無法初始化無框視窗".into());
        }
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            clean_style(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32) as isize,
        );
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            clean_ex_style(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32) as isize,
        );
        if SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        ) == 0
        {
            return Err("無法更新無框視窗".into());
        }
    }
    // Older Windows may still open the app; the material command shows the exact
    // unsupported result in Settings instead of crashing or claiming acceptance.
    let _ = round(window, None);
    Ok(())
}
pub fn round(window: &WebviewWindow, _viewport_width: Option<f64>) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as HWND;
    unsafe {
        let mut data = 0;
        if GetWindowSubclass(hwnd, Some(frame_proc), SUBCLASS_ID, &mut data) == 0 {
            return Err("原生圓角尚未初始化".into());
        }
        rounded_surface(hwnd, &*(data as *const Shape))
    }
}
pub fn material(window: &WebviewWindow, enabled: bool) -> Result<(), String> {
    round(window, None)?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as HWND;
    let backdrop = if enabled {
        DWMSBT_TRANSIENTWINDOW
    } else {
        DWMSBT_NONE
    };
    let hr = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE as _,
            &backdrop as *const _ as _,
            4,
        )
    };
    if hr < 0 {
        return Err(
            "此 Windows 不支援此版原生圓角 Acrylic（需要 Windows 11 22H2 以上）；未套用灰底替代"
                .into(),
        );
    }
    round(window, None)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_filter_preserves_controls() {
        let output =
            clean_style(WS_VISIBLE | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX);
        assert_eq!(output & (WS_CAPTION | WS_THICKFRAME), 0);
        assert_ne!(output & WS_SYSMENU, 0);
    }
    #[test]
    fn compositor_rounding_removes_incompatible_region() {
        unsafe {
            let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                class.as_ptr(),
                WS_POPUP,
                0,
                0,
                440,
                740,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!hwnd.is_null());
            let state = Shape {
                updating: Cell::new(false),
            };
            for (w, h) in [(440, 740), (660, 1110)] {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    0,
                    0,
                    w,
                    h,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                SetWindowRgn(hwnd, CreateRoundRectRgn(0, 0, w, h, 84, 84), 0);
                rounded_surface(hwnd, &state).unwrap();
                let region = CreateRectRgn(0, 0, 0, 0);
                assert_eq!(GetWindowRgn(hwnd, region), 0);
                DeleteObject(region);
                let mut preference = 0i32;
                assert_eq!(
                    DwmGetWindowAttribute(
                        hwnd,
                        DWMWA_WINDOW_CORNER_PREFERENCE as _,
                        &mut preference as *mut _ as _,
                        4
                    ),
                    0
                );
                assert_eq!(preference, DWMWCP_ROUND);
            }
            DestroyWindow(hwnd);
        }
    }
}
