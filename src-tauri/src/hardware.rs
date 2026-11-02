//! What the PC is made of, for the log: most game problems come down to the graphics driver,
//! too little memory or a full disk.
#![allow(unsafe_code)]

use std::fmt::Write as _;
use std::path::Path;

use sysinfo::{CpuRefreshKind, DiskKind, Disks, RefreshKind, System};

const BASEBOARD: u8 = 2;
const MEMORY_DEVICE: u8 = 17;
const END_OF_TABLE: u8 = 127;
/// What makers leave in the table instead of a name.
const PLACEHOLDERS: &[&str] = &[
    "",
    "unknown",
    "default string",
    "to be filled by o.e.m.",
    "not specified",
    "undefined",
    "none",
];

/// Physical cores, or logical ones when the system does not tell.
pub fn cores() -> usize {
    System::physical_core_count()
        .or_else(|| std::thread::available_parallelism().ok().map(usize::from))
        .unwrap_or(1)
}

/// Processor, cores, graphics cards, memory sticks and motherboard on one line. Serial numbers
/// stay out, since the log may end up in a public report.
pub fn summary() -> String {
    let system =
        System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing()));
    let processor = system
        .cpus()
        .first()
        .map_or("unknown processor", |cpu| cpu.brand().trim());
    let threads = system.cpus().len();
    let cores = System::physical_core_count().unwrap_or(threads);
    let graphics = graphics_cards();
    let graphics = if graphics.is_empty() {
        "no graphics card found".to_owned()
    } else {
        graphics.join(", ")
    };
    let mut line = format!("{processor} ({cores} cores, {threads} threads), {graphics}");
    let board = firmware_table().map(|table| describe_firmware(&table));
    if let Some(board) = board.filter(|board| !board.is_empty()) {
        line.push_str(", ");
        line.push_str(&board);
    }
    line
}

/// Free space and kind of the disk that holds `path`.
pub fn disk(path: &Path) -> String {
    let disks = Disks::new_with_refreshed_list();
    let Some(disk) = disks
        .list()
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
    else {
        return "unknown disk".to_owned();
    };
    let kind = match disk.kind() {
        DiskKind::SSD => "SSD",
        DiskKind::HDD => "HDD",
        DiskKind::Unknown(_) => "disk",
    };
    format!(
        "{} MB free of {} MB on {kind}",
        disk.available_space() / (1024 * 1024),
        disk.total_space() / (1024 * 1024)
    )
}

