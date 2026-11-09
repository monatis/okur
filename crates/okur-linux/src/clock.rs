//! The system clock and the user's locale formats on Linux ([`okur_core::clock::Clock`]), behind
//! Say System Time (`Insert+F12`) and Say System Date: the local time from the C library
//! (`localtime_r`, the system's time zone), and the time's pattern, AM and PM, and the day and
//! month names from the user's locale (`LC_TIME`, read through `nl_langinfo_l`, which unlike
//! `setlocale` touches no state other threads share). POSIX gives no long date pattern: the
//! date keeps en-US's ("dddd, MMMM d, yyyy") with the locale's names.

use okur_core::clock::{Clock, LocalDateTime, LocaleFormats};

/// The Linux local time, formatted with the user's locale.
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxClock;

impl Clock for LinuxClock {
    fn now(&self) -> LocalDateTime {
        local_now().unwrap_or_else(|| {
            // No local time from the C library: the time in UTC.
            let ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
                .unwrap_or(0);
            LocalDateTime::from_unix_ms(ms, 0)
        })
    }

    fn formats(&self) -> LocaleFormats {
        let mut formats = LocaleFormats::en_us();
        let Some(locale) = UserLocale::time() else {
            return formats;
        };
        if let Some(long) = locale.item(libc::T_FMT).as_deref().and_then(picture) {
            formats.short_time = without_seconds(&long);
            formats.long_time = long;
        }
        if let Some(am) = locale.item(libc::AM_STR).filter(|s| !s.is_empty()) {
            formats.am = am;
        }
        if let Some(pm) = locale.item(libc::PM_STR).filter(|s| !s.is_empty()) {
            formats.pm = pm;
        }
        let days = [
            libc::DAY_1,
            libc::DAY_2,
            libc::DAY_3,
            libc::DAY_4,
            libc::DAY_5,
            libc::DAY_6,
            libc::DAY_7,
        ];
        for (name, item) in formats.days.iter_mut().zip(days) {
            if let Some(day) = locale.item(item).filter(|s| !s.is_empty()) {
                *name = day;
            }
        }
        let months = [
            libc::MON_1,
            libc::MON_2,
            libc::MON_3,
            libc::MON_4,
            libc::MON_5,
            libc::MON_6,
            libc::MON_7,
            libc::MON_8,
            libc::MON_9,
            libc::MON_10,
            libc::MON_11,
            libc::MON_12,
        ];
        for (name, item) in formats.months.iter_mut().zip(months) {
            if let Some(month) = locale.item(item).filter(|s| !s.is_empty()) {
                *name = month;
            }
        }
        formats
    }
}

/// The local time now, from the C library; none where it has none.
fn local_now() -> Option<LocalDateTime> {
    // SAFETY: time(NULL) only returns the time.
    let now = unsafe { libc::time(std::ptr::null_mut()) };
    // SAFETY: a zeroed tm is a valid value of the plain C struct, which localtime_r fills.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers are valid for the call, and localtime_r keeps neither.
    if unsafe { libc::localtime_r(&now, &mut tm) }.is_null() {
        return None;
    }
    let part = |value: libc::c_int| u32::try_from(value).ok();
    Some(LocalDateTime {
        year: tm.tm_year.checked_add(1900)?,
        month: part(tm.tm_mon)?.checked_add(1)?,
        day: part(tm.tm_mday)?,
        hour: part(tm.tm_hour)?,
        minute: part(tm.tm_min)?,
        // A leap second is said as the last of its minute.
        second: part(tm.tm_sec)?.min(59),
    })
}

/// The user's locale for times (`LC_TIME` from the environment), freed when dropped.
struct UserLocale(libc::locale_t);

impl UserLocale {
    fn time() -> Option<Self> {
        // SAFETY: an empty name takes the locale from the environment; a null base makes a new
        // locale object, which is freed on drop. A null return is no locale.
        let locale =
            unsafe { libc::newlocale(libc::LC_TIME_MASK, c"".as_ptr(), std::ptr::null_mut()) };
        (!locale.is_null()).then_some(Self(locale))
    }

    /// One of the locale's strings, copied at once (the C library may reuse its buffer).
    fn item(&self, item: libc::nl_item) -> Option<String> {
        // SAFETY: the locale object is valid until drop; the returned string is read before any
        // other call on it.
        let text = unsafe { libc::nl_langinfo_l(item, self.0) };
        if text.is_null() {
            return None;
        }
        // SAFETY: nl_langinfo_l returns a NUL-terminated string.
        let text = unsafe { std::ffi::CStr::from_ptr(text) };
        Some(text.to_string_lossy().into_owned())
    }
}

impl Drop for UserLocale {
    fn drop(&mut self) {
        // SAFETY: the object came from newlocale and is freed once.
        unsafe { libc::freelocale(self.0) };
    }
}

/// A POSIX time format (`%H:%M:%S`, `%r`) as the picture string Okur's clock reads (Windows'
/// `HH:mm:ss`, `h:mm:ss tt`); none for a directive it has no picture for. A 12-hour hour is
/// said without its leading zero, as en-US Windows' `h` says it ("9:05 PM").
fn picture(posix: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = posix.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            // A letter would be a picture's own directive: quoted.
            if c.is_ascii_alphabetic() {
                out.push('\'');
                out.push(c);
                out.push('\'');
            } else {
                out.push(c);
            }
            continue;
        }
        out.push_str(match chars.next()? {
            'H' => "HH",
            'k' => "H",
            'I' | 'l' => "h",
            'M' => "mm",
            'S' => "ss",
            'p' | 'P' => "tt",
            'r' => "h:mm:ss tt",
            'T' => "HH:mm:ss",
            'R' => "HH:mm",
            '%' => "%",
            _ => return None,
        });
    }
    (!out.is_empty()).then_some(out)
}

/// A time picture without its seconds: "h:mm:ss tt" is "h:mm tt".
fn without_seconds(picture: &str) -> String {
    picture
        .replace(":ss", "")
        .replace(".ss", "")
        .replace("ss", "")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// POSIX's time formats as pictures: the C locale's 24 hours, en_US's 12 hours with AM
    /// and PM, and none for a directive without a picture.
    #[test]
    fn posix_time_formats_as_pictures() {
        assert_eq!(picture("%H:%M:%S").as_deref(), Some("HH:mm:ss"));
        assert_eq!(picture("%r").as_deref(), Some("h:mm:ss tt"));
        assert_eq!(picture("%I:%M:%S %p").as_deref(), Some("h:mm:ss tt"));
        assert_eq!(picture("%T").as_deref(), Some("HH:mm:ss"));
        assert_eq!(picture("%H.%M.%S").as_deref(), Some("HH.mm.ss"));
        assert_eq!(picture("%H h %M").as_deref(), Some("HH 'h' mm"));
        assert_eq!(picture("%E"), None);
        assert_eq!(without_seconds("h:mm:ss tt"), "h:mm tt");
        assert_eq!(without_seconds("HH:mm:ss"), "HH:mm");
        assert_eq!(without_seconds("HH.mm.ss"), "HH.mm");
    }

    /// The clock reads the system's time and the locale's names (no UI, speech or network).
    #[test]
    fn the_clock_has_a_time_and_names() {
        let now = LinuxClock.now();
        assert!((1..=12).contains(&now.month) && now.hour < 24, "{now:?}");
        let formats = LinuxClock.formats();
        assert!(formats.short_time.contains("mm"), "{formats:?}");
        assert!(formats.days.iter().all(|d| !d.is_empty()), "{formats:?}");
    }
}
