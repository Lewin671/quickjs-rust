use crate::date::{MS_PER_DAY, MS_PER_HOUR, MS_PER_MINUTE, MS_PER_SECOND};

pub(super) struct UtcDateTime {
    pub(super) year: i32,
    pub(super) month: i32,
    pub(super) date: i32,
    pub(super) day: i32,
    pub(super) hours: i32,
    pub(super) minutes: i32,
    pub(super) seconds: i32,
    pub(super) milliseconds: i32,
}

pub(super) fn parse_iso_string(source: &str) -> Option<f64> {
    let bytes = source.as_bytes();
    if bytes.len() < 20 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
        return None;
    }
    let year = parse_i32(&source[0..4])?;
    let month = parse_i32(&source[5..7])?;
    let day = parse_i32(&source[8..10])?;
    if bytes.get(10) != Some(&b'T') {
        return None;
    }
    let hours = parse_i32(&source[11..13])?;
    let minutes = parse_i32(&source[14..16])?;
    let seconds = parse_i32(&source[17..19])?;
    if bytes.get(13) != Some(&b':') || bytes.get(16) != Some(&b':') {
        return None;
    }

    let mut cursor = 19;
    let mut millis = 0;
    if bytes.get(cursor) == Some(&b'.') {
        let start = cursor + 1;
        cursor = start;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        let digits = &source[start..cursor];
        if digits.is_empty() {
            return None;
        }
        millis = parse_millis(digits);
    }
    if bytes.get(cursor) != Some(&b'Z') || cursor + 1 != bytes.len() {
        return None;
    }
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=23).contains(&hours)
        || !(0..=59).contains(&minutes)
        || !(0..=59).contains(&seconds)
    {
        return None;
    }

    Some(
        days_from_civil(year, month, day) as f64 * MS_PER_DAY
            + f64::from(hours) * MS_PER_HOUR
            + f64::from(minutes) * MS_PER_MINUTE
            + f64::from(seconds) * MS_PER_SECOND
            + f64::from(millis),
    )
}

pub(super) fn format_iso_string(millis: f64) -> String {
    let components = utc_date_time(millis);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        components.year,
        components.month + 1,
        components.date,
        components.hours,
        components.minutes,
        components.seconds,
        components.milliseconds
    )
}

pub(super) fn format_utc_string(millis: f64) -> String {
    const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let components = utc_date_time(millis);
    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        WEEKDAYS[components.day as usize],
        components.date,
        MONTHS[components.month as usize],
        components.year,
        components.hours,
        components.minutes,
        components.seconds
    )
}

pub(super) fn utc_date_time(millis: f64) -> UtcDateTime {
    let time = millis.trunc();
    let days = (time / MS_PER_DAY).floor() as i64;
    let mut within_day = time - days as f64 * MS_PER_DAY;
    if within_day < 0.0 {
        within_day += MS_PER_DAY;
    }
    let (year, month, day) = civil_from_days(days);
    UtcDateTime {
        year,
        month: month - 1,
        date: day,
        day: (days + 4).rem_euclid(7) as i32,
        hours: (within_day / MS_PER_HOUR).floor() as i32,
        minutes: ((within_day % MS_PER_HOUR) / MS_PER_MINUTE).floor() as i32,
        seconds: ((within_day % MS_PER_MINUTE) / MS_PER_SECOND).floor() as i32,
        milliseconds: (within_day % MS_PER_SECOND).floor() as i32,
    }
}

pub(super) fn days_from_civil(year: i32, month: i32, day: i32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::from(era * 146_097 + day_of_era - 719_468)
}

fn parse_millis(digits: &str) -> i32 {
    let mut value = 0;
    let mut scale = 100;
    for digit in digits.bytes().take(3) {
        value += i32::from(digit - b'0') * scale;
        scale /= 10;
    }
    value
}

fn parse_i32(source: &str) -> Option<i32> {
    source.parse::<i32>().ok()
}

fn civil_from_days(days: i64) -> (i32, i32, i32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era as i32 + (era as i32) * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i32::from(month <= 2);
    (year, month as i32, day as i32)
}
