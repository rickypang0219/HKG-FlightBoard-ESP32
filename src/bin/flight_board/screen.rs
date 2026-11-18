use core::fmt::Write;

use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyleBuilder, ascii::FONT_6X10, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
    text::{Baseline, Text, TextStyleBuilder},
};

pub const ROWS: usize = 8;
const HAN_16: &[u8] = include_bytes!("fonts/han16.bin");

const BLACK: Rgb565 = rgb(0x101016);
const BLUE: Rgb565 = rgb(0x171B73);
const AMBER: Rgb565 = rgb(0xFFC574);
const WHITE: Rgb565 = rgb(0xFAF5E9);
const YELLOW: Rgb565 = rgb(0xFFF1A0);
const MUTED: Rgb565 = rgb(0xB5B4C5);
const RED: Rgb565 = rgb(0xFF795F);

pub struct Flight<'a> {
    pub time: &'a str,
    pub number: &'a str,
    pub destination: &'a str,
    pub gate: &'a str,
    pub status: &'a str,
    pub destination_zh: &'a str,
    pub status_zh: &'a str,
}

#[derive(Clone, Copy)]
pub struct PageInfo<'a> {
    pub time: &'a str,
    pub number: &'a str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Chinese,
}

impl Language {
    fn text<'a>(self, english: &'a str, chinese: &'a str) -> &'a str {
        match self {
            Self::English => english,
            Self::Chinese => chinese,
        }
    }
}

pub fn draw<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    flights: &[Flight<'_>],
    page: Option<PageInfo<'_>>,
    language: Language,
    stale: bool,
) -> Result<(), D::Error> {
    fill(d, 0, 0, 320, 55, BLACK)?;
    label(d, "HKG", 10, 8, &FONT_10X20, AMBER)?;
    match language {
        Language::English => label(d, "Departures", 210, 8, &FONT_10X20, WHITE)?,
        Language::Chinese => text(d, "離港航班", 246, 10, WHITE)?,
    }
    fill(d, 10, 33, 300, 1, AMBER)?;
    for (english, chinese, x) in [
        ("Time", "時間", 10),
        ("Flight", "航班", 49),
        ("Destination", "目的地", 108),
        ("Status", "狀態", 230),
    ] {
        text(d, language.text(english, chinese), x, 38, AMBER)?;
    }
    centered(d, language.text("Gate", "閘口"), 206, 38, AMBER)?;

    for index in 0..ROWS {
        let top = 55 + 20 * index as i32;
        let background = if index % 2 == 0 { BLUE } else { BLACK };
        fill(d, 0, top, 320, 20, background)?;
        if let Some(flight) = flights.get(index) {
            let y = top + 2;
            text(d, flight.time, 10, y, WHITE)?;
            text(d, flight.number, 49, y, YELLOW)?;
            text(
                d,
                language.text(flight.destination, flight.destination_zh),
                108,
                y,
                WHITE,
            )?;
            if flight.gate != "-" {
                fill(d, 191, top + 3, 30, 14, AMBER)?;
                centered(d, flight.gate, 206, y, BLACK)?;
            } else {
                centered(d, "-", 206, y, MUTED)?;
            }
            let status_color = match flight.status {
                "Cancelled" | "Delayed" | "Gate closed" => RED,
                "Boarding" | "Boarding soon" | "Final call" | "Gate open" => YELLOW,
                "-" => MUTED,
                _ => WHITE,
            };
            text(
                d,
                language.text(flight.status, flight.status_zh),
                230,
                y,
                status_color,
            )?;
        }
    }

    if flights.is_empty() {
        fill(d, 16, 111, 288, 26, BLACK)?;
        let message = if page.is_some() {
            language.text("No matching departures", "暫無航班資料")
        } else {
            language.text("Waiting for USB flight data", "等候 USB 航班資料")
        };
        centered(d, message, 160, 116, WHITE)?;
    }
    footer(d, page, language, stale)
}

fn footer<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    page: Option<PageInfo<'_>>,
    language: Language,
    stale: bool,
) -> Result<(), D::Error> {
    fill(d, 0, 215, 320, 25, BLACK)?;
    fill(d, 10, 217, 300, 1, AMBER)?;
    let mut time = heapless::String::<24>::new();
    let stamp = page.map_or("--:--:--", |page| page.time);
    let prefix = language.text("Data", "更新");
    let _ = write!(time, "{prefix} {stamp} HKT");
    text(d, &time, 10, 222, MUTED)?;
    if let Some(page) = page {
        centered(d, page.number, 160, 222, AMBER)?;
    }
    let (source, color) = if stale {
        (language.text("Feed paused", "更新暫停"), AMBER)
    } else {
        (language.text("AAHK via USB", "機管局 USB"), MUTED)
    };
    text(d, source, 310 - text_width(source), 222, color)
}

fn fill<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    color: Rgb565,
) -> Result<(), D::Error> {
    d.fill_solid(
        &Rectangle::new(Point::new(x, y), Size::new(width, height)),
        color,
    )
}

fn label<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    text: &str,
    x: i32,
    y: i32,
    font: &MonoFont<'_>,
    color: Rgb565,
) -> Result<(), D::Error> {
    let character = MonoTextStyleBuilder::new()
        .font(font)
        .text_color(color)
        .build();
    let style = TextStyleBuilder::new().baseline(Baseline::Top).build();
    Text::with_text_style(text, Point::new(x, y), character, style).draw(d)?;
    Ok(())
}

fn centered<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    text: &str,
    x: i32,
    y: i32,
    color: Rgb565,
) -> Result<(), D::Error> {
    self::text(d, text, x - text_width(text) / 2, y, color)
}

pub fn text_width(text: &str) -> i32 {
    text.chars()
        .map(|ch| if ch.is_ascii() { 6 } else { 16 })
        .sum()
}

fn text<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    text: &str,
    mut x: i32,
    y: i32,
    color: Rgb565,
) -> Result<(), D::Error> {
    if text.is_ascii() {
        return label(d, text, x, y + 3, &FONT_6X10, color);
    }

    for ch in text.chars() {
        if ch.is_ascii() {
            label(d, ch.encode_utf8(&mut [0; 4]), x, y + 3, &FONT_6X10, color)?;
            x += 6;
        } else {
            if let Some(bitmap) = glyph(ch) {
                d.draw_iter(bitmap.chunks_exact(2).enumerate().flat_map(|(row, bytes)| {
                    let bits = u16::from_be_bytes([bytes[0], bytes[1]]);
                    (0..16).filter_map(move |col| {
                        (bits & (0x8000 >> col) != 0)
                            .then_some(Pixel(Point::new(x + col, y + row as i32), color))
                    })
                }))?;
            } else {
                label(d, "?", x + 5, y + 3, &FONT_6X10, color)?;
            }
            x += 16;
        }
    }
    Ok(())
}

fn glyph(ch: char) -> Option<&'static [u8]> {
    let code = u16::try_from(u32::from(ch)).ok()?;
    let (records, _) = HAN_16.as_chunks::<34>();
    let index = records
        .binary_search_by_key(&code, |g| u16::from_be_bytes([g[0], g[1]]))
        .ok()?;
    Some(&records[index][2..])
}

const fn rgb(hex: u32) -> Rgb565 {
    Rgb565::new(
        (hex >> 19) as u8,
        ((hex >> 10) & 63) as u8,
        ((hex >> 3) & 31) as u8,
    )
}
