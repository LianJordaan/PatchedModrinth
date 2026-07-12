Unicode true

!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!define REPO "LianJordaan/ByteLauncher"

Name "ByteLauncher ${VERSION}"

!ifdef OFFLINE
  OutFile "ByteLauncher-Setup-Offline.exe"
!else
  OutFile "ByteLauncher-Setup.exe"
!endif

; Per-user install into the existing Modrinth App folder — no admin required.
RequestExecutionLevel user
InstallDir "$LOCALAPPDATA\Modrinth App"
ShowInstDetails show

!define MUI_ICON "icon.ico"
!define MUI_UNICON "icon.ico"
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\ByteLauncher.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ByteLauncher"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_LANGUAGE "English"

; --- Require an existing Modrinth App install ---------------------------------
Function .onInit
  SetShellVarContext current
  StrCpy $INSTDIR "$LOCALAPPDATA\Modrinth App"
  ${IfNot} ${FileExists} "$INSTDIR\Modrinth App.exe"
  ${AndIfNot} ${FileExists} "$INSTDIR\ByteLauncher.exe"
    MessageBox MB_ICONSTOP "The Modrinth App must be installed first.$\r$\n$\r$\nByteLauncher runs on top of the Modrinth App and reuses its data folder (instances, accounts, settings, worlds), so it can only be installed where the Modrinth App already exists at:$\r$\n$INSTDIR$\r$\n$\r$\nInstall the Modrinth App from https://modrinth.com/app, then run this installer again."
    Abort
  ${EndIf}
FunctionEnd

Section "ByteLauncher"
  SetShellVarContext current
  InitPluginsDir

  ; --- Close the app if it is running -----------------------------------------
  StrCpy $0 "0"
  nsExec::Exec 'cmd /c tasklist /NH /FI "IMAGENAME eq ByteLauncher.exe" | find /I "ByteLauncher.exe"'
  Pop $1
  ${If} $1 == "0"
    StrCpy $0 "1"
  ${EndIf}
  nsExec::Exec 'cmd /c tasklist /NH /FI "IMAGENAME eq Modrinth App.exe" | find /I "Modrinth App.exe"'
  Pop $1
  ${If} $1 == "0"
    StrCpy $0 "1"
  ${EndIf}
  ${If} $0 == "1"
    MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION "ByteLauncher (or the Modrinth App) is currently running and must be closed to continue.$\r$\n$\r$\nClick OK to close it and continue." IDOK bl_kill
    Abort
    bl_kill:
    nsExec::Exec 'taskkill /F /IM "ByteLauncher.exe"'
    Pop $1
    nsExec::Exec 'taskkill /F /IM "Modrinth App.exe"'
    Pop $1
    Sleep 1500
  ${EndIf}

  ; --- Obtain ByteLauncher.exe into a temp dir first --------------------------
  ; Nothing on disk is changed until we have a valid exe in hand.
!ifdef OFFLINE
  DetailPrint "Preparing bundled ByteLauncher ${VERSION}..."
  File "/oname=$PLUGINSDIR\ByteLauncher.exe" "ByteLauncher.exe"
!else
  DetailPrint "Downloading the latest ByteLauncher from GitHub..."
  File "/oname=$PLUGINSDIR\download.ps1" "download.ps1"
  nsExec::ExecToLog 'powershell -NoProfile -ExecutionPolicy Bypass -File "$PLUGINSDIR\download.ps1" -OutFile "$PLUGINSDIR\ByteLauncher.exe"'
  Pop $1
  ${If} $1 != "0"
    MessageBox MB_ICONSTOP "Failed to download ByteLauncher (error code $1).$\r$\n$\r$\nCheck your internet connection and try again, or use the offline installer. Nothing on your system was changed."
    Abort
  ${EndIf}
!endif

  ${IfNot} ${FileExists} "$PLUGINSDIR\ByteLauncher.exe"
    MessageBox MB_ICONSTOP "ByteLauncher could not be prepared. Nothing on your system was changed."
    Abort
  ${EndIf}

  ; --- Put ByteLauncher.exe in place first ------------------------------------
  ; Independent filename, so aborting here leaves the Modrinth App fully intact.
  ClearErrors
  CopyFiles /SILENT "$PLUGINSDIR\ByteLauncher.exe" "$INSTDIR\ByteLauncher.exe"
  ${If} ${Errors}
    MessageBox MB_ICONSTOP "Could not write ByteLauncher.exe to:$\r$\n$INSTDIR"
    Abort
  ${EndIf}

  ; --- Back up the ORIGINAL Modrinth App exe ----------------------------------
  ; Tell a REAL app exe (Modrinth or ByteLauncher, tens of MB) from our tiny shim
  ; (well under 2 MB) by file size: only a real exe is worth backing up, and we
  ; never overwrite an existing backup. This makes re-runs safe and avoids ever
  ; destroying the genuine original, even if a stale .old.exe were lying around.
  StrCpy $8 "0"
  ClearErrors
  FileOpen $9 "$INSTDIR\Modrinth App.exe" r
  ${IfNot} ${Errors}
    FileSeek $9 0 END $8
    FileClose $9
  ${EndIf}
  ${If} $8 > 3000000
    ; Modrinth App.exe is a real app exe, not our shim.
    ${IfNot} ${FileExists} "$INSTDIR\Modrinth App.old.exe"
      ClearErrors
      Rename "$INSTDIR\Modrinth App.exe" "$INSTDIR\Modrinth App.old.exe"
      ${If} ${Errors}
        MessageBox MB_ICONSTOP "Could not back up the existing Modrinth App.exe. Make sure the app is fully closed, then run the installer again."
        Abort
      ${EndIf}
    ${Else}
      MessageBox MB_ICONSTOP "Unexpected state: both a real Modrinth App.exe and a Modrinth App.old.exe already exist. To avoid destroying your original, nothing was changed. Remove or rename Modrinth App.old.exe, then run the installer again."
      Abort
    ${EndIf}
  ${EndIf}

  ; --- Install the compatibility shim as "Modrinth App.exe" -------------------
  ; Overwrites a previous shim on re-run; never overwrites Modrinth App.old.exe.
  SetOutPath "$INSTDIR"
  File "/oname=Modrinth App.exe" "shim.exe"

  ; --- Fresh Start Menu shortcut ----------------------------------------------
  CreateShortcut "$SMPROGRAMS\ByteLauncher.lnk" "$INSTDIR\ByteLauncher.exe" "" "$INSTDIR\ByteLauncher.exe" 0

  DetailPrint "ByteLauncher ${VERSION} installed to $INSTDIR"
SectionEnd
