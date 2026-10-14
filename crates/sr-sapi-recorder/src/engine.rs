//! The COM in-process server: class factory, `ISpTTSEngine` and `ISpObjectWithToken`.

use crate::{action_name, Fragment, SpokenRecord};
use std::ffi::c_void;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use windows::core::{implement, IUnknown, Interface, Ref, BOOL, GUID, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_POINTER, S_FALSE,
};
use windows::Win32::Media::Audio::WAVEFORMATEX;
use windows::Win32::Media::Speech::{
    ISpObjectToken, ISpObjectWithToken, ISpObjectWithToken_Impl, ISpTTSEngine, ISpTTSEngineSite,
    ISpTTSEngine_Impl, SpObjectToken, SPVES_ABORT, SPVTEXTFRAG,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoTaskMemAlloc, CoTaskMemFree, IClassFactory, IClassFactory_Impl, CLSCTX_ALL,
};

/// The recorder's COM class id (registered by `install-speech-recorder.ps1`).
pub const CLSID_RECORDER: GUID = GUID::from_u128(0xc7abfa41_1b9f_4c8c_a9e6_56e2d350d16d);

/// `SPDFID_WaveFormatEx`: the output format is described by a `WAVEFORMATEX`.
const SPDFID_WAVE_FORMAT_EX: GUID = GUID::from_u128(0xc31adbae_527f_4ff5_a230_f62bb61ff70c);

/// The output format when no delegate voice is available: 16 kHz, 16-bit mono.
const FALLBACK_RATE: u32 = 16_000;

/// COM entry point: the class factory for [`CLSID_RECORDER`].
///
/// # Safety
/// Called by COM with valid pointers.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() || rclsid.is_null() {
        return E_POINTER;
    }
    // SAFETY: COM passes valid pointers; the out pointer is cleared before use.
    unsafe {
        *ppv = std::ptr::null_mut();
        if *rclsid != CLSID_RECORDER {
            return CLASS_E_CLASSNOTAVAILABLE;
        }
        let factory: IClassFactory = Factory.into();
        factory.query(riid, ppv)
    }
}

