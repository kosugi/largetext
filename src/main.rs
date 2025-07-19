use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::sync::OnceLock;
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::*, Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

extern "system" {
    pub fn DrawTextW(
        hdc: HDC,
        lpchText: *const u16,
        cchText: i32,
        lprc: *mut RECT,
        format: u32,
    ) -> i32;
}

fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

static TEXT_TO_DISPLAY: OnceLock<Box<[u16]>> = OnceLock::new();

fn to_wide(s: &str) -> Box<[u16]> {
    let wide: Vec<u16> = OsStr::new(s).encode_wide().chain(once(0)).collect();
    wide.into_boxed_slice()
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let text = if args.len() == 0 {
        String::from("Hello")
    } else {
        args.join(" ")
    };

    TEXT_TO_DISPLAY.set(to_wide(&text)).unwrap();

    unsafe {
        let hinstance = HINSTANCE::from(GetModuleHandleW(None)?);
        let class_name = w!("BigTextWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            hbrBackground: HBRUSH(COLOR_WINDOW.0 as isize),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
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

            let mut best_size = 10;
            for size in (10..300).rev() {
                let hfont = create_font(size);
                let old_font = SelectObject(hdc, HGDIOBJ(hfont.0));

                let mut calc = usable;
                let flags = DT_SINGLELINE.0 | DT_CALCRECT.0;
                if let Some(text) = TEXT_TO_DISPLAY.get() {
                    DrawTextW(hdc, text.as_ptr(), -1, &mut calc, flags);
                }

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
            FillRect(hdc, &rect, HBRUSH(so.0));

            let mut draw_rect = usable;
            let flags = DT_CENTER.0 | DT_VCENTER.0 | DT_SINGLELINE.0;
            if let Some(text) = TEXT_TO_DISPLAY.get() {
                DrawTextW(hdc, text.as_ptr(), -1, &mut draw_rect, flags);
            }

            SelectObject(hdc, old_font);
            DeleteObject(hfont);

            EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_CHAR | WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
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
