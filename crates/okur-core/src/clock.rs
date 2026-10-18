//! The system clock and locale formats behind Say System Time (`Insert+F12`).
//!
//! JAWS speaks the time and date in the user's locale formats: on an en-US system
//! `10:11 PM`, `10:11:47 PM` with seconds, `Wednesday, October 14, 2026` for the date, and
//! `Week 42` [obs, 308]. The platform supplies the local time and the locale's patterns
//! (Windows `GetLocaleInfoEx`); the formatting here interprets them, so live use and parity
//! replay (a fixed clock at the recorded time, the recorded culture) run the same code.

/// A local calendar date and wall-clock time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalDateTime {
    pub year: i32,
    /// 1 to 12.
    pub month: u32,
    /// 1 to 31.
    pub day: u32,
    /// 0 to 23.
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = i64::from(year) - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

impl LocalDateTime {
    /// The local time `unix_ms` milliseconds after the epoch, `utc_offset_minutes` east of UTC.
    pub fn from_unix_ms(unix_ms: i64, utc_offset_minutes: i32) -> Self {
        let local_secs = unix_ms.div_euclid(1000) + i64::from(utc_offset_minutes) * 60;
        let days = local_secs.div_euclid(86_400);
        let secs = local_secs.rem_euclid(86_400);
        // Inverse of `days_from_civil`.
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        Self {
            year: i32::try_from(year).unwrap_or(1970),
            month: u32::try_from(month).unwrap_or(1),
            day: u32::try_from(day).unwrap_or(1),
            hour: u32::try_from(secs / 3600).unwrap_or(0),
            minute: u32::try_from(secs % 3600 / 60).unwrap_or(0),
            second: u32::try_from(secs % 60).unwrap_or(0),
        }
    }

    /// 0 for Sunday to 6 for Saturday.
    pub fn weekday(&self) -> u32 {
        // 1970-01-01 was a Thursday.
        u32::try_from((days_from_civil(self.year, self.month, self.day) + 4).rem_euclid(7))
            .unwrap_or(0)
    }

    /// 1 for January 1st.
    pub fn day_of_year(&self) -> u32 {
        u32::try_from(
            days_from_civil(self.year, self.month, self.day) - days_from_civil(self.year, 1, 1),
        )
        .unwrap_or(0)
            + 1
    }
}

/// How a locale numbers the weeks of a year (Windows `LOCALE_IFIRSTWEEKOFYEAR`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstWeek {
    /// Week 1 contains January 1st.
    ContainsJanuaryFirst,
    /// Week 1 is the first full week.
    FirstFullWeek,
    /// Week 1 is the first with at least four days (ISO 8601 with a Monday start).
    FirstFourDayWeek,
}

/// A locale's time and date patterns (Windows picture strings such as `h:mm tt`) and names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleFormats {
    pub short_time: String,
    pub long_time: String,
    pub long_date: String,
    /// January first.
    pub months: [String; 12],
    /// Sunday first.
    pub days: [String; 7],
    pub am: String,
    pub pm: String,
    /// 0 for Sunday to 6 for Saturday.
    pub first_day_of_week: u32,
    pub first_week: FirstWeek,
}

impl LocaleFormats {
    /// Windows' en-US defaults, the reference machine's culture.
    pub fn en_us() -> Self {
        let names = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let months = names(&[
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ]);
        let days = names(&[
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ]);
        Self {
            short_time: "h:mm tt".into(),
            long_time: "h:mm:ss tt".into(),
            long_date: "dddd, MMMM d, yyyy".into(),
            months: std::array::from_fn(|i| months.get(i).cloned().unwrap_or_default()),
            days: std::array::from_fn(|i| days.get(i).cloned().unwrap_or_default()),
            am: "AM".into(),
            pm: "PM".into(),
            first_day_of_week: 0,
            first_week: FirstWeek::ContainsJanuaryFirst,
        }
    }

    /// Formats `at` with a Windows time or date picture string: `h hh H HH m mm s ss t tt d dd
    /// ddd dddd M MM MMM MMMM y yy yyyy`, with `'quoted'` literals.
    pub fn format(&self, picture: &str, at: &LocalDateTime) -> String {
        let chars: Vec<char> = picture.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while let Some(&c) = chars.get(i) {
            if c == '\'' {
                let end = chars
                    .iter()
                    .skip(i + 1)
                    .position(|&q| q == '\'')
                    .map_or(chars.len(), |p| i + 1 + p);
                out.extend(chars.get(i + 1..end).into_iter().flatten());
                i = end + 1;
                continue;
            }
            let run = chars.iter().skip(i).take_while(|&&r| r == c).count();
            let hour12 = match at.hour % 12 {
                0 => 12,
                h => h,
            };
            let pad = |n: u32| {
                if run >= 2 {
                    format!("{n:02}")
                } else {
                    n.to_string()
                }
            };
            let ampm = if at.hour < 12 { &self.am } else { &self.pm };
            let month = usize::try_from(at.month.saturating_sub(1)).unwrap_or(0);
            let weekday = usize::try_from(at.weekday()).unwrap_or(0);
            let piece = match c {
                'h' => pad(hour12),
                'H' => pad(at.hour),
                'm' => pad(at.minute),
                's' => pad(at.second),
                't' if run == 1 => ampm.chars().next().map(String::from).unwrap_or_default(),
                't' => ampm.clone(),
                'd' if run >= 4 => self.days.get(weekday).cloned().unwrap_or_default(),
                'd' if run == 3 => self
                    .days
                    .get(weekday)
                    .map(|d| d.chars().take(3).collect())
                    .unwrap_or_default(),
                'd' => pad(at.day),
                'M' if run >= 4 => self.months.get(month).cloned().unwrap_or_default(),
                'M' if run == 3 => self
                    .months
                    .get(month)
                    .map(|m| m.chars().take(3).collect())
                    .unwrap_or_default(),
                'M' => pad(at.month),
                'y' if run <= 2 => format!("{:02}", at.year.rem_euclid(100)),
                'y' => at.year.to_string(),
                other => other.to_string().repeat(run),
            };
            out.push_str(&piece);
            i += run;
        }
        out.trim().to_owned()
    }

