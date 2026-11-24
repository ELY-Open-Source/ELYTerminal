; ELYTerminal for one user: its files, a Start menu entry and an uninstaller.
Unicode true
ManifestDPIAware true
!include "MUI2.nsh"
!include "WinMessages.nsh"

!ifndef VERSION
  !error "pass /DVERSION"
!endif
!ifndef STAGE
  !error "pass /DSTAGE, the folder holding elyterminal.exe, conpty.dll, OpenConsole.exe, OpenConsoleProxy.dll and LICENSE-conpty"
!endif
!ifndef OUTFILE
  !error "pass /DOUTFILE"
!endif
!ifndef ICON
  !error "pass /DICON"
!endif
!define UNINSTALL "Software\Microsoft\Windows\CurrentVersion\Uninstall\ELYTerminal"
!define CLASSES "Software\Classes\Directory"
!define TERMINAL "{79B54338-A0FA-474F-B304-4932B36B6F55}"

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
  File "${STAGE}\OpenConsoleProxy.dll"
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
  !insertmacro ElyGone "OpenConsoleProxy.dll"
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
  ; Explorer menu's PATH entry goes, the folder passed by the environment so no quote reaches PowerShell's parser.
  System::Call 'kernel32::SetEnvironmentVariable(t "ELY_FOLDER", t "$INSTDIR")i.r0'
  StrCmp $0 0 0 +3
    MessageBox MB_ICONSTOP "$(Unremoved) PATH" /SD IDOK
    Abort
  nsExec::ExecToLog `powershell -NoProfile -ExecutionPolicy Bypass -Command "$$d = $$env:ELY_FOLDER.TrimEnd('\'); $$k = Get-Item 'HKCU:\Environment'; $$p = $$k.GetValue('Path', '', 'DoNotExpandEnvironmentNames'); if ($$p) { $$n = ($$p -split ';' | Where-Object { [Environment]::ExpandEnvironmentVariables($$_).Trim().TrimEnd('\') -ne $$d }) -join ';'; if ($$n -ne $$p) { Set-ItemProperty 'HKCU:\Environment' -Name Path -Value $$n -Type ($$k.GetValueKind('Path')) } }; exit [int]!$$?"`
  Pop $0
  StrCmp $0 0 +3
    MessageBox MB_ICONSTOP "$(Unremoved) PATH" /SD IDOK
    Abort
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
  ; The default terminal goes when it is this copy, as Settings' Restore takes it.
  ; A 32-bit installer sees COM's classes through WOW64 unless told.
  SetRegView 64
  ReadRegStr $0 HKCU "Software\Classes\CLSID\${TERMINAL}\LocalServer32" ""
  StrCmp $0 '"$INSTDIR\elyterminal.exe"' 0 ElyKept
    ReadRegStr $1 HKCU "Console\%%Startup" "DelegationTerminal"
    StrCmp $1 "${TERMINAL}" 0 +3
      DeleteRegValue HKCU "Console\%%Startup" "DelegationConsole"
      DeleteRegValue HKCU "Console\%%Startup" "DelegationTerminal"
    DeleteRegKey HKCU "Software\Classes\CLSID\${TERMINAL}"
    DeleteRegKey HKCU "Software\Classes\CLSID\{2EACA947-7F5F-4CFA-BA87-8F7FBEEFBE69}"
    DeleteRegKey HKCU "Software\Classes\CLSID\{3171DE52-6EFA-4AEF-8A9F-D02BD67E7A4F}"
    DeleteRegKey HKCU "Software\Classes\Interface\{E686C757-9A35-4A1C-B3CE-0BCC8B5C69F4}"
    DeleteRegKey HKCU "Software\Classes\Interface\{6F23DA90-15C5-4203-9DB0-64E73F1B1B00}"
    DeleteRegKey HKCU "Software\Classes\Interface\{746E6BC0-AB05-4E38-AB14-71E86763141F}"
  ElyKept:
  SetRegView default
SectionEnd
