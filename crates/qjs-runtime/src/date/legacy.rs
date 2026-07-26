//! Parsing for the implementation-defined date formats that web content
//! actually uses.
//!
//! `Date.parse` must accept the Date Time String Format; everything else is
//! implementation defined, and in practice every engine accepts the shapes a
//! human writes: `1/1/2007 1:11:11`, `January 1 2001 00:00:00 +0000`,
//! `Jan 1, 2007`. Rejecting them makes ordinary date code silently produce
//! `NaN`, so this parser recognizes the same family of inputs as the reference
//! engines.
//!
//! The grammar is deliberately token-directed rather than positional: the
//! source is split into words, and each word is classified as a month name, a
//! slash-separated numeric date, a clock time, a time-zone offset, a meridiem
//! marker, or a bare number. A bare number becomes the year when it cannot be
//! a day, and the day otherwise, which is what the reference engines do.

use super::iso::days_from_civil;

const MS_PER_DAY: f64 = 86_400_000.0;
const MS_PER_HOUR: f64 = 3_600_000.0;
const MS_PER_MINUTE: f64 = 60_000.0;
const MS_PER_SECOND: f64 = 1_000.0;

#[derive(Default)]
struct Parts {
    year: Option<i32>,
    month: Option<i32>,
    day: Option<i32>,
    hours: Option<i32>,
    minutes: i32,
    seconds: i32,
    milliseconds: i32,
    /// Offset east of UTC in minutes. `None` means the input named no zone;
    /// the runtime's local time is UTC, so it is treated as UTC.
    zone_minutes: Option<i32>,
    meridiem: Option<Meridiem>,
}

#[derive(Clone, Copy, PartialEq)]
enum Meridiem {
    Am,
    Pm,
}

pub(super) fn parse_legacy_string(source: &str) -> Option<f64> {
    let stripped = strip_comments(source);
    let mut parts = Parts::default();
    let mut saw_any = false;
    for word in stripped
        .split(|character: char| character.is_whitespace() || character == ',')
        .filter(|word| !word.is_empty())
    {
        if !consume_word(word, &mut parts) {
            return None;
        }
        saw_any = true;
    }
    if !saw_any {
        return None;
    }
    build(parts)
}

/// Removes parenthesized comments, which the reference engines ignore, as in
/// `Mon Jan 01 2007 00:00:00 GMT+0000 (Coordinated Universal Time)`.
fn strip_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut depth = 0_usize;
    for character in source.chars() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => result.push(character),
            _ => {}
        }
    }
    result
}

fn consume_word(word: &str, parts: &mut Parts) -> bool {
    if let Some(month) = month_from_name(word) {
        if parts.month.is_some() {
            return false;
        }
        parts.month = Some(month);
        return true;
    }
    if word.eq_ignore_ascii_case("am") || word.eq_ignore_ascii_case("a.m.") {
        return set_meridiem(parts, Meridiem::Am);
    }
    if word.eq_ignore_ascii_case("pm") || word.eq_ignore_ascii_case("p.m.") {
        return set_meridiem(parts, Meridiem::Pm);
    }
    if let Some(zone) = zone_from_word(word) {
        if parts.zone_minutes.is_some() {
            return false;
        }
        parts.zone_minutes = Some(zone);
        return true;
    }
    if word.contains('/') {
        return consume_slashed_date(word, parts);
    }
    if word.contains(':') {
        return consume_time(word, parts);
    }
    // A weekday name carries no information the rest of the string does not.
    if is_weekday_name(word) {
        return true;
    }
    consume_number(word, parts)
}

fn set_meridiem(parts: &mut Parts, meridiem: Meridiem) -> bool {
    if parts.meridiem.is_some() {
        return false;
    }
    parts.meridiem = Some(meridiem);
    true
}

fn consume_slashed_date(word: &str, parts: &mut Parts) -> bool {
    if parts.month.is_some() || parts.day.is_some() {
        return false;
    }
    let fields: Vec<&str> = word.split('/').collect();
    if fields.len() != 3 && fields.len() != 2 {
        return false;
    }
    let mut numbers = Vec::with_capacity(fields.len());
    for field in &fields {
        match parse_unsigned(field) {
            Some(value) => numbers.push(value),
            None => return false,
        }
    }
    parts.month = Some(numbers[0]);
    parts.day = Some(numbers[1]);
    if let Some(year) = numbers.get(2) {
        parts.year = Some(expand_two_digit_year(*year, fields[2].len()));
    }
    true
}

