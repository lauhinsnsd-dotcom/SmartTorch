@echo off
rem Starts the overlay for a Chinese game client: it runs in the tray (right-click its icon, Quit; or Ctrl+Alt+Q).
rem Edit GAMELOG if the game is installed somewhere else. start-zh-debug.bat shows what it does and saves pictures.
cd /d "%~dp0"
set "GAMELOG=E:\Torchlight Infinite\Game\UE_game\TorchLight\Saved\Logs\UE_game.log"
if not exist "vorax-overlay.exe" (
  echo vorax-overlay.exe is not in this folder yet. Run build-windows.bat first.
  pause
  exit /b
)
if not exist "%GAMELOG%" (
  echo Cannot find the game log:
  echo %GAMELOG%
  echo In the game: Esc - Account Related - Enable Log, then log out and back in.
  echo If the game is installed somewhere else, edit the GAMELOG line in this file.
  pause
  exit /b
)
start "" "%~dp0vorax-overlay.exe" --log "%GAMELOG%" --ocr-lang zh
exit /b 0
