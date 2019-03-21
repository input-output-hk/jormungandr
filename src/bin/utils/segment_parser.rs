use std::fmt::Display;
use std::str::{FromStr, Split};

pub struct SegmentParser<'a> {
    segments: Split<'a, char>,
}

impl<'a> SegmentParser<'a> {
    pub fn new(parsed: &'a str) -> Self {
        SegmentParser {
            segments: parsed.split(':'),
        }
    }

    pub fn get_next(&mut self) -> Result<&'a str, String> {
        self.segments
            .next()
            .ok_or("too few argument segments".to_string())
    }

    pub fn parse_next<T: FromStr<Err = E>, E: Display>(&mut self) -> Result<T, String> {
        self.get_next()?
            .parse()
            .map_err(|e| format!("failed to parse argument segment: {}", e))
    }

    pub fn finish(mut self) -> Result<(), String> {
        match self.segments.next() {
            Some(_) => Err("too many argument segments")?,
            None => Ok(()),
        }
    }
}