/// Reads the memory sticks and the motherboard out of the raw SMBIOS table.
fn describe_firmware(table: &[u8]) -> String {
    let mut sticks = Vec::new();
    let mut board = None;
    let mut rest = table;
    while rest.len() >= 4 {
        let (kind, length) = (rest[0], usize::from(rest[1]));
        if length < 4 || rest.len() < length {
            break;
        }
        let formatted = &rest[..length];
        let strings_end = rest[length..]
            .windows(2)
            .position(|pair| pair == [0, 0])
            .map_or(rest.len(), |end| length + end + 2);
        let strings: Vec<&str> = rest[length..strings_end]
            .split(|&byte| byte == 0)
            .filter_map(|text| std::str::from_utf8(text).ok())
            .collect();
        let text = |offset: usize| {
            let index = usize::from(*formatted.get(offset)?);
            let value = strings.get(index.checked_sub(1)?)?.trim();
            (!PLACEHOLDERS
                .iter()
                .any(|placeholder| value.eq_ignore_ascii_case(placeholder)))
            .then_some(value)
        };
        match kind {
            BASEBOARD => {
                board = Some(
                    [text(4), text(5)]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
            MEMORY_DEVICE => {
                if let Some(stick) = memory_stick(formatted, text(0x17), text(0x1A)) {
                    sticks.push(stick);
                }
            }
            END_OF_TABLE => break,
            _ => {}
        }
        rest = &rest[strings_end..];
    }
    let mut parts = Vec::new();
    if !sticks.is_empty() {
        parts.push(format!("memory {}", sticks.join(" + ")));
    }
    if let Some(board) = board.filter(|board| !board.is_empty()) {
        parts.push(format!("board {board}"));
    }
    parts.join(", ")
}

fn memory_stick(formatted: &[u8], maker: Option<&str>, part: Option<&str>) -> Option<String> {
    let word = |offset: usize| {
        let bytes = formatted.get(offset..offset + 2)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    };
    let size_mb = match word(0x0C)? {
        0 | 0xFFFF => return None,
        0x7FFF => {
            let bytes = formatted.get(0x1C..0x20)?;
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) & 0x7FFF_FFFF
        }
        size if size & 0x8000 != 0 => u32::from(size & 0x7FFF) / 1024,
        size => u32::from(size),
    };
    let kind = match formatted.get(0x12) {
        Some(0x18) => "DDR3",
        Some(0x1A) => "DDR4",
        Some(0x1D) => "LPDDR3",
        Some(0x1E) => "LPDDR4",
        Some(0x22) => "DDR5",
        Some(0x23) => "LPDDR5",
        _ => "RAM",
    };
    let speed = word(0x20)
        .filter(|&speed| speed != 0 && speed != 0xFFFF)
        .or_else(|| word(0x15).filter(|&speed| speed != 0 && speed != 0xFFFF));
    let mut stick = if size_mb >= 1024 && size_mb % 1024 == 0 {
        format!("{} GB {kind}", size_mb / 1024)
    } else {
        format!("{size_mb} MB {kind}")
    };
    if let Some(speed) = speed {
        let _ = write!(stick, " {speed} MT/s");
    }
    for name in [maker, part].into_iter().flatten() {
        stick.push(' ');
        stick.push_str(name);
    }
    Some(stick)
}

/// The SMBIOS table without the header Windows puts in front of it.
#[cfg(windows)]
fn firmware_table() -> Option<Vec<u8>> {
    use windows_sys::Win32::System::SystemInformation::GetSystemFirmwareTable;

    const RSMB: u32 = u32::from_be_bytes(*b"RSMB");
    const HEADER: usize = 8;

    // SAFETY: a null buffer of size 0 only asks for the size.
    let size = unsafe { GetSystemFirmwareTable(RSMB, 0, std::ptr::null_mut(), 0) };
    if size == 0 {
        return None;
    }
    let mut buffer = vec![0u8; usize::try_from(size).ok()?];
    // SAFETY: `buffer` holds exactly `size` bytes.
    let written = unsafe { GetSystemFirmwareTable(RSMB, 0, buffer.as_mut_ptr(), size) };
    if written == 0 || written > size {
        return None;
    }
    buffer.truncate(usize::try_from(written).ok()?);
    buffer.get(HEADER..).map(<[u8]>::to_vec)
}

/// Only root may read the table on Linux, so the motherboard comes from what anyone may read
/// and the memory sticks stay unknown.
#[cfg(not(windows))]
fn firmware_table() -> Option<Vec<u8>> {
    let read = |name: &str| {
        std::fs::read_to_string(format!("/sys/class/dmi/id/{name}"))
            .ok()
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
    };
    let (vendor, name) = (read("board_vendor")?, read("board_name")?);
    let mut table = vec![BASEBOARD, 8, 0, 0, 1, 2, 0, 0];
    for text in [vendor, name] {
        table.extend(text.bytes());
        table.push(0);
    }
    table.push(0);
    Some(table)
}

/// Every display adapter Windows has a driver entry for, with the driver version.
#[cfg(windows)]
fn graphics_cards() -> Vec<String> {
    const DISPLAY_CLASS: &str =
        "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}";
    const MAX_ADAPTERS: u32 = 16;

    (0..MAX_ADAPTERS)
        .filter_map(|index| {
            let key = format!("{DISPLAY_CLASS}\\{index:04}");
            let name = registry_text(&key, "DriverDesc")?;
            Some(match registry_text(&key, "DriverVersion") {
                Some(driver) => format!("{name} (driver {driver})"),
                None => name,
            })
        })
        .collect()
}

#[cfg(windows)]
fn registry_text(key: &str, value: &str) -> Option<String> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};

    let key: Vec<u16> = key.encode_utf16().chain(std::iter::once(0)).collect();
    let value: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let mut buffer = [0u16; 256];
    let mut bytes = u32::try_from(size_of_val(&buffer)).unwrap_or(u32::MAX);
    // SAFETY: the names are NUL-terminated and outlive the call, and `bytes` is the size of
    // `buffer`, which the call fills with a NUL-terminated string.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &raw mut bytes,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let end = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    let text = String::from_utf16_lossy(&buffer[..end]);
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// The kernel names the driver of each card, which tells the vendor and whether the
/// proprietary one is in use.
#[cfg(not(windows))]
fn graphics_cards() -> Vec<String> {
    let Ok(cards) = std::fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };
    let mut drivers: Vec<String> = cards
        .flatten()
        .filter(|card| {
            let name = card.file_name();
            let name = name.to_string_lossy();
            name.starts_with("card") && !name.contains('-')
        })
        .filter_map(|card| std::fs::read_link(card.path().join("device/driver")).ok())
        .filter_map(|driver| Some(format!("{} driver", driver.file_name()?.to_string_lossy())))
        .collect();
    drivers.sort();
    drivers.dedup();
    drivers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_memory_and_board() {
        let mut board = vec![BASEBOARD, 8, 0, 0, 1, 2, 3, 0];
        board.extend(b"ASUSTeK\0PRIME B450M-A\0SERIAL123\0\0");
        let mut stick = vec![0u8; 0x22];
        stick[0] = MEMORY_DEVICE;
        stick[1] = 0x22;
        stick[0x0C..0x0E].copy_from_slice(&8192u16.to_le_bytes());
        stick[0x12] = 0x1A;
        stick[0x15..0x17].copy_from_slice(&3200u16.to_le_bytes());
        stick[0x17] = 1;
        stick[0x18] = 2;
        stick[0x1A] = 3;
        stick[0x20..0x22].copy_from_slice(&2666u16.to_le_bytes());
        stick.extend(b"Kingston\0SN-SECRET\0KF3200C16D4/8GX\0\0");
        let mut empty = vec![0u8; 0x22];
        empty[0] = MEMORY_DEVICE;
        empty[1] = 0x22;
        empty.extend([0, 0]);
        let table = [
            board,
            stick.clone(),
            stick,
            empty,
            vec![END_OF_TABLE, 4, 0, 0, 0, 0],
        ]
        .concat();
        assert_eq!(
            describe_firmware(&table),
            "memory 8 GB DDR4 2666 MT/s Kingston KF3200C16D4/8GX + 8 GB DDR4 2666 MT/s Kingston \
             KF3200C16D4/8GX, board ASUSTeK PRIME B450M-A"
        );
    }

    #[test]
    fn survives_a_broken_table() {
        assert_eq!(describe_firmware(&[MEMORY_DEVICE, 0x40, 0, 0]), "");
        assert_eq!(describe_firmware(&[]), "");
    }
}
