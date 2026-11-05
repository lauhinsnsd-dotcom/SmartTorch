@echo off
rem Builds vorax-overlay.exe (with Chinese client support) from this folder.
rem First run: installs Rust and the Visual Studio C++ Build Tools with winget (about 3-6 GB, 15-30 minutes).
setlocal
cd /d "%~dp0"

where cargo >nul 2>nul
if errorlevel 1 if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>nul
if errorlevel 1 (
  echo [1/3] Installing the Visual Studio C++ Build Tools. Approve the Windows prompt if one appears...
  winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-package-agreements --accept-source-agreements --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
  echo [2/3] Installing Rust...
  winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements
  set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)
where cargo >nul 2>nul
if errorlevel 1 (
  echo.
  echo Rust did not install. Close this window, open a new one and run build-windows.bat again.
  echo If it still fails, install Rust from https://rustup.rs and run this file again.
  pause
  exit /b 1
)

echo [3/3] Building SmartTorch.exe (a few minutes the first time)...
rustup default stable >nul 2>nul
rem files unzipped over an older copy can keep old dates: rebuild the reader and the overlay every time
cargo clean --release -p vorax-scan -p vorax-overlay >nul 2>nul
cargo build --release -p vorax-overlay
if errorlevel 1 (
  echo.
  echo The build failed. Take a screenshot of the red error text above and send it.
  pause
  exit /b 1
)
copy /y "target\release\vorax-overlay.exe" "%~dp0SmartTorch.exe" >nul
echo.
echo Done: %~dp0SmartTorch.exe
echo Start it by double-clicking SmartTorch.exe
pause
