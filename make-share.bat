@echo off
rem Makes vorax-zh-share.zip for friends: the built overlay, the start file and the licence. No building needed on their side.
cd /d "%~dp0"
if not exist "vorax-overlay.exe" (
  echo Run build-windows.bat first.
  pause
  exit /b 1
)
if exist share rmdir /s /q share
mkdir share
copy /y vorax-overlay.exe share\ >nul
copy /y start-zh.bat share\ >nul
copy /y start-zh-debug.bat share\ >nul
copy /y LICENSE share\LICENSE.txt >nul
rem the hint's Chinese font (Noto Sans TC, built into the exe) comes with its licence
copy /y data\fonts\OFL.txt share\FONT-LICENSE.txt >nul
rem the Chinese user guide (the only .txt here), under an ASCII name: zips keep Chinese names unreliably
for %%f in (*.txt) do copy /y "%%f" share\README-zh.txt >nul
if not exist share\README-zh.txt echo The Chinese guide (.txt) is missing from this folder.
if exist vorax-zh-share.zip del vorax-zh-share.zip
powershell -NoProfile -Command "Compress-Archive -Path 'share\*' -DestinationPath 'vorax-zh-share.zip'"
rmdir /s /q share
echo.
echo Done: %~dp0vorax-zh-share.zip  - send this file to your friends.
pause
