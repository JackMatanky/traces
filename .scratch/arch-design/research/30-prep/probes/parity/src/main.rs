use std::mem::size_of;

use jiff::civil::Date;
use pretty_assertions::assert_eq;

fn legacy(s: &[u8]) -> Result<Date, &'static str> {
    let date = speedate::Date::parse_bytes_rfc3339(s).map_err(|e| match e {
        speedate::ParseError::OutOfRangeMonth
        | speedate::ParseError::OutOfRangeDay => "a field is out of range",
        _ => "invalid input",
    })?;
    Date::new(date.year as i16, date.month as i8, date.day as i8)
        .map_err(|_| "a field is out of range")
}

fn candidate(s: &[u8]) -> Result<Date, &'static str> {
    if s.len() != 10
        || s[4] != b'-'
        || s[7] != b'-'
        || !s[..4]
            .iter()
            .chain(s[5..7].iter())
            .chain(s[8..].iter())
            .all(u8::is_ascii_digit)
    {
        return Err("invalid input");
    }
    let year = s[..4].iter().fold(0i16, |v, b| v * 10 + i16::from(b - b'0'));
    let month = ((s[5] - b'0') * 10 + s[6] - b'0') as i8;
    let day = ((s[8] - b'0') * 10 + s[9] - b'0') as i8;
    Date::new(year, month, day).map_err(|_| "a field is out of range")
}

enum DateTimePayload {
    Civil(jiff::civil::DateTime),
    Instant(jiff::Timestamp),
}

fn main() {
    let mut counts = [0usize; 5]; // checked, valid, acceptance, value, error-category differences
    let mut examples = Vec::new();
    let mut check = |s: &[u8]| {
        let a = legacy(s);
        let b = candidate(s);
        counts[0] += 1;
        counts[1] += usize::from(a.is_ok());
        counts[2] += usize::from(a.is_ok() != b.is_ok());
        counts[3] += usize::from(matches!((&a, &b), (Ok(x), Ok(y)) if x != y));
        counts[4] +=
            usize::from(matches!((&a, &b), (Err(x), Err(y)) if x != y));
        if a != b && examples.len() < 10 {
            examples.push(format!("{:?}: legacy={a:?} candidate={b:?}", s));
        }
    };
    for year in 0..=9999 {
        for month in 0..=13 {
            for day in 0..=32 {
                check(format!("{year:04}-{month:02}-{day:02}").as_bytes());
            }
        }
    }
    for seed in [
        b"2024-02-29",
        b"1900-02-29",
        b"0000-01-01",
        b"9999-12-31",
        b"2026-00-00",
        b"2026-13-32",
    ] {
        for index in 0..10 {
            for byte in 0..=255 {
                let mut s = *seed;
                s[index] = byte;
                check(&s);
            }
        }
    }
    for s in [
        "",
        "2026",
        "2026-01",
        "20260101",
        "2026-01-01T00:00:00",
        "2026-01-01Z",
        " 2026-01-01",
        "2026-01-01 ",
        "2026-1-01",
        "２０２６-01-01",
    ] {
        check(s.as_bytes());
    }
    println!(
        "checked={} legacy_valid={} acceptance_differences={} \
         value_differences={} error_category_differences={}",
        counts[0], counts[1], counts[2], counts[3], counts[4]
    );
    for example in examples {
        println!("DIFFERENCE {example}");
    }
    for s in [
        "20260101",
        "2026-01-01T00:00:00",
        "2026-01-01Z",
        "0000-01-01",
        "-0009-01-01",
    ] {
        println!("raw_jiff {s:?}={:?}", s.parse::<Date>());
    }
    let variants = [
        DateTimePayload::Civil(
            "2026-01-01T12:00:00".parse().expect("civil fixture"),
        ),
        DateTimePayload::Instant(jiff::Timestamp::UNIX_EPOCH),
    ];
    for value in variants {
        match value {
            DateTimePayload::Civil(d) => println!("payload civil={d}"),
            DateTimePayload::Instant(t) => println!("payload instant={t}"),
        }
    }
    println!(
        "sizes Date={} DateTime={} Timestamp={} Span={} DateTimePayload={} \
         PeriodTuple={}",
        size_of::<Date>(),
        size_of::<jiff::civil::DateTime>(),
        size_of::<jiff::Timestamp>(),
        size_of::<jiff::Span>(),
        size_of::<DateTimePayload>(),
        size_of::<(Date, u8)>()
    );
    assert_eq!(counts[2], 0, "guarded acceptance differs");
    assert_eq!(counts[3], 0, "guarded values differ");
    assert_eq!(counts[4], 0, "error categories differ");
}
