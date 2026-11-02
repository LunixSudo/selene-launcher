pub const MIN_MB: u64 = 1024;
const SYSTEM_RESERVE_MB: u64 = 2048;

/// Memory to give the game when the player has not picked an amount.
///
/// Newer vanilla versions want 3 GB; mods add up, so bigger packs get more. The game never
/// gets more than 60% of the computer's memory and always leaves 2 GB for the system, so the
/// rest of the PC does not start swapping.
pub fn recommended_mb(total_mb: u64, mod_count: usize) -> u32 {
    let wanted: u64 = match mod_count {
        0 => 3072,
        1..=50 => 5120,
        51..=150 => 8192,
        _ => 10_240,
    };
    let ceiling = (total_mb * 3 / 5)
        .min(total_mb.saturating_sub(SYSTEM_RESERVE_MB))
        .max(MIN_MB);
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
        assert_eq!(recommended_mb(32_768, 0), 3072);
        assert_eq!(recommended_mb(32_768, 20), 5120);
        assert_eq!(recommended_mb(32_768, 100), 8192);
        assert_eq!(recommended_mb(32_768, 300), 10_240);
    }

    #[test]
    fn leaves_the_system_enough() {
        assert_eq!(recommended_mb(16_384, 300), 9830);
        assert_eq!(recommended_mb(8192, 300), 4915);
        assert_eq!(recommended_mb(6144, 20), 3686);
        assert_eq!(recommended_mb(4096, 20), 2048);
        assert_eq!(recommended_mb(1024, 0), 1024);
    }

    #[test]
    fn leaves_room_for_the_system() {
        assert_eq!(max_mb(16_384), 14_336);
        assert_eq!(max_mb(2048), 1024);
    }
}