fn consume_time(word: &str, parts: &mut Parts) -> bool {
    if parts.hours.is_some() {
        return false;
    }
    let (clock, fraction) = match word.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (word, None),
    };
    let fields: Vec<&str> = clock.split(':').collect();
    if fields.len() < 2 || fields.len() > 3 {
        return false;
    }
    let Some(hours) = parse_unsigned(fields[0]) else {
        return false;
    };
    let Some(minutes) = parse_unsigned(fields[1]) else {
        return false;
    };
    let seconds = match fields.get(2) {
        Some(field) => match parse_unsigned(field) {
            Some(value) => value,
            None => return false,
        },
        None => 0,
    };
    parts.hours = Some(hours);
    parts.minutes = minutes;
    parts.seconds = seconds;
    if let Some(fraction) = fraction {
        if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
        let mut value = 0;
        let mut scale = 100;
        for digit in fraction.bytes().take(3) {
            value += i32::from(digit - b'0') * scale;
            scale /= 10;
        }
        parts.milliseconds = value;
    }
    true
}

fn consume_number(word: &str, parts: &mut Parts) -> bool {
    let (digits, negative) = match word.strip_prefix('-') {
        Some(rest) => (rest, true),
        None => (word.strip_prefix('+').unwrap_or(word), false),
    };
    let Some(value) = parse_unsigned(digits) else {
        return false;
    };
    let signed = if negative { -value } else { value };
    // A four-or-more digit number, a negative one, or one too large to be a
    // day is the year. Otherwise it fills the day, then the year.
    if negative || digits.len() >= 3 || value > 31 {
        if parts.year.is_some() {
            return false;
        }
        parts.year = Some(signed);
        return true;
    }
    if parts.day.is_none() && (parts.month.is_some() || parts.year.is_none()) {
        parts.day = Some(value);
        return true;
    }
    if parts.year.is_none() {
        parts.year = Some(expand_two_digit_year(value, digits.len()));
        return true;
    }
    false
}

fn expand_two_digit_year(year: i32, digit_count: usize) -> i32 {
    if digit_count > 2 {
        return year;
    }
    // The reference engines map 0-49 to 2000-2049 and 50-99 to 1950-1999.
    if year < 50 { year + 2000 } else { year + 1900 }
}

fn parse_unsigned(source: &str) -> Option<i32> {
    if source.is_empty() || !source.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    source.parse::<i32>().ok()
}

fn zone_from_word(word: &str) -> Option<i32> {
    if word.eq_ignore_ascii_case("z")
        || word.eq_ignore_ascii_case("ut")
        || word.eq_ignore_ascii_case("utc")
        || word.eq_ignore_ascii_case("gmt")
    {
        return Some(0);
    }
    let named_zone_prefix = word.len() > 3
        && (word[..3].eq_ignore_ascii_case("gmt") || word[..3].eq_ignore_ascii_case("utc"));
    let rest = if named_zone_prefix { &word[3..] } else { word };
    let (sign, digits) = match rest.as_bytes().first()? {
        b'+' => (1, &rest[1..]),
        b'-' => (-1, &rest[1..]),
        _ => return None,
    };
    let (hours, minutes) = match digits.split_once(':') {
        Some((hours, minutes)) => (parse_unsigned(hours)?, parse_unsigned(minutes)?),
        None => match digits.len() {
            4 => (parse_unsigned(&digits[..2])?, parse_unsigned(&digits[2..])?),
            2 => (parse_unsigned(digits)?, 0),
            1 => (parse_unsigned(digits)?, 0),
            _ => return None,
        },
    };
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(sign * (hours * 60 + minutes))
}

fn month_from_name(word: &str) -> Option<i32> {
    const MONTHS: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    if word.len() < 3 {
        return None;
    }
    let lowered = word.trim_end_matches('.').to_ascii_lowercase();
    MONTHS
        .iter()
        .position(|month| month.starts_with(&lowered) && lowered.len() >= 3)
        .map(|index| index as i32 + 1)
}

