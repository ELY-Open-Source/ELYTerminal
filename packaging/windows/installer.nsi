; ELYTerminal for one user: its files, a Start menu entry and an uninstaller.
Unicode true
ManifestDPIAware true
!include "MUI2.nsh"

!ifndef VERSION
  !error "pass /DVERSION"
!endif
!ifndef STAGE
  !error "pass /DSTAGE, the folder holding elyterminal.exe, conpty.dll, OpenConsole.exe and LICENSE-conpty"
!endif
!ifndef OUTFILE
  !error "pass /DOUTFILE"
!endif
!ifndef ICON
  !error "pass /DICON"
!endif
!define UNINSTALL "Software\Microsoft\Windows\CurrentVersion\Uninstall\ELYTerminal"
!define CLASSES "Software\Classes\Directory"

!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
Name "ELYTerminal"
OutFile "${OUTFILE}"
RequestExecutionLevel user
InstallDir "$LOCALAPPDATA\Programs\ELYTerminal"
InstallDirRegKey HKCU "${UNINSTALL}" "InstallLocation"
SetCompressor /SOLID lzma
AllowSkipFiles off

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

LangString Busy ${LANG_ENGLISH} "ELYTerminal is running, so its files cannot be replaced. Close it and try again."
LangString Busy ${LANG_SIMPCHINESE} "ELYTerminal 正在运行，无法替换它的文件。请关闭后重试。"
LangString Unwritten ${LANG_ENGLISH} "Setup stopped: it could not write"
LangString Unwritten ${LANG_SIMPCHINESE} "安装已停止：无法写入"
LangString Unremoved ${LANG_ENGLISH} "Uninstall stopped, and can be run again: it could not remove"
LangString Unremoved ${LANG_SIMPCHINESE} "卸载已停止，可再次运行：无法删除"

; Stops, naming WHAT, when the step before it failed.
!macro ElyDone WHY WHAT
  IfErrors 0 +3
    MessageBox MB_ICONSTOP "$(${WHY}) ${WHAT}" /SD IDOK
    Abort
!macroend

; Deletes one of the program's files, or stops with the uninstaller kept.
!macro ElyGone FILE
  ClearErrors
  Delete "$INSTDIR\${FILE}"
  !insertmacro ElyDone Unremoved "$INSTDIR\${FILE}"
!macroend

; Stops while a running copy holds the program's file.
!macro ElyFree UN
Function ${UN}ElyFree
  IfFileExists "$INSTDIR\elyterminal.exe" 0 free
  ClearErrors
  FileOpen $0 "$INSTDIR\elyterminal.exe" a
  IfErrors 0 +3
    MessageBox MB_ICONSTOP "$(Busy)" /SD IDOK
    Abort
  FileClose $0
free:
FunctionEnd
!macroend
!insertmacro ElyFree ""
!insertmacro ElyFree "un."

; Removes an Explorer entry whose command is this copy's.
!macro ElyMenu KEY
  ReadRegStr $0 HKCU "${KEY}\command" ""
  StrCmp $0 '"$INSTDIR\elyterminal.exe" "%V"' 0 +2
    DeleteRegKey HKCU "${KEY}"
!macroend

Section "Install"
  Call ElyFree
  SetOutPath "$INSTDIR"
  File "${STAGE}\elyterminal.exe"
  File "${STAGE}\conpty.dll"
  File "${STAGE}\OpenConsole.exe"
  File "${STAGE}\LICENSE-conpty"
  ClearErrors
  WriteUninstaller "$INSTDIR\uninstall.exe"
  !insertmacro ElyDone Unwritten "$INSTDIR\uninstall.exe"
  CreateShortcut "$SMPROGRAMS\ELYTerminal.lnk" "$INSTDIR\elyterminal.exe"
  !insertmacro ElyDone Unwritten "$SMPROGRAMS\ELYTerminal.lnk"
  WriteRegStr HKCU "${UNINSTALL}" "DisplayName" "ELYTerminal"
  WriteRegStr HKCU "${UNINSTALL}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL}" "Publisher" "ELY Open Source"
  WriteRegStr HKCU "${UNINSTALL}" "URLInfoAbout" "https://terminal.elygpui.com"
  WriteRegStr HKCU "${UNINSTALL}" "DisplayIcon" "$INSTDIR\elyterminal.exe"
  WriteRegStr HKCU "${UNINSTALL}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINSTALL}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL}" "NoRepair" 1
  !insertmacro ElyDone Unwritten "HKCU\${UNINSTALL}"
SectionEnd

Section "Uninstall"
  Call un.ElyFree
  !insertmacro ElyGone "elyterminal.exe"
  !insertmacro ElyGone "conpty.dll"
  !insertmacro ElyGone "OpenConsole.exe"
  !insertmacro ElyGone "LICENSE-conpty"
  ; One install writes the shortcut and the key, so the key says whose both are.
  ReadRegStr $0 HKCU "${UNINSTALL}" "InstallLocation"
  StrCmp $0 "$INSTDIR" 0 +3
    Delete "$SMPROGRAMS\ELYTerminal.lnk"
    DeleteRegKey HKCU "${UNINSTALL}"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  ; Explorer's entries go when they name this copy; the settings stay.
  !insertmacro ElyMenu "${CLASSES}\shell\ELYTerminal"
  !insertmacro ElyMenu "${CLASSES}\Background\shell\ELYTerminal"
SectionEnd
