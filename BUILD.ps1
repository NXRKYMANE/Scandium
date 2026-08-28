# Scandium 构建脚本：读取版本号 → 构建 Rust 项目 → 编译 Inno 安装包
# 用法: .\BUILD.ps1

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot

# Rust MSVC 工具链：无 VS（vswhere）时使用本机 F:\DevTools 固化的 MSVC + Windows SDK（自动取最新版本）
$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) {
    $msvc = Get-ChildItem "F:\DevTools\MSVC" -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1
    $sdkVer = Get-ChildItem "F:\DevTools\Windows11 SDK\Lib" -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1
    $sdkBase = "F:\DevTools\Windows11 SDK"
    if ($msvc -and $sdkVer -and (Test-Path "$($msvc.FullName)\bin\Hostx64\x64\link.exe")) {
        # link.exe 入 PATH 供 rustc 调用，LIB/INCLUDE 指向固化 SDK
        $env:Path = "$($msvc.FullName)\bin\Hostx64\x64;$env:Path"
        $env:LIB = "$($msvc.FullName)\lib\x64;$sdkBase\Lib\$($sdkVer.Name)\ucrt\x64;$sdkBase\Lib\$($sdkVer.Name)\um\x64"
        $env:INCLUDE = "$($msvc.FullName)\include;$sdkBase\Include\$($sdkVer.Name)\ucrt;$sdkBase\Include\$($sdkVer.Name)\um;$sdkBase\Include\$($sdkVer.Name)\shared"
    }
}

# Inno Setup 6 / 7 兼容路径
$Iscc = @(
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 7\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $Iscc) { throw "Inno Setup not found" }

# 1. 从 Cargo.toml 读取版本号
$cargo = Get-Content "$ProjectRoot\Project\Cargo.toml"
$version = ($cargo | Select-String '^version = ".*"').Line -replace 'version = "|"', ''
Write-Host "Version: $version" -ForegroundColor Cyan

# 2. 构建 Rust 项目
Write-Host "Building project..." -ForegroundColor Yellow
cargo build --release --manifest-path "$ProjectRoot\Project\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw "Build failed" }

# 3. 复制产物到 Publish（installer.iss 的 Source 相对 Project 目录）
Write-Host "Copying artifact..." -ForegroundColor Yellow
$publishDir = "$ProjectRoot\Publish"
New-Item -ItemType Directory -Path $publishDir -Force | Out-Null
Copy-Item "$ProjectRoot\Project\target\release\scandium_svc.exe" "$publishDir\scandium_svc.exe" -Force

# 3.5 UPX 极限压缩（LZMA + 极限暴力压缩，体积最极限）
$upxPath = "F:\DevTools\UPX\upx.exe"
if (-not (Test-Path $upxPath)) { throw "UPX not found at $upxPath" }
Write-Host "Compressing with UPX (--ultra-brute --lzma)..." -ForegroundColor Yellow
& $upxPath --ultra-brute --lzma "$publishDir\scandium_svc.exe"
if ($LASTEXITCODE -ne 0) { throw "UPX compression failed" }

# 4. 更新 Project\installer.iss 中的版本号
Write-Host "Updating installer.iss..." -ForegroundColor Yellow
$iss = Get-Content "$ProjectRoot\Project\installer.iss" -Raw -Encoding UTF8
$iss = $iss -replace '#define MyAppVersion ".*"', "#define MyAppVersion `"$version`""
[System.IO.File]::WriteAllText("$ProjectRoot\Project\installer.iss", $iss, [System.Text.UTF8Encoding]::new($true))

# 5. 编译 Inno 安装包
Write-Host "Compiling installer..." -ForegroundColor Yellow
& $Iscc "$ProjectRoot\Project\installer.iss"
if ($LASTEXITCODE -ne 0) { throw "Installer build failed" }

Write-Host "Done: Publish\scandium-svc-win-x64-setup-v$version.exe" -ForegroundColor Green
