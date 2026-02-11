; B24 Tool Installer - Inno Setup Script
; Builds a setup.exe that installs B24 Tool with Start Menu + optional Desktop shortcut

#define MyAppName "B24 Tool"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "ClarkeFL"
#define MyAppURL "https://github.com/ClarkeFL/b24studio"
#define MyAppExeName "b24-tool.exe"

[Setup]
; IMPORTANT: Keep this AppId the same for all future versions!
AppId={{8F2B7A4E-3C1D-4E5F-9A8B-6D2E1F0C3A5B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\target\installer
OutputBaseFilename=B24-Tool-{#MyAppVersion}-Setup
SetupIconFile=..\assets\b24-tool.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{commondesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchAfterInstall,{#MyAppName}}"; Flags: nowait postinstall skipifsilent

[CustomMessages]
LaunchAfterInstall=Launch %1

[Code]
// Prevent downgrade: warn if a newer version is already installed
function InitializeSetup: Boolean;
var
  InstalledVersion: String;
begin
  Result := True;
  if RegQueryStringValue(HKEY_CURRENT_USER,
       'Software\Microsoft\Windows\CurrentVersion\Uninstall\{#SetupSetting("AppId")}_is1',
       'DisplayVersion', InstalledVersion) then
  begin
    if CompareStr(InstalledVersion, '{#MyAppVersion}') > 0 then
    begin
      MsgBox('A newer version (' + InstalledVersion + ') of {#MyAppName} is already installed.' + #13#10 +
             'Please uninstall it first if you want to downgrade.', mbInformation, MB_OK);
      Result := False;
    end;
  end;
end;
