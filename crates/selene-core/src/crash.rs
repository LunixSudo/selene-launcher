//! Guesses why the game closed with an error from the last lines it printed.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CrashCause {
    OutOfMemory,
    SystemMemory,
    WrongJava,
    MissingDependency,
    /// A mod made for another game version.
    WrongGameVersion,
    IncompatibleMods,
    ModError,
    GraphicsDriver,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub cause: CrashCause,
    /// The line or two from the log that explain it, when there are any.
    pub details: Vec<String>,
    /// The mods the log blames, by the name or id it gives.
    pub mods: Vec<String>,
    /// Enabled mods made for another game version, which the launcher knows of itself.
    pub unfit: Vec<String>,
}

const MAX_DETAILS: usize = 6;

pub fn analyze<S: AsRef<str>>(log: &[S]) -> CrashReport {
    let lines: Vec<&str> = log.iter().map(AsRef::as_ref).collect();
    let find = |needles: &[&str]| {
        lines
            .iter()
            .rposition(|line| needles.iter().any(|needle| line.contains(needle)))
    };

    // Java could not get the memory it was told to take: the PC has less free than the
    // instance asks for, the opposite of running out inside the game.
    if let Some(index) = find(&[
        "Could not reserve enough space",
        "insufficient memory for the Java Runtime Environment",
        "Native memory allocation (malloc) failed",
    ]) {
        return report(CrashCause::SystemMemory, &lines[index..=index]);
    }
    if let Some(index) = find(&["java.lang.OutOfMemoryError"]) {
        return report(CrashCause::OutOfMemory, &lines[index..=index]);
    }
    if let Some(index) = find(&[
        "UnsupportedClassVersionError",
        "compiled by a more recent version of the Java Runtime",
    ]) {
        return report(CrashCause::WrongJava, &lines[index..=index]);
    }
    // Fabric and Quilt name the game "Minecraft", NeoForge and Forge by its id.
    let wrong_game: Vec<&str> = lines
        .iter()
        .filter(|line| {
            (line.contains("but only the wrong version is present")
                && (line.contains("'Minecraft'") || line.contains("of minecraft")))
                || line.contains("Mod ID: 'minecraft'")
        })
        .map(|line| line.trim())
        .collect();
    if !wrong_game.is_empty() {
        return report(CrashCause::WrongGameVersion, &wrong_game);
    }
    // Fabric and Quilt list each unmet requirement on its own line, NeoForge and Forge too.
    let missing: Vec<&str> = lines
        .iter()
        .filter(|line| {
            line.contains("which is missing!")
                || line.contains("requires any version")
                || line.contains("Actual version: '[MISSING]'")
        })
        .map(|line| line.trim())
        .collect();
    if !missing.is_empty() {
        return report(CrashCause::MissingDependency, &missing);
    }
    if let Some(index) = find(&[
        "Incompatible mods found!",
        "Incompatible mod set!",
        "incompatible mod set",
    ]) {
        let end = (index + 1 + MAX_DETAILS).min(lines.len());
        let details: Vec<&str> = lines[index + 1..end]
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();
        return report(CrashCause::IncompatibleMods, &details);
    }
    if let Some(index) = find(&[
        "Mixin apply failed",
        "Mixin apply for mod",
        "MixinApplyError",
        "InvalidInjectionException",
        "Could not execute entrypoint",
    ]) {
        return report(CrashCause::ModError, &lines[index..=index]);
    }
    if let Some(index) = find(&[
        "Pixel format not accelerated",
        "GLFW error 65542",
        "GLFW error 65543",
        "does not appear to support OpenGL",
        "EXCEPTION_ACCESS_VIOLATION",
    ]) {
        return report(CrashCause::GraphicsDriver, &lines[index..=index]);
    }
    report(CrashCause::Unknown, &[])
}

