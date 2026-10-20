//! Okur among the other processes: shut down last, the system's screen-reader flag, the end
//! of the session, and the keys held as it starts
//! (`docs/references/startup-and-secure-screens-proposal.md`, §5.5 and §5.6).

/// Asks Windows to end Okur last at shutdown and sign-out, and to leave it out of the "apps
/// are blocking shutdown" screen: "If it were, the user would have no way of reading the
/// dialog" (NVDA's design notes, which make the same call).
pub fn shut_down_last() {
    #[cfg(windows)]
    {
        /// `SHUTDOWN_NORETRY`: no retry dialog for this process.
        const SHUTDOWN_NORETRY: u32 = 0x1;
        // SAFETY: a process-wide setting with constant arguments; 0x100 is the lowest level an
        // application may take, so it is shut down after the others.
        if let Err(e) = unsafe {
            windows::Win32::System::Threading::SetProcessShutdownParameters(0x100, SHUTDOWN_NORETRY)
        } {
            tracing::warn!("shutdown order not set: {e}");
        }
    }
}

/// Whether the system's screen-reader flag (`SPI_GETSCREENREADER`) is on; `None` off Windows.
pub fn screen_reader_flag() -> Option<bool> {
    #[cfg(windows)]
    {
        windows_impl::flag()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// The system's screen-reader flag, on while this is alive. Applications read it to adapt
/// to a screen reader. JAWS sets it too, and may run beside Okur: a flag that was on before
/// Okur raised it stays on after.
pub struct ScreenReaderFlag {
    was_on: bool,
}

impl ScreenReaderFlag {
    /// Turns the flag on, for this sign-in only (it is not written to the user's profile, so a
    /// copy that is killed leaves it on until the next sign-in at most).
    pub fn raise() -> Self {
        let was_on = screen_reader_flag().unwrap_or(false);
        #[cfg(windows)]
        windows_impl::set_flag(true);
        Self { was_on }
    }
}

impl Drop for ScreenReaderFlag {
    fn drop(&mut self) {
        if !self.was_on {
            #[cfg(windows)]
            windows_impl::set_flag(false);
        }
    }
}

/// What to do when Windows ends the session (shutdown, restart, sign-out): it blocks until
/// Okur has stopped, briefly; Windows may end the process once it returns.
pub type OnSessionEnd = Box<dyn Fn() + Send>;

/// A hidden top-level window that hears Windows end the session (`WM_QUERYENDSESSION`,
/// `WM_ENDSESSION`): message-only windows "do not receive broadcast messages".
pub struct EndSessionWatcher {
    _thread: Option<std::thread::JoinHandle<()>>,
}

impl EndSessionWatcher {
    /// Starts the window on a thread of its own; `None` when it could not be made.
    pub fn start(on_end: OnSessionEnd) -> Option<Self> {
        #[cfg(windows)]
        {
            windows_impl::watch(on_end).map(|thread| Self {
                _thread: Some(thread),
            })
        }
        #[cfg(not(windows))]
        {
            let _ = on_end;
            Some(Self { _thread: None })
        }
    }
}

/// The two Shift keys' state, as Okur starts.
pub trait KeyState {
    fn left_shift_down(&self) -> bool;
    fn right_shift_down(&self) -> bool;
}

/// Both Shift keys held: JAWS's documented way to keep it from starting, "press and hold
/// both the LEFT SHIFT and RIGHT SHIFT keys as Windows boots, or after you have logged in"
/// (JAWS 2025 help, Starting JAWS), and Okur's way out if a build ever made the sign-in
/// screen unusable.
pub fn both_shifts_held(keys: &dyn KeyState) -> bool {
    keys.left_shift_down() && keys.right_shift_down()
}

/// The keyboard's state now.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemKeyState;

impl KeyState for SystemKeyState {
    fn left_shift_down(&self) -> bool {
        #[cfg(windows)]
        {
            windows_impl::key_down(0xA0) // VK_LSHIFT
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    fn right_shift_down(&self) -> bool {
        #[cfg(windows)]
        {
            windows_impl::key_down(0xA1) // VK_RSHIFT
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

/// Keys held as a test says.
#[derive(Debug, Clone, Copy, Default)]
pub struct MockKeyState {
    pub left_shift: bool,
    pub right_shift: bool,
}

impl KeyState for MockKeyState {
    fn left_shift_down(&self) -> bool {
        self.left_shift
    }

    fn right_shift_down(&self) -> bool {
        self.right_shift
    }
}

/// Lets the process `pid` take the foreground with its next window: a UI Access Okur may set
/// the foreground, and passes that on to the lowered `okur-ui` as it shows a dialog. Fails
/// harmlessly when Okur may not.
pub fn allow_foreground(pid: u32) {
    #[cfg(windows)]
    {
        // SAFETY: a plain call with a process id.
        if let Err(e) =
            unsafe { windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(pid) }
        {
            tracing::debug!(pid, "foreground not passed on: {e}");
        }
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
    }
}

/// Ends a process: an Okur that did not quit when asked.
pub fn terminate(pid: u32) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
        // SAFETY: opens the process to end it; the handle is closed by `Owned`.
        let process = unsafe { OpenProcess(PROCESS_TERMINATE, false, pid) }
            .map(crate::elevation::windows_impl::Owned)
            .map_err(|e| format!("process {pid}: {e}"))?;
        // SAFETY: the process opened above.
        unsafe { TerminateProcess(process.0, 1) }.map_err(|e| format!("process {pid}: {e}"))
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        Err("processes are ended only on Windows".to_owned())
    }
}

/// A process's executable, by process id.
pub fn image_path(pid: u32) -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    {
        use windows::core::PWSTR;
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        // SAFETY: opens the process for a query; the handle is closed by `Owned`.
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
            .map(crate::elevation::windows_impl::Owned)
            .ok()?;
        let mut path = [0u16; 1024];
        let mut len = u32::try_from(path.len()).ok()?;
        // SAFETY: a buffer of `len` characters.
        unsafe {
            QueryFullProcessImageNameW(
                process.0,
                PROCESS_NAME_WIN32,
                PWSTR(path.as_mut_ptr()),
                &mut len,
            )
        }
        .ok()?;
        let path = String::from_utf16_lossy(path.get(..usize::try_from(len).ok()?)?);
        Some(std::path::PathBuf::from(path))
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        None
    }
}

/// Starts a program through the shell (`ShellExecuteExW`), as Windows requires of a UI Access
/// program: `CreateProcess` of one fails with `ERROR_ELEVATION_REQUIRED`. `verb` "runas"
/// asks for elevation (the installer's worker), `None` opens it as the caller. With `wait`,
/// waits for it to end and returns its exit code.
pub fn shell_execute(
    program: &std::path::Path,
    parameters: &str,
    verb: Option<&str>,
    wait: bool,
) -> Result<Option<u32>, String> {
    #[cfg(windows)]
    {
        windows_impl::shell_execute(program, parameters, verb, wait)
    }
    #[cfg(not(windows))]
    {
        let _ = (program, parameters, verb, wait);
        Err("programs are started through the shell only on Windows".to_owned())
    }
}

/// The running processes whose executable is one of `names` (file names, any case), with
/// their executables' paths where they can be read.
pub fn processes_named(names: &[&str]) -> Vec<(u32, Option<std::path::PathBuf>)> {
    #[cfg(windows)]
    {
        windows_impl::processes_named(names)
    }
    #[cfg(not(windows))]
    {
        let _ = names;
        Vec::new()
    }
}

/// Asks Windows to delete a file, or an empty folder, when it next starts: a running program
/// cannot be deleted (it can be renamed first).
pub fn delete_at_reboot(path: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT};
        let wide = HSTRING::from(path.as_os_str());
        // SAFETY: a valid path; no new name means delete.
        unsafe {
            MoveFileExW(
                PCWSTR(wide.as_ptr()),
                PCWSTR::null(),
                MOVEFILE_DELAY_UNTIL_REBOOT,
            )
        }
        .map_err(|e| format!("{}: {e}", path.display()))
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("deleting at the next start is Windows'".to_owned())
    }
}

/// Shows a message in a standard message box and waits for OK: the installer's word where
/// no console started it. Screen readers read it as any message box.
pub fn message_box(title: &str, text: &str) {
    #[cfg(windows)]
    {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::UI::WindowsAndMessaging::{
            MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND,
        };
        let (title, text) = (HSTRING::from(title), HSTRING::from(text));
        // SAFETY: the strings outlive the call.
        let _ = unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                PCWSTR(title.as_ptr()),
                MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND,
            )
        };
    }
    #[cfg(not(windows))]
    {
        println!("{title}: {text}");
    }
}

