//! Janela realmente sem moldura no Windows.
//!
//! O egui pede ao winit a "sombra de janela sem moldura". Para isso o winit mantém os estilos de
//! janela com barra de título e, ao calcular a área da janela, a empurra 1 px para baixo — o
//! próprio código dele avisa que isso deixa "uma pequena borda preta de 1 px no topo". O Windows
//! também desenha uma sombra embaixo da janela inteira, que aparece pelos cantos transparentes.
//!
//! Aqui a janela passa a responder ela mesma algumas mensagens do Windows, antes do winit:
//! - `WM_NCCALCSIZE`: a janela inteira é área de conteúdo, sem o 1 px;
//! - `WM_NCPAINT`: nunca pinta moldura;
//! - `WM_NCACTIVATE`, `WM_SETTEXT`, `WM_SETICON`: não redesenham a barra de título clássica
//!   (que aparecia por um instante ao arrastar, com cara de Windows 7).
//!
//! E pede ao DWM: sem sombra/moldura, sem borda colorida (Windows 11) e sem arredondar.

use raw_window_handle::HasWindowHandle;

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::Graphics::Dwm::{
        DWMNCRP_DISABLED, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_NCRENDERING_POLICY, DWMWA_WINDOW_CORNER_PREFERENCE,
        DWMWCP_DONOTROUND, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
    };
    use windows_sys::Win32::UI::Controls::MARGINS;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, GWL_STYLE, GWLP_WNDPROC, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCPAINT, WM_SETICON,
        WM_SETTEXT, WNDPROC, WS_VISIBLE,
    };

    /// Procedimento de janela original (o do winit), para repassar todo o resto.
    static ORIGINAL: AtomicIsize = AtomicIsize::new(0);

    unsafe fn call_original(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let original: WNDPROC = unsafe { std::mem::transmute(ORIGINAL.load(Ordering::Relaxed)) };
        unsafe { CallWindowProcW(original, hwnd, msg, wparam, lparam) }
    }

    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            // wparam != 0: o Windows quer saber a área de conteúdo. Retornar 0 sem mexer = a janela toda.
            WM_NCCALCSIZE if wparam != 0 => 0,
            WM_NCPAINT => 0,
            // lparam = -1 diz ao Windows para não repintar a moldura ao ganhar/perder o foco.
            WM_NCACTIVATE => unsafe { call_original(hwnd, msg, wparam, -1) },
            // Com o desenho do DWM desligado, trocar título/ícone pintaria a barra clássica:
            // esconde o bit de "visível" só durante a troca.
            WM_SETTEXT | WM_SETICON => unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                SetWindowLongPtrW(hwnd, GWL_STYLE, style & !(WS_VISIBLE as isize));
                let r = call_original(hwnd, msg, wparam, lparam);
                SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                r
            },
            _ => unsafe { call_original(hwnd, msg, wparam, lparam) },
        }
    }

    pub fn apply(hwnd: HWND) {
        unsafe {
            let current = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
            if current != proc as *const () as isize {
                ORIGINAL.store(current, Ordering::Relaxed);
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
            }
            let margins = MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 0, cyBottomHeight: 0 };
            let (no_shadow, none, corners) = (DWMNCRP_DISABLED, DWMWA_COLOR_NONE, DWMWCP_DONOTROUND);
            // As duas últimas só existem no Windows 11; no 10 o Windows ignora e segue normal.
            let results = [
                DwmExtendFrameIntoClientArea(hwnd, &margins),
                DwmSetWindowAttribute(hwnd, DWMWA_NCRENDERING_POLICY as u32, (&raw const no_shadow).cast(), 4),
                DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR as u32, (&raw const none).cast(), 4),
                DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE as u32, (&raw const corners).cast(), 4),
            ];
            // Faz o Windows recalcular a área da janela com o WM_NCCALCSIZE novo.
            let flags = SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE;
            SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0, flags);
            log::info!("Moldura do Windows removida (DWM: {results:x?})");
        }
    }
}

/// Chamar uma vez depois que a janela existe (e de novo se o foco mudar; é barato).
#[allow(unused_variables)]
pub fn apply(window: &impl HasWindowHandle) {
    #[cfg(windows)]
    {
        use raw_window_handle::RawWindowHandle;
        if let Ok(handle) = window.window_handle()
            && let RawWindowHandle::Win32(h) = handle.as_raw()
        {
            imp::apply(h.hwnd.get() as windows_sys::Win32::Foundation::HWND);
        }
    }
}
