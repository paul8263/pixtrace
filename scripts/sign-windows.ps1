<#
.SYNOPSIS
使用 Authenticode 证书签名 PixTrace 的 exe / msi 发布产物。

.EXAMPLE
$env:PIXTRACE_SIGN_PFX = "C:\certs\pixtrace.pfx"
$env:PIXTRACE_SIGN_PASSWORD = "***"
.\scripts\sign-windows.ps1 -Files .\dist-release\PixTrace_0.1.0_x64-setup.exe

.EXAMPLE
$env:PIXTRACE_SIGN_THUMBPRINT = "ABCDEF..."
.\scripts\sign-windows.ps1 -Files .\dist-release\PixTrace_0.1.0_x64_en-US.msi
#>
param(
    [Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)]
    [string[]] $Files,
    [string] $PfxPath = $env:PIXTRACE_SIGN_PFX,
    [string] $PfxPassword = $env:PIXTRACE_SIGN_PASSWORD,
    [string] $Thumbprint = $env:PIXTRACE_SIGN_THUMBPRINT,
    [string] $TimestampUrl = "http://timestamp.digicert.com",
    [string] $Signtool
)

$ErrorActionPreference = "Stop"

if (-not $Signtool) {
    $command = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($command) {
        $Signtool = $command.Source
    } else {
        $candidates = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match "\\x64\\signtool\.exe$" } |
            Sort-Object FullName -Descending
        if (-not $candidates) {
            throw "未找到 signtool.exe。请安装 Windows SDK，或通过 -Signtool 指定路径。"
        }
        $Signtool = $candidates[0].FullName
    }
}

if (-not $PfxPath -and -not $Thumbprint) {
    throw "请设置 PIXTRACE_SIGN_PFX 或 PIXTRACE_SIGN_THUMBPRINT。"
}

foreach ($file in $Files) {
    $resolved = (Resolve-Path $file).Path
    if ($PfxPath) {
        & $Signtool sign /fd SHA256 /f $PfxPath /p $PfxPassword /tr $TimestampUrl /td SHA256 $resolved
    } else {
        & $Signtool sign /fd SHA256 /sha1 $Thumbprint /tr $TimestampUrl /td SHA256 $resolved
    }
    if ($LASTEXITCODE -ne 0) {
        throw "签名失败：$resolved"
    }
    & $Signtool verify /pa $resolved
    if ($LASTEXITCODE -ne 0) {
        throw "签名校验失败：$resolved"
    }
    Write-Host "已签名：$resolved"
}
