@echo off
rem Starts the overlay for a Chinese game client with a console and saves every picture read (for reporting a misread).
rem GAMELOG: leave empty, the overlay finds the game's log itself. Fill it in only if the tray says it can't.
cd /d "%~dp0"
set "GAMELOG="
if not exist "vorax-overlay.exe" (
  echo vorax-overlay.exe is not in this folder yet. Run build-windows.bat first.
  pause
  exit /b
)
if not exist "%~dp0frames" mkdir "%~dp0frames"
echo Pictures read are saved to: %~dp0frames  (send them if a board is misread)
echo Keep this window open while you play.
if not defined GAMELOG goto find
if not exist "%GAMELOG%" (
  echo Cannot find the game log:
  echo %GAMELOG%
  echo In the game: Esc - Account Related - Enable Log, then log out and back in.
  echo Or empty the GAMELOG line in this file: the overlay then looks for the log itself.
  pause
  exit /b
)
echo Game log: %GAMELOG%
vorax-overlay.exe --log "%GAMELOG%" --ocr-lang zh --frames "%~dp0frames" --console
pause
exit /b
:find
echo Game log: looked for by the overlay (in the game's install folder once the game is open)
vorax-overlay.exe --ocr-lang zh --frames "%~dp0frames" --console
pause
