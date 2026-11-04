@echo off
rem Starts the overlay for a Chinese game client with a console and saves every picture read (for reporting a misread).
rem Edit GAMELOG if the game is installed somewhere else.
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
if not exist "%~dp0frames" mkdir "%~dp0frames"
echo Game log: %GAMELOG%
echo Pictures read are saved to: %~dp0frames  (send them if a board is misread)
echo Keep this window open while you play.
vorax-overlay.exe --log "%GAMELOG%" --ocr-lang zh --frames "%~dp0frames" --console
pause
