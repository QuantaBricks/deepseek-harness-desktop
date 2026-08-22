; Keep the per-user install root short: the embedded Node dependency tree has
; file names that exceed MAX_PATH under Tauri's default product-name folder.
!macro NSIS_HOOK_PREINSTALL
  StrCpy $INSTDIR "$LOCALAPPDATA\D"
!macroend
