//! Guesses why the game closed with an error from the last lines it printed.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CrashCause {
    OutOfMemory,
    WrongJava,
    MissingDependency,
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
}

const MAX_DETAILS: usize = 6;

pub fn analyze<S: AsRef<str>>(log: &[S]) -> CrashReport {
    let lines: Vec<&str> = log.iter().map(AsRef::as_ref).collect();
    let find = |needles: &[&str]| {
        lines
            .iter()
            .rposition(|line| needles.iter().any(|needle| line.contains(needle)))
    };

    if let Some(index) = find(&["java.lang.OutOfMemoryError"]) {
        return report(CrashCause::OutOfMemory, &lines[index..=index]);
    }
    if let Some(index) = find(&[
        "UnsupportedClassVersionError",
        "compiled by a more recent version of the Java Runtime",
    ]) {
        return report(CrashCause::WrongJava, &lines[index..=index]);
    }
    // Fabric and Quilt list each unmet requirement on its own line.
    let missing: Vec<&str> = lines
        .iter()
        .filter(|line| line.contains("which is missing!") || line.contains("requires any version"))
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
    CrashReport {
        cause,
        details: lines
            .iter()
            .take(MAX_DETAILS)
            .map(|line| line.trim().to_owned())
            .collect(),
    }
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
    fn falls_back_to_unknown() {
        assert_eq!(analyze(&["something odd"]).cause, CrashCause::Unknown);
    }
}
