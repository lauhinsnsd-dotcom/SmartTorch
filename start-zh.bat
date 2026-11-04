@echo off
rem Starts the overlay for a Chinese game client: it runs in the tray (right-click its icon, Quit; or Ctrl+Alt+Q).
rem GAMELOG: leave empty, the overlay finds the game's log itself. Fill it in only if the tray says it can't.
cd /d "%~dp0"
set "GAMELOG="
if not exist "vorax-overlay.exe" (
  echo vorax-overlay.exe is not in this folder yet. Run build-windows.bat first.
  pause
  exit /b
)
if not defined GAMELOG goto find
if not exist "%GAMELOG%" (
  echo Cannot find the game log:
  echo %GAMELOG%
  echo In the game: Esc - Account Related - Enable Log, then log out and back in.
  echo Or empty the GAMELOG line in this file: the overlay then looks for the log itself.
  pause
  exit /b
)
start "" "%~dp0vorax-overlay.exe" --log "%GAMELOG%" --ocr-lang zh
exit /b 0
:find
start "" "%~dp0vorax-overlay.exe" --ocr-lang zh
exit /b 0
