//! Aviso sonoro usando o que o sistema já tem, sem incluir biblioteca de áudio.
//! `file` vazio usa o som padrão do sistema.

pub fn notify(file: &str) {
    let file = file.trim();

    #[cfg(windows)]
    {
        use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};
        use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
        use windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONASTERISK;
        let played = !file.is_empty() && {
            let wide: Vec<u16> = file.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe { PlaySoundW(wide.as_ptr(), std::ptr::null_mut(), SND_FILENAME | SND_ASYNC | SND_NODEFAULT) != 0 }
        };
        if !played {
            unsafe { MessageBeep(MB_ICONASTERISK) };
        }
    }

    #[cfg(target_os = "macos")]
    {
        let file = if file.is_empty() { "/System/Library/Sounds/Glass.aiff" } else { file };
        spawn("afplay", &[file]);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        const DEFAULT: &str = "/usr/share/sounds/freedesktop/stereo/complete.oga";
        let file = if file.is_empty() { DEFAULT } else { file };
        // Tenta os players mais comuns; se nenhum existir, fica só o aviso visual.
        let _ = spawn("paplay", &[file]) || spawn("pw-play", &[file]) || spawn("aplay", &["-q", file]);
    }
}

#[cfg(unix)]
fn spawn(cmd: &str, args: &[&str]) -> bool {
    use std::process::{Command, Stdio};
    Command::new(cmd)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}
