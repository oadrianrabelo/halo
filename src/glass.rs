//! Desfoque nativo atrás da janela (vibrancy no macOS, blur no Windows).
//! No Linux quem decide é o compositor, então aqui não faz nada.

use raw_window_handle::HasWindowHandle;

/// Tira a moldura e a sombra que o Windows desenha na janela.
///
/// O egui pede ao Windows a "sombra de janela sem moldura". O Windows estende a moldura 1 px
/// para dentro da janela e desenha uma sombra embaixo dela inteira. Numa janela opaca isso não
/// aparece, mas os cantos do Halo são transparentes: dava para ver um retângulo escurecido e uma
/// linha no topo em volta do formato arredondado. Aqui a moldura volta a zero, o desenho da parte
/// "não cliente" (sombra) é desligado, a borda colorida do Windows 11 some e o sistema deixa de
/// arredondar a janela por conta própria.
///
/// Por último, a janela deixa de ter o estilo de "janela com barra de título": mesmo sem barra,
/// o Windows reservava 1 px no topo para ela, que aparecia como uma linha cinza.
#[allow(unused_variables)]
pub fn remove_system_border(window: &impl HasWindowHandle) {
    #[cfg(windows)]
    {
        use raw_window_handle::RawWindowHandle;
        use windows_sys::Win32::Graphics::Dwm::{
            DWMNCRP_DISABLED, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_NCRENDERING_POLICY,
            DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
        };
        use windows_sys::Win32::UI::Controls::MARGINS;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GWL_STYLE, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
            SetWindowLongPtrW, SetWindowPos, WS_CAPTION, WS_POPUP, WS_THICKFRAME,
        };

        let Ok(handle) = window.window_handle() else { return };
        let RawWindowHandle::Win32(h) = handle.as_raw() else { return };
        let hwnd = h.hwnd.get() as windows_sys::Win32::Foundation::HWND;
        let margins = MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 0, cyBottomHeight: 0 };
        let none = DWMWA_COLOR_NONE;
        let corners = DWMWCP_DONOTROUND;
        let no_shadow = DWMNCRP_DISABLED;
        // As duas últimas só existem no Windows 11; no 10 o Windows ignora e segue normal.
        let results = unsafe {
            [
                DwmExtendFrameIntoClientArea(hwnd, &margins),
                DwmSetWindowAttribute(hwnd, DWMWA_NCRENDERING_POLICY as u32, (&raw const no_shadow).cast(), 4),
                DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR as u32, (&raw const none).cast(), 4),
                DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE as u32, (&raw const corners).cast(), 4),
            ]
        };
        // Só mexe no estilo se ainda tiver barra/borda (o winit pode recolocar ao mostrar a janela).
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
        let popup = (style & !(WS_CAPTION | WS_THICKFRAME)) | WS_POPUP;
        if popup != style {
            unsafe {
                SetWindowLongPtrW(hwnd, GWL_STYLE, popup as isize);
                let flags = SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE;
                SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0, flags);
            }
        }
        log::info!("Moldura e sombra do Windows removidas (resultados: {results:x?}, estilo {style:#x} → {popup:#x})");
    }
}

pub fn supported() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

#[allow(unused_variables)]
pub fn set(window: &impl HasWindowHandle, on: bool, radius: f32, dark: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy, clear_vibrancy};
        let _ = clear_vibrancy(window);
        if on {
            let material = if dark { NSVisualEffectMaterial::HudWindow } else { NSVisualEffectMaterial::Popover };
            apply_vibrancy(window, material, Some(NSVisualEffectState::Active), Some(radius as f64))
                .map_err(|e| e.to_string())?;
        }
    }

    #[cfg(windows)]
    {
        use window_vibrancy::{apply_blur, clear_blur};
        if on {
            apply_blur(window, Some((0, 0, 0, 0))).map_err(|e| e.to_string())?;
        } else {
            let _ = clear_blur(window);
        }
    }

    Ok(())
}
