#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

[Setup]
AppId={{B7A3C1E2-5D4F-4A8B-9C6E-2F1D8E7A4B30}
AppName=Cutline
AppVersion={#AppVersion}
AppPublisher=waykehatescoding
DefaultDirName={autopf}\Cutline
DefaultGroupName=Cutline
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
SetupIconFile=assets\icon.ico
UninstallDisplayIcon={app}\cutline.exe
OutputDir=dist
OutputBaseFilename=Cutline
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Tasks]
Name: "desktopicon"; Description: "Create a desktop icon"; Flags: unchecked
Name: "explorer"; Description: "Add Edit this video to the Explorer right-click menu"

[Files]
Source: "target\release\cutline.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Cutline"; Filename: "{app}\cutline.exe"
Name: "{autodesktop}\Cutline"; Filename: "{app}\cutline.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp4\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit this video"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp4\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp4\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mov\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit this video"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mov\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mov\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mkv\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit this video"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mkv\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mkv\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.webm\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit this video"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.webm\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.webm\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.avi\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit this video"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.avi\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.avi\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.m4v\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit this video"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.m4v\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.m4v\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wmv\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit this video"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wmv\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wmv\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flv\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit this video"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flv\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flv\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.ts\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit this video"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.ts\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.ts\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp3\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp3\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp3\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wav\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wav\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wav\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flac\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flac\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flac\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.m4a\shell\Cutline"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Edit with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.m4a\shell\Cutline"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.m4a\shell\Cutline\command"; ValueType: string; ValueData: """{app}\cutline.exe"" ""%1"""; Tasks: explorer

[Run]
Filename: "{app}\cutline.exe"; Description: "Launch Cutline"; Flags: nowait postinstall

[Messages]
FinishedLabel=Setup has installed [name] on your computer.%n%nCutline requires ffmpeg and ffprobe. Install them and add them to PATH, or place them next to cutline.exe.
