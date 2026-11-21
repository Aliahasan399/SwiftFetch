@echo off
REM SwiftFetch native messaging host launcher (Windows).
REM
REM Native messaging cannot launch a .py file directly, so the host manifest
REM points at this batch file. Adjust PYTHON to the interpreter you want; the
REM host itself only needs the Python standard library.
setlocal
if not defined SWIFTFETCH_SHARED_SECRET (
  echo [swiftfetch] SWIFTFETCH_SHARED_SECRET is not set 1>&2
  exit /b 2
)
python "%~dp0host.py" %*
exit /b %ERRORLEVEL%