/// Attaches a program without a console of its own (`okur-sr.exe`) to the console of the
/// program that started it, so that what it prints is seen there; false when there is none.
pub fn attach_parent_console() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        // SAFETY: a plain call; it fails harmlessly when there is no parent console.
        unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_ok()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::OnSessionEnd;
    use std::sync::Mutex;
    use windows::core::{w, BOOL};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
        SystemParametersInfoW, TranslateMessage, MSG, SPIF_SENDCHANGE, SPI_GETSCREENREADER,
        SPI_SETSCREENREADER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOW_EX_STYLE, WINDOW_STYLE,
        WM_ENDSESSION, WM_QUERYENDSESSION, WNDCLASSW,
    };

    pub(super) fn flag() -> Option<bool> {
        let mut on = BOOL(0);
        // SAFETY: SPI_GETSCREENREADER writes one BOOL.
        unsafe {
            SystemParametersInfoW(
                SPI_GETSCREENREADER,
                0,
                Some(std::ptr::from_mut(&mut on).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        }
        .ok()?;
        Some(on.as_bool())
    }

    pub(super) fn set_flag(on: bool) {
        // SAFETY: SPI_SETSCREENREADER takes the flag in uiParam; applications are told.
        if let Err(e) = unsafe {
            SystemParametersInfoW(SPI_SETSCREENREADER, u32::from(on), None, SPIF_SENDCHANGE)
        } {
            tracing::warn!(on, "screen reader flag not set: {e}");
        }
    }

    pub(super) fn shell_execute(
        program: &std::path::Path,
        parameters: &str,
        verb: Option<&str>,
        wait: bool,
    ) -> Result<Option<u32>, String> {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::System::Threading::{
            GetExitCodeProcess, WaitForSingleObject, INFINITE,
        };
        use windows::Win32::UI::Shell::{
            ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        };
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let file = HSTRING::from(program.as_os_str());
        let parameters = HSTRING::from(parameters);
        let verb = verb.map(HSTRING::from);
        let mut info = SHELLEXECUTEINFOW {
            cbSize: u32::try_from(std::mem::size_of::<SHELLEXECUTEINFOW>()).unwrap_or(0),
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            lpVerb: verb.as_ref().map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr())),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(parameters.as_ptr()),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        // SAFETY: the strings outlive the call; the process handle is closed below.
        unsafe { ShellExecuteExW(&mut info) }.map_err(|e| format!("{}: {e}", program.display()))?;
        if info.hProcess.is_invalid() {
            return Ok(None);
        }
        let process = crate::elevation::windows_impl::Owned(info.hProcess);
        if !wait {
            return Ok(None);
        }
        let mut code = 0u32;
        // SAFETY: the started process's handle.
        unsafe {
            let _ = WaitForSingleObject(process.0, INFINITE);
            GetExitCodeProcess(process.0, &mut code)
        }
        .map_err(|e| format!("{}: {e}", program.display()))?;
        Ok(Some(code))
    }

    pub(super) fn processes_named(names: &[&str]) -> Vec<(u32, Option<std::path::PathBuf>)> {
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        };
        // SAFETY: a snapshot of the process list, closed by `Owned`.
        let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
            return Vec::new();
        };
        let snapshot = crate::elevation::windows_impl::Owned(snapshot);
        let mut entry = PROCESSENTRY32W {
            dwSize: u32::try_from(std::mem::size_of::<PROCESSENTRY32W>()).unwrap_or(0),
            ..Default::default()
        };
        let mut found = Vec::new();
        // SAFETY: the snapshot and an entry of the size it says.
        let mut more = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(entry.szExeFile.get(..len).unwrap_or_default());
            if names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
                found.push((entry.th32ProcessID, super::image_path(entry.th32ProcessID)));
            }
            // SAFETY: as above.
            more = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
        }
        found
    }

    pub(super) fn key_down(vk: i32) -> bool {
        // SAFETY: a plain query.
        (unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000) != 0
    }

    /// What the window calls when the session ends: one per process.
    static ON_END: Mutex<Option<OnSessionEnd>> = Mutex::new(None);

    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            // Okur never holds the session open.
            WM_QUERYENDSESSION => LRESULT(1),
            WM_ENDSESSION => {
                if wparam.0 != 0 {
                    tracing::info!("the session is ending: Okur stops");
                    if let Ok(on_end) = ON_END.lock() {
                        if let Some(on_end) = on_end.as_ref() {
                            on_end();
                        }
                    }
                }
                LRESULT(0)
            }
            // SAFETY: the default handling for every other message.
            _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
        }
    }

    pub(super) fn watch(on_end: OnSessionEnd) -> Option<std::thread::JoinHandle<()>> {
        if let Ok(mut slot) = ON_END.lock() {
            *slot = Some(on_end);
        }
        let (made_tx, made_rx) = std::sync::mpsc::channel::<bool>();
        let thread = std::thread::Builder::new()
            .name("okur-session-end".to_owned())
            .spawn(move || {
                // SAFETY: a window class and a hidden top-level window of this thread, whose
                // messages this thread's loop dispatches.
                unsafe {
                    let instance = GetModuleHandleW(None).ok().map(Into::into);
                    let class = WNDCLASSW {
                        lpfnWndProc: Some(window_proc),
                        hInstance: instance.unwrap_or_default(),
                        lpszClassName: w!("OkurSessionEnd"),
                        ..Default::default()
                    };
                    RegisterClassW(&class);
                    let window = CreateWindowExW(
                        WINDOW_EX_STYLE(0),
                        w!("OkurSessionEnd"),
                        w!("Okur"),
                        WINDOW_STYLE(0),
                        0,
                        0,
                        0,
                        0,
                        None,
                        None,
                        instance,
                        None,
                    );
                    let _ = made_tx.send(window.is_ok());
                    if window.is_err() {
                        return;
                    }
                    let mut message = MSG::default();
                    while GetMessageW(&mut message, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
            })
            .ok()?;
        made_rx.recv().unwrap_or(false).then_some(thread)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_both_shifts_held_together_stop_a_start() {
        let held = |left_shift, right_shift| {
            both_shifts_held(&MockKeyState {
                left_shift,
                right_shift,
            })
        };
        assert!(held(true, true));
        assert!(!held(true, false));
        assert!(!held(false, true));
        assert!(!held(false, false));
    }
}
