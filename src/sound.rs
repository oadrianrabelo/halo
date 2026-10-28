//! Aviso sonoro usando o que o sistema já tem, sem incluir biblioteca de áudio.

pub fn notify() {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::MessageBeep(
            windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONASTERISK,
        );
    }

    #[cfg(target_os = "macos")]
    spawn("afplay", &["/System/Library/Sounds/Glass.aiff"]);

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Tenta os players mais comuns; se nenhum existir, fica só o aviso visual.
        const SOUND: &str = "/usr/share/sounds/freedesktop/stereo/complete.oga";
        if !spawn("canberra-gtk-play", &["-i", "complete"])
            && !spawn("paplay", &[SOUND])
            && !spawn("pw-play", &[SOUND])
        {
            spawn("aplay", &["-q", "/usr/share/sounds/alsa/Front_Center.wav"]);
        }
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
