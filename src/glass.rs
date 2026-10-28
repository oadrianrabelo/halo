//! Desfoque nativo atrás da janela (vibrancy no macOS, blur no Windows).
//! No Linux quem decide é o compositor, então aqui não faz nada.

use raw_window_handle::HasWindowHandle;

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