/// COM entry point. The engine stays loaded: the host may keep fragments' memory alive.
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        riid: *const GUID,
        object: *mut *mut c_void,
    ) -> windows::core::Result<()> {
        if outer.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        if object.is_null() {
            return Err(E_POINTER.into());
        }
        let engine: IUnknown = Recorder::default().into();
        // SAFETY: valid interface id and out pointer from the caller.
        unsafe { engine.query(riid, object) }.ok()
    }

    fn LockServer(&self, _lock: BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct State {
    token: Option<ISpObjectToken>,
    log: Option<PathBuf>,
    delegate: Option<ISpTTSEngine>,
}

#[implement(ISpTTSEngine, ISpObjectWithToken)]
#[derive(Default)]
struct Recorder {
    state: Mutex<State>,
}

/// A string value of the voice's registry token.
fn token_string(token: &ISpObjectToken, name: PCWSTR) -> Option<String> {
    // SAFETY: SAPI returns a CoTaskMemAlloc'd string, freed here after copying.
    unsafe {
        let value: PWSTR = token.GetStringValue(name).ok()?;
        let text = value.to_string().ok();
        CoTaskMemFree(Some(value.0 as *const c_void));
        text.filter(|t| !t.is_empty())
    }
}

/// The real voice named by token id `id` (e.g. `HKEY_LOCAL_MACHINE\...\TTS_MS_EN-US_DAVID_11.0`).
fn create_delegate(id: &str) -> windows::core::Result<ISpTTSEngine> {
    let id = windows::core::HSTRING::from(id);
    // SAFETY: SAPI token activation with valid arguments; CreateInstance also hands the new
    // engine its token.
    unsafe {
        let token: ISpObjectToken = CoCreateInstance(&SpObjectToken, None, CLSCTX_ALL)?;
        token.SetId(PCWSTR::null(), &id, false)?;
        let mut engine: *mut c_void = std::ptr::null_mut();
        token.CreateInstance(None, CLSCTX_ALL.0, &ISpTTSEngine::IID, &mut engine)?;
        Ok(ISpTTSEngine::from_raw(engine))
    }
}

/// The fragments of one `Speak` call.
fn fragments(mut frag: *const SPVTEXTFRAG) -> Vec<Fragment> {
    let mut out = Vec::new();
    // SAFETY: SAPI's fragment list is a valid linked list for the duration of `Speak`, and
    // each fragment's text pointer covers `ulTextLen` UTF-16 units.
    unsafe {
        while let Some(f) = frag.as_ref() {
            let units = if f.pTextStart.is_null() {
                &[][..]
            } else {
                std::slice::from_raw_parts(f.pTextStart.0, f.ulTextLen as usize)
            };
            out.push(Fragment {
                action: action_name(f.State.eAction.0).to_owned(),
                text: String::from_utf16_lossy(units),
                pitch: f.State.PitchAdj.MiddleAdj,
                rate: f.State.RateAdj,
                volume: f.State.Volume,
                silence_ms: f.State.SilenceMSecs,
            });
            frag = f.pNext;
        }
    }
    out
}

fn append(log: &PathBuf, record: &SpokenRecord) {
    let Ok(line) = serde_json::to_string(record) else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
    {
        let _ = writeln!(file, "{line}");
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// Speaks a short tone per character when no delegate voice exists, so silence detection
/// still hears speech.
fn tone(site: &ISpTTSEngineSite, chars: usize) -> windows::core::Result<()> {
    let samples = (FALLBACK_RATE as usize / 20) * chars.clamp(1, 200);
    let chunk: Vec<i16> = (0..FALLBACK_RATE as usize / 20)
        .map(|i| {
            let phase = i as f32 * 440.0 * std::f32::consts::TAU / FALLBACK_RATE as f32;
            (phase.sin() * 2000.0) as i16
        })
        .collect();
    let mut written = 0;
    while written < samples {
        // SAFETY: plain site calls; the buffer outlives the call.
        unsafe {
            if site.GetActions() & SPVES_ABORT.0 as u32 != 0 {
                break;
            }
            site.Write(
                chunk.as_ptr().cast(),
                u32::try_from(chunk.len() * 2).unwrap_or(0),
            )?;
        }
        written += chunk.len();
    }
    Ok(())
}

impl ISpTTSEngine_Impl for Recorder_Impl {
    fn Speak(
        &self,
        flags: u32,
        format: *const GUID,
        wave: *const WAVEFORMATEX,
        frags: *const SPVTEXTFRAG,
        site: Ref<ISpTTSEngineSite>,
    ) -> windows::core::Result<()> {
        let site = site.ok()?;
        let record = SpokenRecord {
            t: now_ms(),
            frags: fragments(frags),
        };
        let (log, delegate) = match self.state.lock() {
            Ok(state) => (state.log.clone(), state.delegate.clone()),
            Err(_) => (None, None),
        };
        if let Some(log) = &log {
            append(log, &record);
        }
        match delegate {
            // SAFETY: the caller's arguments are passed on unchanged.
            Some(delegate) => unsafe { delegate.Speak(flags, format, wave, frags, site) },
            None => tone(site, record.text().chars().count()),
        }
    }

    fn GetOutputFormat(
        &self,
        target_format: *const GUID,
        target_wave: *const WAVEFORMATEX,
        format: *mut GUID,
        wave: *mut *mut WAVEFORMATEX,
    ) -> windows::core::Result<()> {
        let delegate = self.state.lock().ok().and_then(|s| s.delegate.clone());
        if let Some(delegate) = delegate {
            // SAFETY: the caller's arguments are passed on unchanged.
            return unsafe { delegate.GetOutputFormat(target_format, target_wave, format, wave) };
        }
        if format.is_null() || wave.is_null() {
            return Err(E_POINTER.into());
        }
        // SAFETY: out pointers checked above; the format is CoTaskMemAlloc'd for the caller.
        unsafe {
            let block = CoTaskMemAlloc(std::mem::size_of::<WAVEFORMATEX>()).cast::<WAVEFORMATEX>();
            if block.is_null() {
                return Err(windows::Win32::Foundation::E_OUTOFMEMORY.into());
            }
            block.write(WAVEFORMATEX {
                wFormatTag: 1, // WAVE_FORMAT_PCM
                nChannels: 1,
                nSamplesPerSec: FALLBACK_RATE,
                nAvgBytesPerSec: FALLBACK_RATE * 2,
                nBlockAlign: 2,
                wBitsPerSample: 16,
                cbSize: 0,
            });
            *format = SPDFID_WAVE_FORMAT_EX;
            *wave = block;
        }
        Ok(())
    }
}

impl ISpObjectWithToken_Impl for Recorder_Impl {
    fn SetObjectToken(&self, token: Ref<ISpObjectToken>) -> windows::core::Result<()> {
        let token = token.ok()?.clone();
        let log = token_string(&token, windows::core::w!("LogFile")).map(PathBuf::from);
        let delegate = token_string(&token, windows::core::w!("Delegate"))
            .and_then(|id| create_delegate(&id).ok());
        if let Ok(mut state) = self.state.lock() {
            *state = State {
                token: Some(token),
                log,
                delegate,
            };
        }
        Ok(())
    }

    fn GetObjectToken(&self) -> windows::core::Result<ISpObjectToken> {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.token.clone())
            .ok_or_else(|| S_FALSE.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Media::Speech::{
        ISpEventSink_Impl, ISpTTSEngineSite_Impl, SPVA_Speak, SPVA_SpellOut, SPEVENT, SPVSKIPTYPE,
    };

    /// A synthesizer site that counts the audio bytes written to it.
    #[implement(ISpTTSEngineSite)]
    #[derive(Default)]
    struct CountingSite {
        bytes: Mutex<u32>,
    }

    impl ISpEventSink_Impl for CountingSite_Impl {
        fn AddEvents(&self, _events: *const SPEVENT, _count: u32) -> windows::core::Result<()> {
            Ok(())
        }
        fn GetEventInterest(&self, interest: *mut u64) -> windows::core::Result<()> {
            // SAFETY: out pointer from the caller.
            unsafe { interest.write(0) };
            Ok(())
        }
    }

    impl ISpTTSEngineSite_Impl for CountingSite_Impl {
        fn GetActions(&self) -> u32 {
            0
        }
        fn Write(&self, _buffer: *const c_void, cb: u32) -> windows::core::Result<u32> {
            if let Ok(mut bytes) = self.bytes.lock() {
                *bytes += cb;
            }
            Ok(cb)
        }
        fn GetRate(&self) -> windows::core::Result<i32> {
            Ok(0)
        }
        fn GetVolume(&self) -> windows::core::Result<u16> {
            Ok(100)
        }
        fn GetSkipInfo(
            &self,
            _kind: *mut SPVSKIPTYPE,
            _items: *mut i32,
        ) -> windows::core::Result<()> {
            Ok(())
        }
        fn CompleteSkip(&self, _skipped: i32) -> windows::core::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn speak_logs_every_fragment_and_still_produces_audio() {
        let log = std::env::temp_dir().join(format!("sr-recorder-test-{}.jsonl", now_ms()));
        let recorder = Recorder::default();
        if let Ok(mut state) = recorder.state.lock() {
            state.log = Some(log.clone());
        }
        let engine: ISpTTSEngine = recorder.into();

        let first: Vec<u16> = "left paren".encode_utf16().collect();
        let second: Vec<u16> = "B".encode_utf16().collect();
        let mut tail = SPVTEXTFRAG::default();
        tail.State.eAction = SPVA_SpellOut;
        tail.State.PitchAdj.MiddleAdj = 4;
        tail.State.Volume = 100;
        tail.pTextStart = PCWSTR(second.as_ptr());
        tail.ulTextLen = 1;
        let mut head = SPVTEXTFRAG::default();
        head.State.eAction = SPVA_Speak;
        head.State.Volume = 100;
        head.pTextStart = PCWSTR(first.as_ptr());
        head.ulTextLen = 10;
        head.pNext = &mut tail;

        let site_impl = CountingSite::default();
        let site: ISpTTSEngineSite = site_impl.into();
        // SAFETY: the fragment list and its text outlive the call.
        let spoken =
            unsafe { engine.Speak(0, &SPDFID_WAVE_FORMAT_EX, std::ptr::null(), &head, &site) };
        assert!(spoken.is_ok(), "{spoken:?}");

        let written = std::fs::read_to_string(&log).unwrap_or_default();
        let _ = std::fs::remove_file(&log);
        let record: Option<SpokenRecord> = written
            .lines()
            .next()
            .and_then(|l| serde_json::from_str(l).ok());
        let record = record.unwrap_or(SpokenRecord {
            t: 0,
            frags: Vec::new(),
        });
        assert_eq!(record.text(), "left paren B");
        assert_eq!(
            record.frags.get(1).map(|f| (f.action.as_str(), f.pitch)),
            Some(("spell", 4))
        );
    }
}
