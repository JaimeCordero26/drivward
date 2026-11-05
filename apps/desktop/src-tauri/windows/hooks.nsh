; Instala el servicio de Windows y abre la bandeja al iniciar sesión (DESK-01, DESK-02).

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog '"$INSTDIR\drivward-service.exe" install'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Run" "Drivward" '"$INSTDIR\drivward-desktop.exe" --bandeja'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog '"$INSTDIR\drivward-service.exe" uninstall'
  DeleteRegValue HKLM "Software\Microsoft\Windows\CurrentVersion\Run" "Drivward"
!macroend
