//! Starting a helper below Okur's own rights (Windows).
//!
//! The installed screen reader runs with UI Access, at High integrity for an administrator.
//! A child it starts the plain way inherits both. `okur-ui`, which draws Okur's windows, is
//! meant to stay a normal process (`docs/references/helper-ui-architecture.md`), so that the
//! harness and other tools at Medium integrity can read Okur's dialogs. It is therefore
//! started with a copy of Okur's token with UI Access cleared and the integrity lowered to
//! Medium, which a process may always do to its own token
//! (`docs/references/startup-and-secure-screens-proposal.md`, §5.2).

use std::fs::File;
use std::os::windows::io::FromRawHandle;
use std::path::Path;

use windows::core::{HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, LocalFree, SetHandleInformation, HANDLE, HANDLE_FLAGS, HANDLE_FLAG_INHERIT,
    HLOCAL, WAIT_OBJECT_0,
};
use windows::Win32::Security::Authorization::ConvertStringSidToSidW;
use windows::Win32::Security::{
    DuplicateTokenEx, GetLengthSid, SecurityImpersonation, SetTokenInformation,
    TokenIntegrityLevel, TokenPrimary, TokenUIAccess, PSID, SECURITY_ATTRIBUTES,
    SID_AND_ATTRIBUTES, TOKEN_ACCESS_MASK, TOKEN_ADJUST_DEFAULT, TOKEN_ASSIGN_PRIMARY,
    TOKEN_DUPLICATE, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
};
use windows::Win32::System::Pipes::CreatePipe;
use windows::Win32::System::Threading::{
    CreateProcessAsUserW, DeleteProcThreadAttributeList, GetCurrentProcess,
    InitializeProcThreadAttributeList, OpenProcessToken, UpdateProcThreadAttribute,
    WaitForSingleObject, EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, STARTF_USESTDHANDLES, STARTUPINFOEXW,
    STARTUPINFOW,
};

use crate::elevation::windows_impl::Owned;

/// Medium integrity's label.
const MEDIUM_INTEGRITY: &str = "S-1-16-8192";
/// `SE_GROUP_INTEGRITY`: the SID is an integrity label.
const SE_GROUP_INTEGRITY: u32 = 0x20;
/// `MAXIMUM_ALLOWED`: every right the caller may have on the new token.
const MAXIMUM_ALLOWED: u32 = 0x0200_0000;

/// A child started with the lowered token.
pub struct LoweredChild {
    process: Owned,
    pid: u32,
}

impl LoweredChild {
    pub fn id(&self) -> u32 {
        self.pid
    }

    /// Whether the child has ended.
    pub fn has_exited(&self) -> bool {
        // SAFETY: the child's process handle, waited on without blocking.
        unsafe { WaitForSingleObject(self.process.0, 0) == WAIT_OBJECT_0 }
    }
}

/// A child's standard input to write and its standard output to read.
pub struct ChildPipes {
    pub input: File,
    pub output: File,
}

/// A copy of this process's token, for a child: UI Access cleared and the integrity Medium.
fn lowered_token() -> Result<Owned, String> {
    let mut own = HANDLE::default();
    // SAFETY: the current process's pseudo-handle and a valid out-pointer.
    unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT,
            &mut own,
        )
    }
    .map_err(|e| format!("own token: {e}"))?;
    let own = Owned(own);
    let mut copy = HANDLE::default();
    // SAFETY: duplicates the token opened above as a primary token.
    unsafe {
        DuplicateTokenEx(
            own.0,
            TOKEN_ACCESS_MASK(MAXIMUM_ALLOWED),
            None,
            SecurityImpersonation,
            TokenPrimary,
            &mut copy,
        )
    }
    .map_err(|e| format!("token copy: {e}"))?;
    let copy = Owned(copy);
    let no_ui_access = 0u32;
    // SAFETY: TokenUIAccess takes one DWORD.
    if let Err(e) = unsafe {
        SetTokenInformation(
            copy.0,
            TokenUIAccess,
            std::ptr::from_ref(&no_ui_access).cast(),
            4,
        )
    } {
        tracing::warn!("okur-ui keeps UI Access: {e}");
    }
    let label = HSTRING::from(MEDIUM_INTEGRITY);
    let mut sid = PSID::default();
    // SAFETY: a valid SID string; the SID is freed with LocalFree below.
    unsafe { ConvertStringSidToSidW(PCWSTR(label.as_ptr()), &mut sid) }
        .map_err(|e| format!("medium integrity's SID: {e}"))?;
    let mandatory = TOKEN_MANDATORY_LABEL {
        Label: SID_AND_ATTRIBUTES {
            Sid: sid,
            Attributes: SE_GROUP_INTEGRITY,
        },
    };
    // SAFETY: the label and the SID it points to, alive for the call.
    let size = u32::try_from(std::mem::size_of::<TOKEN_MANDATORY_LABEL>()).unwrap_or(0)
        + unsafe { GetLengthSid(sid) };
    // SAFETY: as above.
    let lowered = unsafe {
        SetTokenInformation(
            copy.0,
            TokenIntegrityLevel,
            std::ptr::from_ref(&mandatory).cast(),
            size,
        )
    };
    // SAFETY: frees what ConvertStringSidToSid allocated.
    let _ = unsafe { LocalFree(Some(HLOCAL(sid.0))) };
    lowered.map_err(|e| format!("medium integrity: {e}"))?;
    Ok(copy)
}

