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
Name: "explorer"; Description: "Add Play and Edit to the Explorer right-click menu"
Name: "player"; Description: "Use Cutline as a video player"

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
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp4\shell\CutlinePlay"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Play with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp4\shell\CutlinePlay"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mp4\shell\CutlinePlay\command"; ValueType: string; ValueData: """{app}\cutline.exe"" --play ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mov\shell\CutlinePlay"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Play with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mov\shell\CutlinePlay"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mov\shell\CutlinePlay\command"; ValueType: string; ValueData: """{app}\cutline.exe"" --play ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mkv\shell\CutlinePlay"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Play with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mkv\shell\CutlinePlay"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.mkv\shell\CutlinePlay\command"; ValueType: string; ValueData: """{app}\cutline.exe"" --play ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.webm\shell\CutlinePlay"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Play with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.webm\shell\CutlinePlay"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.webm\shell\CutlinePlay\command"; ValueType: string; ValueData: """{app}\cutline.exe"" --play ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.avi\shell\CutlinePlay"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Play with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.avi\shell\CutlinePlay"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.avi\shell\CutlinePlay\command"; ValueType: string; ValueData: """{app}\cutline.exe"" --play ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.m4v\shell\CutlinePlay"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Play with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.m4v\shell\CutlinePlay"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.m4v\shell\CutlinePlay\command"; ValueType: string; ValueData: """{app}\cutline.exe"" --play ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wmv\shell\CutlinePlay"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Play with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wmv\shell\CutlinePlay"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.wmv\shell\CutlinePlay\command"; ValueType: string; ValueData: """{app}\cutline.exe"" --play ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flv\shell\CutlinePlay"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Play with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flv\shell\CutlinePlay"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.flv\shell\CutlinePlay\command"; ValueType: string; ValueData: """{app}\cutline.exe"" --play ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.ts\shell\CutlinePlay"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Play with Cutline"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.ts\shell\CutlinePlay"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\cutline.exe,0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\SystemFileAssociations\.ts\shell\CutlinePlay\command"; ValueType: string; ValueData: """{app}\cutline.exe"" --play ""%1"""; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\Cutline.Video"; ValueType: string; ValueData: "Cutline video"; Flags: uninsdeletekey; Tasks: player
Root: HKCU; Subkey: "Software\Classes\Cutline.Video\DefaultIcon"; ValueType: string; ValueData: "{app}\cutline.exe,0"; Tasks: player
Root: HKCU; Subkey: "Software\Classes\Cutline.Video\shell\open\command"; ValueType: string; ValueData: """{app}\cutline.exe"" --play ""%1"""; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "Cutline"; Flags: uninsdeletekey; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Video player and editor"; Tasks: player
Root: HKCU; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "Cutline"; ValueData: "Software\Cutline\Capabilities"; Flags: uninsdeletevalue; Tasks: player
Root: HKCU; Subkey: "Software\Classes\.mp4\OpenWithProgids"; ValueType: string; ValueName: "Cutline.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mp4"; ValueData: "Cutline.Video"; Tasks: player
Root: HKCU; Subkey: "Software\Classes\.mov\OpenWithProgids"; ValueType: string; ValueName: "Cutline.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mov"; ValueData: "Cutline.Video"; Tasks: player
Root: HKCU; Subkey: "Software\Classes\.mkv\OpenWithProgids"; ValueType: string; ValueName: "Cutline.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".mkv"; ValueData: "Cutline.Video"; Tasks: player
Root: HKCU; Subkey: "Software\Classes\.webm\OpenWithProgids"; ValueType: string; ValueName: "Cutline.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".webm"; ValueData: "Cutline.Video"; Tasks: player
Root: HKCU; Subkey: "Software\Classes\.avi\OpenWithProgids"; ValueType: string; ValueName: "Cutline.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".avi"; ValueData: "Cutline.Video"; Tasks: player
Root: HKCU; Subkey: "Software\Classes\.m4v\OpenWithProgids"; ValueType: string; ValueName: "Cutline.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".m4v"; ValueData: "Cutline.Video"; Tasks: player
Root: HKCU; Subkey: "Software\Classes\.wmv\OpenWithProgids"; ValueType: string; ValueName: "Cutline.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".wmv"; ValueData: "Cutline.Video"; Tasks: player
Root: HKCU; Subkey: "Software\Classes\.flv\OpenWithProgids"; ValueType: string; ValueName: "Cutline.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".flv"; ValueData: "Cutline.Video"; Tasks: player
Root: HKCU; Subkey: "Software\Classes\.ts\OpenWithProgids"; ValueType: string; ValueName: "Cutline.Video"; ValueData: ""; Flags: uninsdeletevalue; Tasks: player
Root: HKCU; Subkey: "Software\Cutline\Capabilities\FileAssociations"; ValueType: string; ValueName: ".ts"; ValueData: "Cutline.Video"; Tasks: player

[Run]
Filename: "ms-settings:defaultapps?registeredAppUser=Cutline"; Description: "Choose Cutline as the default video player"; Flags: shellexec nowait postinstall; Tasks: player
Filename: "{app}\cutline.exe"; Description: "Launch Cutline"; Flags: nowait postinstall

[Messages]
FinishedLabel=Setup has installed [name] on your computer.%n%nCutline requires ffmpeg and ffprobe. Install them and add them to PATH, or place them next to cutline.exe.
