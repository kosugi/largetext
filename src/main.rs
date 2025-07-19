use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::*, Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

fn to_wide(s: &str) -> &'static mut [u16] {
    let vec: Vec<u16> = OsStr::new(s).encode_wide().chain(once(0)).collect();
    Box::leak(vec.into_boxed_slice())
}

static mut TEXT_TO_DISPLAY: Option<&'static mut [u16]> = None;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let text = if args.len() >= 2 { &args[1] } else { "Hello" };

    unsafe {
        TEXT_TO_DISPLAY = Some(to_wide(text));
        let hinstance = HINSTANCE::from(GetModuleHandleW(None)?);
        let class_name = w!("BigTextWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            hbrBackground: HBRUSH(COLOR_WINDOW.0 as isize),
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            class_name,
            w!(""),
            WS_POPUP | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            None,
            None,
            hinstance,
            None,
        );
        ShowWindow(hwnd, SW_SHOWMAXIMIZED);
        UpdateWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(0), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

unsafe fn create_font(size: i32) -> HFONT {
    CreateFontW(
        -size,
        0,
        0,
        0,
        FW_BOLD.0 as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET.0 as u32,
        OUT_DEFAULT_PRECIS.0.into(),
        CLIP_DEFAULT_PRECIS.0.into(),
        DEFAULT_QUALITY.0.into(),
        (DEFAULT_PITCH.0 as u32) | (FF_DONTCARE.0 as u32),
        w!("游明朝"),
    )
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);

            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            let margin_x = width / 20;
            let margin_y = height / 20;

            let usable = RECT {
                left: rect.left + margin_x,
                top: rect.top + margin_y,
                right: rect.right - margin_x,
                bottom: rect.bottom - margin_y,
            };

            let text = TEXT_TO_DISPLAY.as_deref_mut().unwrap();

            let mut best_size = 10;
            for size in (10..300).rev() {
                let hfont = create_font(size);
                let old_font = SelectObject(hdc, HGDIOBJ(hfont.0));

                let mut calc = usable;
                let flags = DT_SINGLELINE | DT_CALCRECT;
                DrawTextW(hdc, text, &mut calc, flags);

                let text_w = calc.right - calc.left;
                let text_h = calc.bottom - calc.top;
                let max_w = usable.right - usable.left;
                let max_h = usable.bottom - usable.top;

                SelectObject(hdc, old_font);
                DeleteObject(hfont);

                if text_w <= max_w && text_h <= max_h {
                    best_size = size;
                    break;
                }
            }

            // 最終確定フォントで描画
            let hfont = create_font(best_size);
            let old_font = SelectObject(hdc, HGDIOBJ(hfont.0));

            SetTextColor(hdc, rgb(255, 255, 255));
            SetBkColor(hdc, rgb(0, 0, 0));
            let so = GetStockObject(BLACK_BRUSH);
            if so.0 == 0 {
                panic!("GetStockObject failed");
            }
            FillRect(hdc, &usable, HBRUSH(so.0));

            let mut draw_rect = usable;
            let flags = DT_CENTER | DT_VCENTER | DT_SINGLELINE;
            DrawTextW(hdc, text, &mut draw_rect, flags);

            SelectObject(hdc, old_font);
            DeleteObject(hfont);

            EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_KEYDOWN | WM_CHAR => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