fn is_weekday_name(word: &str) -> bool {
    const DAYS: [&str; 7] = [
        "sunday",
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
    ];
    if word.len() < 3 {
        return false;
    }
    let lowered = word.trim_end_matches('.').to_ascii_lowercase();
    DAYS.iter().any(|day| day.starts_with(&lowered))
}

fn build(parts: Parts) -> Option<f64> {
    let year = parts.year?;
    let month = parts.month?;
    let day = parts.day.unwrap_or(1);
    let mut hours = parts.hours.unwrap_or(0);
    match parts.meridiem {
        Some(Meridiem::Am) if !(1..=12).contains(&hours) => return None,
        Some(Meridiem::Am) => hours %= 12,
        Some(Meridiem::Pm) if !(1..=12).contains(&hours) => return None,
        Some(Meridiem::Pm) => hours = hours % 12 + 12,
        None => {}
    }
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=24).contains(&hours)
        || !(0..=59).contains(&parts.minutes)
        || !(0..=59).contains(&parts.seconds)
    {
        return None;
    }
    let millis = days_from_civil(year, month, day) as f64 * MS_PER_DAY
        + f64::from(hours) * MS_PER_HOUR
        + f64::from(parts.minutes) * MS_PER_MINUTE
        + f64::from(parts.seconds) * MS_PER_SECOND
        + f64::from(parts.milliseconds)
        - f64::from(parts.zone_minutes.unwrap_or(0)) * MS_PER_MINUTE;
    millis.is_finite().then_some(millis)
}

#[cfg(test)]
mod tests {
    use super::parse_legacy_string;

    fn parse(source: &str) -> Option<f64> {
        parse_legacy_string(source)
    }

    #[test]
    fn parses_the_shapes_web_content_writes() {
        // The runtime's local time zone is UTC, so a zone-less input is UTC.
        assert_eq!(parse("1/1/2007 1:11:11"), Some(1_167_613_871_000.0));
        assert_eq!(parse("Jan 1, 2007"), Some(1_167_609_600_000.0));
        assert_eq!(
            parse("January 1 2001 00:00:00 +0000"),
            Some(978_307_200_000.0)
        );
        assert_eq!(parse("1 January 2001"), Some(978_307_200_000.0));
        assert_eq!(
            parse("Mon Jan 01 2007 00:00:00 GMT+0000 (Coordinated Universal Time)"),
            Some(1_167_609_600_000.0)
        );
        assert_eq!(parse("December 25, 1995 13:30:00"), Some(819_898_200_000.0));
    }

    #[test]
    fn applies_meridiem_and_zone_offsets() {
        assert_eq!(
            parse("Jan 1 2007 12:00:00 AM"),
            parse("Jan 1 2007 00:00:00")
        );
        assert_eq!(
            parse("Jan 1 2007 12:00:00 PM"),
            parse("Jan 1 2007 12:00:00")
        );
        assert_eq!(parse("Jan 1 2007 1:00:00 PM"), parse("Jan 1 2007 13:00:00"));
        assert_eq!(
            parse("Jan 1 2007 00:00:00 GMT-0500"),
            parse("Jan 1 2007 05:00:00 GMT")
        );
        assert_eq!(
            parse("Jan 1 2007 00:00:00 +01:30"),
            parse("Dec 31 2006 22:30:00 UTC")
        );
    }

    #[test]
    fn rejects_text_that_names_no_date() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("not a date"), None);
        assert_eq!(parse("2007"), None);
        assert_eq!(parse("Jan 1 2007 25:00:00"), None);
        assert_eq!(parse("Jan 40 2007"), None);
        assert_eq!(parse("13/40/2007"), None);
    }

    #[test]
    fn expands_two_digit_years_like_the_reference_engines() {
        assert_eq!(parse("1/1/49"), parse("Jan 1 2049"));
        assert_eq!(parse("1/1/50"), parse("Jan 1 1950"));
        assert_eq!(parse("1/1/99"), parse("Jan 1 1999"));
    }
}
