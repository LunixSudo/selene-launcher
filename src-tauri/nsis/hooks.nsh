; Tauri copies the language files before building, so they cannot find the privacy
; policy texts on their own. This file is included from where it lies.
!define PRIVACY_DIR "${__FILEDIR__}\privacy"

; Must match PRIVACY_VERSION in src/commands/privacy.rs.
!define PRIVACY_VERSION "2026-10-28.5"

!macro NSIS_HOOK_POSTINSTALL
  ; What is left of an install in the old folder, which the new one has replaced.
  ${If} "$INSTDIR" != "${OLD_INSTALL_DIR}"
    Delete "${OLD_INSTALL_DIR}\${MAINBINARYNAME}.exe"
    Delete "${OLD_INSTALL_DIR}\uninstall.exe"
    RMDir "${OLD_INSTALL_DIR}"
  ${EndIf}

  ; The user agreed to the policy on the license page, so the launcher does not ask again.
  ; Updates run with /P and skip that page; then the launcher asks itself if the policy changed.
  ${If} $PassiveMode != 1
  ${AndIfNot} ${Silent}
    CreateDirectory "$APPDATA\SeleneLauncher"
    FileOpen $0 "$APPDATA\SeleneLauncher\privacy-accepted.txt" w
    FileWrite $0 "${PRIVACY_VERSION}"
    FileClose $0
  ${EndIf}
!macroend