    /// The week of the year containing `at`, under this locale's rules.
    pub fn week_number(&self, at: &LocalDateTime) -> u32 {
        let jan1 = LocalDateTime {
            month: 1,
            day: 1,
            ..*at
        };
        // Days of week 0 that fall before January 1st.
        let lead = (jan1.weekday() + 7 - self.first_day_of_week) % 7;
        let week = (at.day_of_year() - 1 + lead) / 7 + 1;
        match self.first_week {
            FirstWeek::ContainsJanuaryFirst => week,
            FirstWeek::FirstFullWeek if lead == 0 => week,
            FirstWeek::FirstFourDayWeek if lead <= 3 => week,
            // The partial first week belongs to the previous year.
            _ if week > 1 => week - 1,
            _ => {
                let dec31 = LocalDateTime {
                    year: at.year - 1,
                    month: 12,
                    day: 31,
                    ..*at
                };
                self.week_number(&dec31)
            }
        }
    }
}

/// The current local time and the user's locale formats.
pub trait Clock: Send + Sync {
    fn now(&self) -> LocalDateTime;
    fn formats(&self) -> LocaleFormats;
}

/// A clock stopped at a set time: parity replay sets it to each step's recorded time.
#[derive(Debug, Clone)]
pub struct FixedClock {
    now: std::sync::Arc<std::sync::Mutex<LocalDateTime>>,
    formats: LocaleFormats,
}

impl FixedClock {
    pub fn new(now: LocalDateTime, formats: LocaleFormats) -> Self {
        Self {
            now: std::sync::Arc::new(std::sync::Mutex::new(now)),
            formats,
        }
    }

    /// Moves the clock (shared by clones).
    pub fn set(&self, now: LocalDateTime) {
        if let Ok(mut current) = self.now.lock() {
            *current = now;
        }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> LocalDateTime {
        self.now
            .lock()
            .map(|n| *n)
            .unwrap_or(LocalDateTime::from_unix_ms(0, 0))
    }

    fn formats(&self) -> LocaleFormats {
        self.formats.clone()
    }
}

/// What Say System Time speaks: the time, with seconds for `Ctrl+Insert+F12`.
pub fn say_time(clock: &dyn Clock, seconds: bool) -> String {
    let formats = clock.formats();
    let picture = if seconds {
        &formats.long_time
    } else {
        &formats.short_time
    };
    formats.format(picture, &clock.now())
}

/// What Say System Time pressed twice quickly speaks: the date, or with `Ctrl` the week.
pub fn say_date(clock: &dyn Clock, week: bool) -> String {
    let formats = clock.formats();
    let now = clock.now();
    if week {
        crate::messages::msg::week_number(formats.week_number(&now)).render()
    } else {
        formats.format(&formats.long_date, &now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-14 22:11:47 at UTC+3, as recorded on the reference machine [obs, 308 trial 1].
    fn recorded() -> LocalDateTime {
        LocalDateTime::from_unix_ms(1_792_005_107_069, 180)
    }

    #[test]
    fn unix_time_becomes_local_calendar_time() {
        assert_eq!(
            recorded(),
            LocalDateTime {
                year: 2026,
                month: 10,
                day: 14,
                hour: 22,
                minute: 11,
                second: 47
            }
        );
        assert_eq!(recorded().weekday(), 3);
        assert_eq!(recorded().day_of_year(), 287);
        let leap = LocalDateTime::from_unix_ms(951_782_400_000, 0);
        assert_eq!((leap.year, leap.month, leap.day), (2000, 2, 29));
    }

    #[test]
    fn en_us_patterns_speak_as_jaws_does() {
        let clock = FixedClock::new(recorded(), LocaleFormats::en_us());
        assert_eq!(say_time(&clock, false), "10:11 PM");
        assert_eq!(say_time(&clock, true), "10:11:47 PM");
        assert_eq!(say_date(&clock, false), "Wednesday, October 14, 2026");
        assert_eq!(say_date(&clock, true), "Week 42");
    }

    #[test]
    fn pictures_cover_24_hour_padding_and_literals() {
        let f = LocaleFormats::en_us();
        let at = LocalDateTime::from_unix_ms(1_767_225_605_000, 0); // 2026-01-01 00:00:05
        assert_eq!(f.format("HH:mm:ss", &at), "00:00:05");
        assert_eq!(f.format("h:mm tt", &at), "12:00 AM");
        assert_eq!(f.format("d MMM yy 'at' H", &at), "1 Jan 26 at 0");
        assert_eq!(f.format("ddd dd/MM/yyyy", &at), "Thu 01/01/2026");
    }

    #[test]
    fn week_rules_differ_around_new_year() {
        // Friday 2027-01-01.
        let friday = LocalDateTime::from_unix_ms(1_798_761_600_000, 0);
        assert_eq!(friday.weekday(), 5);
        let mut iso = LocaleFormats::en_us();
        iso.first_day_of_week = 1;
        iso.first_week = FirstWeek::FirstFourDayWeek;
        assert_eq!(LocaleFormats::en_us().week_number(&friday), 1);
        // ISO 8601: 2027-01-01 is in week 53 of 2026.
        assert_eq!(iso.week_number(&friday), 53);
        assert_eq!(iso.week_number(&recorded()), 42);
    }
}
