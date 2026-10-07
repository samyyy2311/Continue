; The call camera's DLL. Cargo builds it as one of the app's dependencies, into
; target/<profile>/deps; makensis runs from target/<profile>/nsis/<arch>.
!macro NSIS_HOOK_POSTINSTALL
  SetOutPath "$INSTDIR"
  File "..\..\deps\continue_camera.dll"
!macroend

; Takes the camera off the system if it was ever turned on; registering it needed admin, so
; this does too.
!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$INSTDIR\continue_camera.dll"
  IfFileExists "$COMMONAPPDATA\Continue\continue_camera.dll" 0 +2
    ExecShellWait "runas" "cmd.exe" '/c regsvr32 /u /s "$COMMONAPPDATA\Continue\continue_camera.dll" & del "$COMMONAPPDATA\Continue\continue_camera.dll"' SW_HIDE
!macroend
