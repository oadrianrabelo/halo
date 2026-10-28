; Instalador do Halo para Windows (Inno Setup 6).
; O CI compila com: iscc /DVersion=X.Y.Z /DSource=caminho\halo.exe installer\halo.iss

#ifndef Version
  #define Version "0.0.0"
#endif
#ifndef Source
  #define Source "..\target\release\halo.exe"
#endif

[Setup]
AppId={{6F1C2B7E-4A0D-4C1E-9D57-8A3B1F0E2C44}
AppName=Halo
AppVersion={#Version}
AppVerName=Halo {#Version}
AppPublisher=Halo
AppPublisherURL=https://github.com/oadrianrabelo/halo
DefaultDirName={localappdata}\Programs\Halo
DefaultGroupName=Halo
DisableProgramGroupPage=yes
DisableDirPage=auto
; Instala só para o usuário atual: não pede administrador e permite a atualização automática.
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=halo-setup-windows-x64
SetupIconFile=..\assets\halo.ico
UninstallDisplayIcon={app}\halo.exe
UninstallDisplayName=Halo
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "Criar atalho na área de trabalho"; GroupDescription: "Atalhos:"; Flags: unchecked
Name: "startup"; Description: "Abrir o Halo junto com o Windows"; GroupDescription: "Inicialização:"; Flags: unchecked

[Files]
Source: "{#Source}"; DestDir: "{app}"; DestName: "halo.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Halo"; Filename: "{app}\halo.exe"
Name: "{autodesktop}\Halo"; Filename: "{app}\halo.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Halo"; ValueData: """{app}\halo.exe"""; Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\halo.exe"; Description: "Abrir o Halo agora"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Sobra da atualização automática.
Type: files; Name: "{app}\halo.exe.old"
