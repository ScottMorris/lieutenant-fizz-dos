@echo off
rem Windows: double-click this after installing DOSBox Staging (https://www.dosbox-staging.org).
if exist "%ProgramFiles%\DOSBox Staging\dosbox.exe" (
  "%ProgramFiles%\DOSBox Staging\dosbox.exe" --conf DOSBOX.CONF
) else (
  dosbox --conf DOSBOX.CONF
)
