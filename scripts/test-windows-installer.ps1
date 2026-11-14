param([Parameter(Mandatory)][string]$Installer, [Parameter(Mandatory)][string]$Payload)
$ErrorActionPreference = 'Stop'
$Installer = (Resolve-Path $Installer).Path
$Payload = (Resolve-Path $Payload).Path
$directory = Join-Path $env:ProgramFiles 'tgskk'
$home = Join-Path $env:LOCALAPPDATA 'tgskk'
$registry = 'HKLM:\Software\Classes\CLSID\{C7993BD4-1094-4968-A311-3DB8A0E01739}\InprocServer32'
if ((Test-Path $directory) -or (Test-Path $home) -or (Test-Path $registry)) {
  throw 'Installer test requires a clean, disposable Windows environment'
}
function Run-Installer {
  $process = Start-Process $Installer -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART' -PassThru -Wait
  if ($process.ExitCode -ne 0) { throw "Installer failed: $($process.ExitCode)" }
}
try {
  Run-Installer
  $registered = (Get-Item $registry).GetValue('')
  if ($registered -ne (Join-Path $directory 'tgskk-tip.dll')) { throw 'Incorrect TSF registration' }
  foreach ($name in @('tgskk-settings.exe','tgskk-zenz-worker.exe','tgskk-google-skkserv.exe','SKK-JISYO.L','models\zenz-v3.2-small-Q5_K_M.gguf')) {
    if (!(Test-Path (Join-Path $directory $name))) { throw "Missing installed file: $name" }
  }
  foreach ($line in Get-Content (Join-Path $Payload 'SHA256SUMS')) {
    if ($line -notmatch '^([a-f0-9]{64})  (.+)$') { throw 'Invalid payload manifest' }
    $expected = $Matches[1]
    $installed = Join-Path $directory $Matches[2]
    if ((Get-FileHash $installed -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
      throw "Installed file hash mismatch: $installed"
    }
  }
  New-Item $home -ItemType Directory | Out-Null
  $utf8 = [Text.UTF8Encoding]::new($false)
  [IO.File]::WriteAllText((Join-Path $home 'windows.ini'), "[General]`nAI=false`nSkkserv=Disabled`n", $utf8)
  [IO.File]::WriteAllText((Join-Path $home 'SKK-JISYO.user'), ";; learning sentinel`n", $utf8)
  [IO.File]::WriteAllText((Join-Path $home 'SKK-JISYO.user.BAK'), 'backup sentinel', $utf8)
  $before = @{}
  Get-ChildItem $home -File | ForEach-Object { $before[$_.Name] = (Get-FileHash $_.FullName).Hash }
  Run-Installer
  foreach ($name in $before.Keys) {
    if ((Get-FileHash (Join-Path $home $name)).Hash -ne $before[$name]) { throw "Upgrade changed $name" }
  }
  $handle = [IO.File]::Open((Join-Path $directory 'tgskk-broker.exe'), [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::None)
  try {
    $blocked = Start-Process $Installer -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART' -PassThru -Wait
    if ($blocked.ExitCode -eq 0) { throw 'Installer replaced an in-use executable' }
  } finally { $handle.Dispose() }
  Add-Type -AssemblyName System.Drawing
  Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class TgskkWindowProbe {
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);
}
'@
  $gui = Start-Process (Join-Path $directory 'tgskk-settings.exe') -PassThru
  try {
    if (!$gui.WaitForInputIdle(10000)) { throw 'Settings GUI did not become idle' }
    $gui.Refresh()
    if ($gui.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Settings GUI did not create a window' }
    $rectangle = [TgskkWindowProbe+Rect]::new()
    if (![TgskkWindowProbe]::GetWindowRect($gui.MainWindowHandle, [ref]$rectangle)) { throw 'GUI window bounds unavailable' }
    $bitmap = [Drawing.Bitmap]::new($rectangle.Right - $rectangle.Left, $rectangle.Bottom - $rectangle.Top)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $dc = $graphics.GetHdc()
    try {
      if (![TgskkWindowProbe]::PrintWindow($gui.MainWindowHandle, $dc, 2)) { throw 'Cannot capture settings GUI' }
    } finally { $graphics.ReleaseHdc($dc); $graphics.Dispose() }
    try { $bitmap.Save((Join-Path $pwd 'build/windows/settings.png'), [Drawing.Imaging.ImageFormat]::Png) }
    finally { $bitmap.Dispose() }
    $inUse = Start-Process $Installer -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART' -PassThru -Wait
    if ($inUse.ExitCode -eq 0) { throw 'Installer replaced the running settings executable' }
    if (!$gui.CloseMainWindow() -or !$gui.WaitForExit(10000)) { throw 'Settings GUI did not close normally' }
  } finally {
    if (!$gui.HasExited) { $gui.Kill(); $gui.WaitForExit() }
    $gui.Dispose()
  }
  $uninstaller = Join-Path $directory 'unins000.exe'
  $class = Split-Path $registry
  $originalAcl = Get-Acl $class
  $denied = Get-Acl $class
  $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
  $rule = [Security.AccessControl.RegistryAccessRule]::new($sid, [Security.AccessControl.RegistryRights]::Delete, [Security.AccessControl.AccessControlType]::Deny)
  $denied.AddAccessRule($rule)
  try {
    Set-Acl $class $denied
    $failed = Start-Process $uninstaller -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART' -PassThru -Wait
    if ($failed.ExitCode -eq 0) { throw 'Uninstaller ignored failed TSF unregistration' }
    if (!(Test-Path (Join-Path $directory 'tgskk-tip.dll')) -or !(Test-Path $registry)) {
      throw 'Failed unregistration deleted files or the retained class path'
    }
  } finally { Set-Acl $class $originalAcl }
  $removed = Start-Process $uninstaller -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART' -PassThru -Wait
  if ($removed.ExitCode -ne 0) { throw "Uninstaller failed: $($removed.ExitCode)" }
  if (Test-Path $registry) { throw 'Uninstall retained TSF registration' }
  foreach ($name in $before.Keys) {
    if ((Get-FileHash (Join-Path $home $name)).Hash -ne $before[$name]) { throw "Uninstall changed $name" }
  }
} finally {
  if (Test-Path (Join-Path $directory 'unins000.exe')) {
    Start-Process (Join-Path $directory 'unins000.exe') -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART' -Wait
  }
  if (Test-Path $home) { Remove-Item $home -Recurse -Force }
}
