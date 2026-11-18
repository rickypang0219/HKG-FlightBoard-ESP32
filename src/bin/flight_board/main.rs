#![no_std]
#![no_main]

mod screen;

use core::{panic::PanicInfo, str};

use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    spi::master::{Config as SpiConfig, Spi},
    time::{Instant, Rate},
    uart::{Config as UartConfig, Uart},
};
use mipidsi::{
    Builder,
    interface::SpiInterface,
    options::{ColorOrder, Orientation, Rotation},
};
use screen::{Flight, Language, PageInfo, ROWS};

const PACKET_BYTES: usize = 768;
const LANGUAGE_SECONDS: u64 = 5;

enum PagePhase {
    Waiting,
    English(Instant),
    Chinese(Instant),
    Finished,
    Stale,
}

esp_bootloader_esp_idf::esp_app_desc!();

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(feature = "st7789"))]
const PANEL: mipidsi::models::ILI9341Rgb565 = mipidsi::models::ILI9341Rgb565;
#[cfg(feature = "st7789")]
const PANEL: mipidsi::models::ST7789 = mipidsi::models::ST7789;

#[esp_hal::main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());
    let mut delay = Delay::new();
    let mut backlight = Output::new(p.GPIO21, Level::Low, OutputConfig::default());
    let cs = Output::new(p.GPIO15, Level::High, OutputConfig::default());
    let dc = Output::new(p.GPIO2, Level::Low, OutputConfig::default());
    let spi = Spi::new(
        p.SPI2,
        SpiConfig::default().with_frequency(Rate::from_mhz(20)),
    )
    .unwrap()
    .with_sck(p.GPIO14)
    .with_mosi(p.GPIO13);
    let spi = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let mut spi_buffer = [0; 1024];
    let interface = SpiInterface::new(spi, dc, &mut spi_buffer);
    let builder = Builder::new(PANEL, interface)
        .orientation(Orientation::new().rotate(Rotation::Deg90).flip_horizontal())
        .color_order(ColorOrder::Bgr);
    #[cfg(feature = "st7789")]
    let builder = builder.invert_colors(mipidsi::options::ColorInversion::Inverted);
    let mut display = builder.init(&mut delay).unwrap();

    let mut uart = Uart::new(p.UART0, UartConfig::default().with_baudrate(115_200))
        .unwrap()
        .with_tx(p.GPIO1)
        .with_rx(p.GPIO3);

    screen::draw(&mut display, &[], None, Language::English, false).unwrap();
    backlight.set_high();
    let _ = uart.write(b"HKG READY\n");

    let mut line = [0u8; PACKET_BYTES];
    let mut current = heapless::String::<PACKET_BYTES>::new();
    let mut length = 0;
    let mut bytes = [0u8; 64];
    let mut phase = PagePhase::Waiting;
    let mut last_update = Instant::now();
    let mut last_byte = Instant::now();

    loop {
        let mut received = false;
        if uart.read_ready() {
            last_byte = Instant::now();
            match uart.read(&mut bytes) {
                Ok(count) => {
                    for &byte in &bytes[..count] {
                        if byte == b'\n' {
                            if let Some(data) =
                                line.get(..length).and_then(|b| str::from_utf8(b).ok())
                                && parse_update(data).is_some()
                            {
                                current.clear();
                                current.push_str(data).unwrap();
                                last_update = Instant::now();
                                received = true;
                            }
                            length = 0;
                        } else if length <= line.len() {
                            if let Some(slot) = line.get_mut(length) {
                                *slot = byte;
                            }
                            length += 1;
                        }
                    }
                }
                Err(_) => length = line.len() + 1,
            }
        } else {
            if length != 0 && last_byte.elapsed().as_millis() >= 100 {
                length = 0;
            }
            delay.delay_millis(1);
        }

        if length != 0 && !received {
            continue;
        }
        let (language, stale, message): (_, _, &[u8]) = match phase {
            _ if received => (Language::English, false, b"HKG VIEW en\n"),
            PagePhase::English(since) if since.elapsed().as_secs() >= LANGUAGE_SECONDS => {
                (Language::Chinese, false, b"HKG VIEW zh-HK\n")
            }
            PagePhase::Chinese(since) if since.elapsed().as_secs() >= LANGUAGE_SECONDS => {
                phase = PagePhase::Finished;
                let _ = uart.write(b"HKG DONE\n");
                continue;
            }
            PagePhase::Finished if last_update.elapsed().as_secs() >= 180 => {
                (Language::Chinese, true, b"")
            }
            _ => continue,
        };

        let (page, flights) = parse_update(&current).unwrap();
        screen::draw(
            &mut display,
            flights.as_slice(),
            Some(page),
            language,
            stale,
        )
        .unwrap();
        phase = match (language, stale) {
            (_, true) => PagePhase::Stale,
            (Language::English, false) => PagePhase::English(Instant::now()),
            (Language::Chinese, false) => PagePhase::Chinese(Instant::now()),
        };
        if !message.is_empty() {
            let _ = uart.write(message);
        }
    }
}

fn parse_update(line: &str) -> Option<(PageInfo<'_>, heapless::Vec<Flight<'_>, ROWS>)> {
    let mut parts = line.split('|');
    if parts.next()? != "HKG3" {
        return None;
    }
    let time = parts.next()?;
    if !time.get(..5).is_some_and(valid_time)
        || !matches!(
            time.as_bytes().get(5..),
            Some([b':', b'0'..=b'5', b'0'..=b'9'])
        )
    {
        return None;
    }
    let number = parts.next()?;
    let (index, count) = number.split_once('/')?;
    let index = index.parse::<u16>().ok()?;
    let count = count.parse::<u16>().ok()?;
    if number.len() > 11 || index == 0 || index > count {
        return None;
    }

    let mut flights = heapless::Vec::new();
    for row in parts {
        let mut fields = row.split('\t');
        let flight = Flight {
            time: fields.next()?,
            number: fields.next()?,
            destination: fields.next()?,
            gate: fields.next()?,
            status: fields.next()?,
            destination_zh: fields.next()?,
            status_zh: fields.next()?,
        };
        if fields.next().is_some()
            || !valid_time(flight.time)
            || ![
                (flight.number, 9),
                (flight.destination, 13),
                (flight.gate, 4),
                (flight.status, 14),
            ]
            .into_iter()
            .all(|(field, max)| {
                !field.is_empty()
                    && field.len() <= max
                    && field.bytes().all(|c| (b' '..=b'~').contains(&c))
            })
            || ![(flight.destination_zh, 80), (flight.status_zh, 84)]
                .into_iter()
                .all(|(field, max)| {
                    !field.is_empty()
                        && screen::text_width(field) <= max
                        && field
                            .chars()
                            .all(|ch| matches!(ch, ' '..='~' | '\u{3400}'..='\u{9fff}'))
                })
        {
            return None;
        }
        flights.push(flight).ok()?;
    }
    Some((PageInfo { time, number }, flights))
}

fn valid_time(text: &str) -> bool {
    let [h1, h2, b':', m1, m2] = text.as_bytes() else {
        return false;
    };
    [h1, h2, m1, m2].into_iter().all(u8::is_ascii_digit)
        && (h1 - b'0') * 10 + (h2 - b'0') < 24
        && *m1 < b'6'
}
