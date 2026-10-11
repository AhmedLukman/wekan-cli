use time::{Date, OffsetDateTime, format_description::well_known::Rfc3339};

/// Wekan serializes JavaScript Dates using ISO strings, including signed six-digit years.
pub(crate) fn validate_response_date(value: &str) -> Result<(), ()> {
    if let Ok(date) = OffsetDateTime::parse(value, &Rfc3339) {
        return within_time_clip(date);
    }
    let bytes = value.as_bytes();
    if bytes.len() != 27
        || !matches!(bytes[0], b'+' | b'-')
        || !bytes[1..7].iter().all(u8::is_ascii_digit)
        || !bytes[23..26].iter().all(u8::is_ascii_digit)
        || bytes[26] != b'Z'
    {
        return Err(());
    }
    let year: i32 = value[..7].parse().map_err(|_| ())?;
    if value.starts_with("-000000") {
        return Err(());
    }
    let ordinary = format!("2000{}", &value[7..]);
    let parsed = OffsetDateTime::parse(&ordinary, &Rfc3339).map_err(|_| ())?;
    let date = Date::from_calendar_date(year, parsed.month(), parsed.day()).map_err(|_| ())?;
    within_time_clip(date.with_time(parsed.time()).assume_utc())
}

fn within_time_clip(date: OffsetDateTime) -> Result<(), ()> {
    if date.unix_timestamp_nanos().abs() <= 8_640_000_000_000_000_000_000 {
        Ok(())
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::validate_response_date;

    #[test]
    fn validates_javascript_date_boundaries_and_calendar_days() {
        for date in [
            "2030-01-01T00:00:00Z",
            "+010000-01-01T00:00:00.000Z",
            "-000001-01-01T00:00:00.000Z",
            "+275760-09-13T00:00:00.000Z",
            "-271821-04-20T00:00:00.000Z",
        ] {
            assert!(validate_response_date(date).is_ok(), "{date}");
        }
        for date in [
            "not-a-date",
            "+010001-02-29T00:00:00.000Z",
            "+275760-09-13T00:00:00.001Z",
            "-000000-01-01T00:00:00.000Z",
            "+010000-13-01T00:00:00.000Z",
        ] {
            assert!(validate_response_date(date).is_err(), "{date}");
        }
    }
}
