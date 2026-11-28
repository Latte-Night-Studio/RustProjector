; SoftProjector per-user installer.
;
; IMPORTANT: keep this file encoded as UTF-8 with a BOM.  NSIS 3.x only
; treats the script as UTF-8 when the BOM is present; otherwise Ukrainian
; strings are decoded using the system code page and appear garbled.
;
; The installer is intentionally per-user (RequestExecutionLevel user) and
; installs into %LOCALAPPDATA%\Programs\RustProjector.  This avoids UAC and
; lets the built-in updater replace the executable without administrator
; rights.
;
; Build with installer\build-installer.ps1 or directly:
;   makensis /DAPP_VERSION=0.2.0 /DSOURCE_EXE=...\softprojector.exe RustProjector.nsi

Unicode true
ManifestDPIAware true
ManifestSupportedOS all

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

!ifndef APP_VERSION
  !define APP_VERSION "0.2.0"
!endif
!ifndef SOURCE_EXE
  !define SOURCE_EXE "..\target\release\softprojector.exe"
!endif
!ifndef PROJECT_ROOT
  !define PROJECT_ROOT ".."
!endif
!ifndef OUTPUT_EXE
  !define OUTPUT_EXE "RustProjector-v${APP_VERSION}-windows-x64-setup.exe"
!endif

!define APP_NAME "SoftProjector"
!define APP_PUBLISHER "Latte-Night-Studio"
!define APP_URL "https://github.com/Latte-Night-Studio/RustProjector"
!define APP_RELEASES "https://github.com/Latte-Night-Studio/RustProjector/releases/latest"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\SoftProjector"
!define SETTINGS_KEY "Software\SoftProjector"

Name "${APP_NAME}"
BrandingText "${APP_NAME} ${APP_VERSION}"
OutFile "${OUTPUT_EXE}"
InstallDir "$LOCALAPPDATA\Programs\RustProjector"
InstallDirRegKey HKCU "${SETTINGS_KEY}" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 64
CRCCheck on

VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey /LANG=1058 "ProductName" "${APP_NAME}"
VIAddVersionKey /LANG=1058 "ProductVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=1058 "FileDescription" "Встановлювач ${APP_NAME}"
VIAddVersionKey /LANG=1058 "FileVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=1058 "CompanyName" "${APP_PUBLISHER}"
VIAddVersionKey /LANG=1058 "LegalCopyright" "GPL-3.0-or-later"
VIAddVersionKey /LANG=1058 "OriginalFilename" "RustProjector-v${APP_VERSION}-windows-x64-setup.exe"

!define MUI_ABORTWARNING
!define MUI_ICON "${NSISDIR}\Contrib\Graphics\Icons\modern-install.ico"
!define MUI_UNICON "${NSISDIR}\Contrib\Graphics\Icons\modern-uninstall.ico"

!define MUI_WELCOMEPAGE_TITLE "Встановлення ${APP_NAME}"
!define MUI_WELCOMEPAGE_TEXT "Ця програма встановить ${APP_NAME} ${APP_VERSION} — церковну програму проєкції з українським інтерфейсом.$\r$\n$\r$\nПрограма встановиться для поточного користувача, тому права адміністратора не потрібні. Якщо ${APP_NAME} зараз запущено, його буде закрито.$\r$\n$\r$\nНатисніть «Далі», щоб продовжити."

!define MUI_FINISHPAGE_RUN "$INSTDIR\softprojector.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Запустити ${APP_NAME}"
!define MUI_FINISHPAGE_LINK "Сторінка проєкту на GitHub"
!define MUI_FINISHPAGE_LINK_LOCATION "${APP_URL}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${PROJECT_ROOT}\LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "Ukrainian"

Section "Основна програма (обов'язково)" SEC_MAIN
  SectionIn RO
  SetOutPath "$INSTDIR"
  SetShellVarContext current

  ; Close a running copy so the executable can be replaced.
  nsExec::Exec '"$SYSDIR\taskkill.exe" /IM softprojector.exe /F'
  Sleep 600

  File "/oname=softprojector.exe" "${SOURCE_EXE}"
  File "${PROJECT_ROOT}\LICENSE"
  File "${PROJECT_ROOT}\README.md"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKCU "${SETTINGS_KEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\softprojector.exe,0"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "URLInfoAbout" "${APP_URL}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "URLUpdateInfo" "${APP_RELEASES}"
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1

  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" $0
SectionEnd

Section "Ярлик у меню «Пуск»" SEC_STARTMENU
  SetShellVarContext current
  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\softprojector.exe" "" "$INSTDIR\softprojector.exe" 0
  CreateShortCut "$SMPROGRAMS\${APP_NAME}\Видалити ${APP_NAME}.lnk" "$INSTDIR\uninstall.exe"
SectionEnd

Section /o "Ярлик на робочому столі" SEC_DESKTOP
  SetShellVarContext current
  CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\softprojector.exe" "" "$INSTDIR\softprojector.exe" 0
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_MAIN} "Виконуваний файл, ліцензія та документація ${APP_NAME}."
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_STARTMENU} "Додати ярлики програми та видалення до меню «Пуск»."
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_DESKTOP} "Створити ярлик ${APP_NAME} на робочому столі."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Function .onInit
  ; Detect a previous installation and offer to use its directory.
  ReadRegStr $0 HKCU "${SETTINGS_KEY}" "InstallDir"
  ${If} $0 != ""
    StrCpy $INSTDIR $0
  ${EndIf}
FunctionEnd

Section "Uninstall"
  SetShellVarContext current

  nsExec::Exec '"$SYSDIR\taskkill.exe" /IM softprojector.exe /F'
  Sleep 600

  Delete "$INSTDIR\softprojector.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\uninstall.exe"

  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Видалити ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"
  Delete "$DESKTOP\${APP_NAME}.lnk"

  ; The database normally lives next to the executable.  Keep it by default
  ; so that songs, themes and schedules survive a reinstall.
  ${If} ${FileExists} "$INSTDIR\spData.sqlite"
    MessageBox MB_YESNO|MB_ICONQUESTION \
      "Видалити також базу даних spData.sqlite?$\r$\n$\r$\nУ ній зберігаються пісні, теми, біблійні версії та розклади. Якщо ви не впевнені, виберіть «Ні» — файл залишиться в теці встановлення." \
      /SD IDNO IDNO keep_data
    Delete "$INSTDIR\spData.sqlite"
    Delete "$INSTDIR\spData.sqlite-journal"
    Delete "$INSTDIR\spData.sqlite-wal"
    Delete "$INSTDIR\spData.sqlite-shm"
  ${EndIf}
  keep_data:

  RMDir "$INSTDIR"

  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  DeleteRegKey HKCU "${SETTINGS_KEY}"
SectionEnd
