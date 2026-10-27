pub const MIN_MB: u64 = 1024;
const SYSTEM_RESERVE_MB: u64 = 2048;

/// Memory to give the game when the player has not picked an amount.
///
/// Vanilla is happy with 2 GB; mods add up, so bigger packs get more. The game never gets
/// more than half of the computer's memory, which leaves room for the system and the
/// garbage collector and is what avoids most out-of-memory crashes on 8 GB machines.
pub fn recommended_mb(total_mb: u64, mod_count: usize) -> u32 {
    let wanted: u64 = match mod_count {
        0 => 2048,
        1..=50 => 4096,
        51..=150 => 6144,
        _ => 8192,
    };
    let ceiling = (total_mb / 2).max(MIN_MB);
    u32::try_from(wanted.min(ceiling).max(MIN_MB)).unwrap_or(u32::MAX)
}

/// The most a player can pick: everything except what the system needs to stay usable.
pub fn max_mb(total_mb: u64) -> u32 {
    u32::try_from(total_mb.saturating_sub(SYSTEM_RESERVE_MB).max(MIN_MB)).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_with_mods() {
        assert_eq!(recommended_mb(32_768, 0), 2048);
        assert_eq!(recommended_mb(32_768, 20), 4096);
        assert_eq!(recommended_mb(32_768, 300), 8192);
    }

    #[test]
    fn never_takes_more_than_half() {
        assert_eq!(recommended_mb(8192, 300), 4096);
        assert_eq!(recommended_mb(4096, 20), 2048);
        assert_eq!(recommended_mb(1024, 0), 1024);
    }

    #[test]
    fn leaves_room_for_the_system() {
        assert_eq!(max_mb(16_384), 14_336);
        assert_eq!(max_mb(2048), 1024);
    }
}
