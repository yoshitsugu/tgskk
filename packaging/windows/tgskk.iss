#ifndef Payload
  #error Payload is required
#endif
#ifndef AppVersion
  #error AppVersion is required
#endif
#ifndef Output
  #error Output is required
#endif
[Setup]
AppId={{FE2BC384-949B-4DCC-AB60-40A41D5D8836}
AppName=tgskk
AppVersion={#AppVersion}
AppPublisher=tgskk contributors
AppPublisherURL=https://github.com/yoshitsugu/tgskk
DefaultDirName={autopf}\tgskk
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
MinVersion=10.0.19041
OutputDir={#Output}
OutputBaseFilename=tgskk-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
CloseApplications=no
RestartApplications=no
LicenseFile={#Payload}\licenses\LICENSE
UninstallDisplayName=tgskk
[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
[Files]
Source: "{#Payload}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "tgskk-tip.dll"
Source: "{#Payload}\tgskk-tip.dll"; DestDir: "{app}"; Flags: ignoreversion regserver
[Icons]
Name: "{autoprograms}\tgskk settings"; Filename: "{app}\tgskk-settings.exe"
[Code]
function OpenExclusiveFile(Name: string; Access, Share: Cardinal; Security: Integer;
  Creation, Flags: Cardinal; Template: Integer): THandle;
  external 'CreateFileW@kernel32.dll stdcall';
function CloseFileHandle(Handle: THandle): Boolean;
  external 'CloseHandle@kernel32.dll stdcall';

function FilesAvailable(): Boolean;
var
  Names: TArrayOfString;
  I: Integer;
  Handle: THandle;
begin
  Result := False;
  Names := ['tgskk-tip.dll', 'tgskk-broker.exe', 'tgskk-settings.exe',
            'tgskk-zenz-worker.exe', 'tgskk-google-skkserv.exe'];
  for I := 0 to GetArrayLength(Names) - 1 do
    if FileExists(ExpandConstant('{app}\') + Names[I]) then begin
      Handle := OpenExclusiveFile(ExpandConstant('{app}\') + Names[I],
        $C0000000, 0, 0, 3, 0, 0);
      if Handle = THandle(-1) then exit;
      CloseFileHandle(Handle);
    end;
  Result := True;
end;

function PrepareToInstall(var NeedsRestart: Boolean): string;
var Registered: string;
begin
  Result := '';
  if RegQueryStringValue(HKLM64,
      'Software\Classes\CLSID\{C7993BD4-1094-4968-A311-3DB8A0E01739}\InprocServer32',
      '', Registered) and
      (CompareText(Registered, ExpandConstant('{app}\tgskk-tip.dll')) <> 0) then begin
    Result := 'Another tgskk DLL is registered. Unregister that build before installing.';
    exit;
  end;
  if not FilesAvailable() then
    Result := 'tgskk is in use. Close input applications and the broker. Sign out other sessions before retrying. No process will be forcibly terminated.';
end;

function InitializeUninstall(): Boolean;
begin
  Result := FilesAvailable();
  if not Result then
    SuppressibleMsgBox('tgskk is in use. Close input applications and the broker, or sign out and retry. Your settings and learned dictionary will be kept.', mbError, MB_OK, IDOK);
end;

procedure CurUninstallStepChanged(Step: TUninstallStep);
var Filename: string;
begin
  if Step = usUninstall then begin
    if not FilesAvailable() then
      RaiseException('tgskk is in use. Installed files were kept. Sign out and retry.');
    Filename := ExpandConstant('{app}\tgskk-tip.dll');
    if not UnregisterServer(True, Filename, True) then begin
      try
        RegisterServer(True, Filename, True);
      except
        Log('Could not restore TSF registration: ' + GetExceptionMessage);
      end;
      RaiseException('TSF unregistration failed. Installed files were kept. Check permissions and retry, or run Setup to repair.');
    end;
  end;
end;
