; PaperGrain — NSIS 3 installer script
; Build (Linux cross): makensis setup.nsi  (run from this directory)
Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma

!define APPNAME "PaperGrain"
!define VERSION "1.0.0"
!define PUBLISHER "CookieFilled"

Name "${APPNAME}"
OutFile "..\dist\PaperGrain-Setup-${VERSION}-x64.exe"
InstallDir "$PROGRAMFILES64\${APPNAME}"
InstallDirRegKey HKLM "Software\${APPNAME}" "InstallDir"
RequestExecutionLevel admin
ShowInstDetails show
ShowUnInstDetails show

; ---------------------------------------------------------------------------
; Modern UI 2
; ---------------------------------------------------------------------------
!include "MUI2.nsh"
!define MUI_ICON "..\assets\icon.ico"
!define MUI_UNICON "..\assets\icon.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\PaperGrain.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${APPNAME}"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; ---------------------------------------------------------------------------
; Sections
; ---------------------------------------------------------------------------
Section "${APPNAME} (required)" SEC_CORE
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "..\build\PaperGrain.exe"
  File /oname=LICENSE.txt "..\LICENSE"
  File /oname=README.txt "..\README-installer.txt"
  SetOutPath "$INSTDIR\textures"
  File "..\textures\fine-grain.png"
  File "..\textures\craft-paper.png"
  File "..\textures\notebook.png"
  File "..\textures\parchment.png"
  SetOutPath "$INSTDIR"

  WriteUninstaller "$INSTDIR\Uninstall.exe"

  WriteRegStr HKLM "Software\${APPNAME}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayName" "${APPNAME}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "DisplayIcon" "$INSTDIR\PaperGrain.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "URLInfoAbout" "https://github.com/cookiefilled/papergrain"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "NoRepair" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}" "EstimatedSize" 700
SectionEnd

Section "Start Menu shortcuts" SEC_SM
  CreateDirectory "$SMPROGRAMS\${APPNAME}"
  CreateShortcut "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk" "$INSTDIR\PaperGrain.exe"
  CreateShortcut "$SMPROGRAMS\${APPNAME}\Uninstall ${APPNAME}.lnk" "$INSTDIR\Uninstall.exe"
SectionEnd

Section "Desktop shortcut" SEC_DM
  CreateShortcut "$DESKTOP\${APPNAME}.lnk" "$INSTDIR\PaperGrain.exe"
SectionEnd

; ---------------------------------------------------------------------------
; Uninstaller
; ---------------------------------------------------------------------------
Section "Uninstall"
  ; stop a running instance (best-effort, silent)
  nsExec::ExecToLog 'taskkill /F /IM PaperGrain.exe'
  Pop $0

  Delete "$INSTDIR\PaperGrain.exe"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\textures\fine-grain.png"
  Delete "$INSTDIR\textures\craft-paper.png"
  Delete "$INSTDIR\textures\notebook.png"
  Delete "$INSTDIR\textures\parchment.png"
  RMDir "$INSTDIR\textures"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk"
  Delete "$SMPROGRAMS\${APPNAME}\Uninstall ${APPNAME}.lnk"
  RMDir "$SMPROGRAMS\${APPNAME}"
  Delete "$DESKTOP\${APPNAME}.lnk"

  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"
  DeleteRegKey HKLM "Software\${APPNAME}"
  ; per-user settings / autostart of the uninstalling user
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APPNAME}"

  MessageBox MB_YESNO|MB_ICONQUESTION \
      "Also delete your PaperGrain settings and custom textures?$\r$\n($APPDATA\PaperGrain)" \
      IDNO +2
    RMDir /r "$APPDATA\PaperGrain"
SectionEnd

LangString DESC_CORE ${LANG_ENGLISH} "PaperGrain and its texture presets (required)."
LangString DESC_SM ${LANG_ENGLISH} "Shortcuts in the Start Menu."
LangString DESC_DM ${LANG_ENGLISH} "Shortcut on the Desktop."

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
!insertmacro MUI_DESCRIPTION_TEXT ${SEC_CORE} $(DESC_CORE)
!insertmacro MUI_DESCRIPTION_TEXT ${SEC_SM} $(DESC_SM)
!insertmacro MUI_DESCRIPTION_TEXT ${SEC_DM} $(DESC_DM)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; ---------------------------------------------------------------------------
; Version info for the installer itself
; ---------------------------------------------------------------------------
VIAddVersionKey "ProductName" "${APPNAME} Setup"
VIAddVersionKey "FileDescription" "${APPNAME} Setup"
VIAddVersionKey "FileVersion" "${VERSION}.0"
VIAddVersionKey "ProductVersion" "${VERSION}.0"
VIAddVersionKey "LegalCopyright" "(c) 2026 CookieFilled - MIT License"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIProductVersion "${VERSION}.0"