/// An anonymous pipe whose `child_end` is inherited and whose other end is the parent's.
fn pipe(child_reads: bool) -> Result<(Owned, Owned), String> {
    let inherit = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(std::mem::size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(0),
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: true.into(),
    };
    let (mut read, mut write) = (HANDLE::default(), HANDLE::default());
    // SAFETY: valid out-pointers and attributes.
    unsafe { CreatePipe(&mut read, &mut write, Some(&inherit), 0) }
        .map_err(|e| format!("pipe: {e}"))?;
    let (read, write) = (Owned(read), Owned(write));
    let (child, parent) = if child_reads {
        (read, write)
    } else {
        (write, read)
    };
    // SAFETY: the parent's end stays with the parent.
    unsafe { SetHandleInformation(parent.0, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) }
        .map_err(|e| format!("pipe: {e}"))?;
    Ok((child, parent))
}

/// Starts `program` with the lowered token, its standard input and output piped to the
/// caller, its standard error closed. Only the two pipe ends are inherited.
pub fn spawn(program: &Path) -> Result<(LoweredChild, ChildPipes), String> {
    let token = lowered_token()?;
    let (child_input, parent_input) = pipe(true)?;
    let (child_output, parent_output) = pipe(false)?;
    let inherited = [child_input.0, child_output.0];

    let mut size = 0usize;
    // SAFETY: a size query for a list of one attribute (it fails with the size).
    let _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut size) };
    let mut list_buffer = vec![0u64; size.div_ceil(8).max(1)];
    let list = LPPROC_THREAD_ATTRIBUTE_LIST(list_buffer.as_mut_ptr().cast());
    // SAFETY: a buffer of at least `size` bytes, eight-byte aligned.
    unsafe { InitializeProcThreadAttributeList(Some(list), 1, None, &mut size) }
        .map_err(|e| format!("attribute list: {e}"))?;
    // SAFETY: the handles' array outlives the process creation below.
    let updated = unsafe {
        UpdateProcThreadAttribute(
            list,
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            Some(inherited.as_ptr().cast()),
            std::mem::size_of_val(&inherited),
            None,
            None,
        )
    };
    let created = updated
        .map_err(|e| format!("attribute list: {e}"))
        .and_then(|()| {
            let startup = STARTUPINFOEXW {
                StartupInfo: STARTUPINFOW {
                    cb: u32::try_from(std::mem::size_of::<STARTUPINFOEXW>()).unwrap_or(0),
                    dwFlags: STARTF_USESTDHANDLES,
                    hStdInput: child_input.0,
                    hStdOutput: child_output.0,
                    hStdError: HANDLE::default(),
                    ..Default::default()
                },
                lpAttributeList: list,
            };
            let application = HSTRING::from(program.as_os_str());
            let mut command_line: Vec<u16> = format!("\"{}\"", program.display())
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mut info = PROCESS_INFORMATION::default();
            // SAFETY: every pointer is alive for the call; the new process's handles are
            // closed or kept below.
            unsafe {
                CreateProcessAsUserW(
                    Some(token.0),
                    PCWSTR(application.as_ptr()),
                    Some(PWSTR(command_line.as_mut_ptr())),
                    None,
                    None,
                    true,
                    EXTENDED_STARTUPINFO_PRESENT,
                    None,
                    PCWSTR::null(),
                    std::ptr::from_ref(&startup.StartupInfo),
                    &mut info,
                )
            }
            .map_err(|e| format!("{}: {e}", program.display()))
            .map(|()| info)
        });
    // SAFETY: the list initialized above.
    unsafe { DeleteProcThreadAttributeList(list) };
    let info = created?;
    // SAFETY: the new process's thread handle, not needed.
    let _ = unsafe { CloseHandle(info.hThread) };
    drop((child_input, child_output));
    // SAFETY: the parent's pipe ends, handed to the files that own them from here.
    let pipes = unsafe {
        let input = File::from_raw_handle(parent_input.0 .0);
        let output = File::from_raw_handle(parent_output.0 .0);
        std::mem::forget(parent_input);
        std::mem::forget(parent_output);
        ChildPipes { input, output }
    };
    Ok((
        LoweredChild {
            process: Owned(info.hProcess),
            pid: info.dwProcessId,
        },
        pipes,
    ))
}