fn report(cause: CrashCause, lines: &[&str]) -> CrashReport {
    let lines = &lines[..lines.len().min(MAX_DETAILS)];
    let mut mods = Vec::new();
    for name in lines.iter().filter_map(|line| blamed_mod(line)) {
        if !mods.contains(&name) {
            mods.push(name);
        }
    }
    CrashReport {
        cause,
        details: lines.iter().map(|line| line.trim().to_owned()).collect(),
        mods,
        unfit: Vec::new(),
    }
}

/// The mod a log line is about, in the ways the loaders put it.
fn blamed_mod(line: &str) -> Option<String> {
    let quoted_after = |marker: &str| {
        let rest = &line[line.find(marker)? + marker.len()..];
        let (name, _) = rest.split_once('\'')?;
        Some(name.to_owned())
    };
    let word_after = |marker: &str| {
        let rest = &line[line.find(marker)? + marker.len()..];
        rest.split_whitespace().next().map(str::to_owned)
    };
    quoted_after("Mod '")
        .or_else(|| quoted_after("Requested by: '"))
        .or_else(|| quoted_after("Mod ID: '"))
        .or_else(|| quoted_after("provided by '"))
        .or_else(|| word_after("Mixin apply for mod "))
        .or_else(|| {
            let config = word_after("Mixin apply failed ")?;
            config
                .split_once(".mixins")
                .map(|(name, _)| name.to_owned())
        })
        .filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_out_of_memory() {
        let log = [
            "[main/INFO]: Loading",
            "java.lang.OutOfMemoryError: Java heap space",
        ];
        assert_eq!(analyze(&log).cause, CrashCause::OutOfMemory);
    }

    #[test]
    fn lists_missing_dependencies() {
        let log = [
            "Incompatible mods found!",
            "\t - Mod 'Sodium Extra' (sodium-extra) 0.6 requires any version of sodium, which is missing!",
        ];
        let report = analyze(&log);
        assert_eq!(report.cause, CrashCause::MissingDependency);
        assert_eq!(report.details.len(), 1);
        assert!(report.details[0].starts_with("- Mod 'Sodium Extra'"));
    }

    #[test]
    fn spots_newer_java_requirements() {
        let log = [
            "has been compiled by a more recent version of the Java Runtime (class file version 65.0)",
        ];
        assert_eq!(analyze(&log).cause, CrashCause::WrongJava);
    }

    #[test]
    fn names_a_mod_for_another_game_version() {
        let log = [
            "Incompatible mods found!",
            "\t - Mod 'Sodium' (sodium) 0.5.11+mc1.21 requires version 1.21 of 'Minecraft' (minecraft), but only the wrong version is present: 1.21.1!",
        ];
        let report = analyze(&log);
        assert_eq!(report.cause, CrashCause::WrongGameVersion);
        assert_eq!(report.mods, ["Sodium"]);

        let log = [
            "\tMod ID: 'minecraft', Requested by: 'sodium', Expected range: '[1.20.1]', Actual version: '1.21.1'",
        ];
        assert_eq!(analyze(&log).mods, ["sodium"]);
    }

    #[test]
    fn names_the_mod_a_mixin_broke_in() {
        let log =
            ["Mixin apply for mod sodium failed sodium.mixins.json:core.MinecraftClientMixin"];
        let report = analyze(&log);
        assert_eq!(report.cause, CrashCause::ModError);
        assert_eq!(report.mods, ["sodium"]);
        let log = [
            "org.spongepowered.asm.mixin.transformer.throwables.MixinTransformerError: Mixin apply failed iris.mixins.json:MixinGameRenderer",
        ];
        assert_eq!(analyze(&log).mods, ["iris"]);
    }

    #[test]
    fn falls_back_to_unknown() {
        assert_eq!(analyze(&["something odd"]).cause, CrashCause::Unknown);
    }

    #[test]
    fn tells_a_full_pc_from_a_full_game() {
        let report = analyze(&[
            "Error occurred during initialization of VM",
            "Could not reserve enough space for 8388608KB object heap",
        ]);
        assert_eq!(report.cause, CrashCause::SystemMemory);
    }
}
