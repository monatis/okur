//! The system clock and the user's locale formats ([`okur_core::clock::Clock`]).

#[cfg(windows)]
pub use windows_clock::WindowsClock;

#[cfg(windows)]
mod windows_clock {
    use okur_core::clock::{Clock, FirstWeek, LocalDateTime, LocaleFormats};
    use windows::core::PCWSTR;
    use windows::Win32::Globalization::{
        GetLocaleInfoEx, LOCALE_IFIRSTDAYOFWEEK, LOCALE_IFIRSTWEEKOFYEAR, LOCALE_S1159,
        LOCALE_S2359, LOCALE_SDAYNAME1, LOCALE_SLONGDATE, LOCALE_SMONTHNAME1, LOCALE_SSHORTTIME,
        LOCALE_STIMEFORMAT,
    };
    use windows::Win32::System::SystemInformation::GetLocalTime;

    /// The Windows local time, formatted with the user's locale (Region settings).
    #[derive(Debug, Default, Clone, Copy)]
    pub struct WindowsClock;

    /// A locale string of the user's default locale.
    fn locale_string(item: u32) -> String {
        let mut buf = [0u16; 128];
        // SAFETY: the buffer is valid for the call; a null name means the user default.
        let n = unsafe { GetLocaleInfoEx(PCWSTR::null(), item, Some(&mut buf)) };
        let len = usize::try_from(n).unwrap_or(0).saturating_sub(1);
        String::from_utf16_lossy(buf.get(..len).unwrap_or_default())
    }

    impl Clock for WindowsClock {
        fn now(&self) -> LocalDateTime {
            // SAFETY: plain Win32 call with no preconditions.
            let t = unsafe { GetLocalTime() };
            LocalDateTime {
                year: i32::from(t.wYear),
                month: u32::from(t.wMonth),
                day: u32::from(t.wDay),
                hour: u32::from(t.wHour),
                minute: u32::from(t.wMinute),
                second: u32::from(t.wSecond),
            }
        }

        fn formats(&self) -> LocaleFormats {
            // Windows numbers day names from Monday (SDAYNAME1) to Sunday (SDAYNAME7), and the
            // first day of the week from 0 for Monday.
            let days = std::array::from_fn(|i| {
                let monday_based = u32::try_from((i + 6) % 7).unwrap_or(0);
                locale_string(LOCALE_SDAYNAME1 + monday_based)
            });
            let first_day = locale_string(LOCALE_IFIRSTDAYOFWEEK)
                .parse::<u32>()
                .map_or(0, |d| (d + 1) % 7);
            let first_week = match locale_string(LOCALE_IFIRSTWEEKOFYEAR).as_str() {
                "1" => FirstWeek::FirstFullWeek,
                "2" => FirstWeek::FirstFourDayWeek,
                _ => FirstWeek::ContainsJanuaryFirst,
            };
            LocaleFormats {
                short_time: locale_string(LOCALE_SSHORTTIME),
                long_time: locale_string(LOCALE_STIMEFORMAT),
                long_date: locale_string(LOCALE_SLONGDATE),
                months: std::array::from_fn(|i| {
                    locale_string(LOCALE_SMONTHNAME1 + u32::try_from(i).unwrap_or(0))
                }),
                days,
                am: locale_string(LOCALE_S1159),
                pm: locale_string(LOCALE_S2359),
                first_day_of_week: first_day,
                first_week,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Reads the machine's locale, but needs no UI, speech or network.
        #[test]
        fn the_user_locale_has_time_and_date_patterns() {
            let formats = WindowsClock.formats();
            assert!(formats.short_time.contains('m'), "{formats:?}");
            assert!(formats.long_date.contains('y'), "{formats:?}");
            assert!(formats.days.iter().all(|d| !d.is_empty()), "{formats:?}");
            assert!(formats.first_day_of_week < 7);
        }
    }
}
