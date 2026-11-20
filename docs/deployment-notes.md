# Raspberry Pi 5 deployment notes

The application targets a Raspberry Pi 5 with 8 GB RAM, a 7-inch touchscreen and a 64-bit Raspberry Pi OS installation. Windows testing covers the application protocol and browser viewport sizes. Physical ARM64 hardware, thermals, latency and touch keyboard behavior still need measurement.

## Prepare the device

Install a supported Python 3.11+ interpreter with venv, an ARM64 llama.cpp CPU build and an appropriate local GGUF. Build the frontend on the development laptop and copy `frontend/dist/` with the backend, or install Node 22.12+ on the Pi to build locally. The built UI requires only FastAPI and llama-server at runtime. Install wheels matching the Pi's Python, Linux and architecture. Windows virtual environments and inference binaries cannot be transferred as working runtimes.

~~~bash
python3 -m venv .venv
.venv/bin/python -m pip install -r backend/requirements.txt
cp -n backend/.env.example backend/.env
# If building the UI on this device
(cd frontend && npm ci && npm run build)
~~~

Start the two services in separate terminals on the device.

~~~bash
llama-server -m ./models/assistant.gguf --alias odyssey-local --host 127.0.0.1 --port 8080 -c 4096 -np 1 -ngl 0 --jinja --offline --no-webui --no-context-shift --cors-origins localhost
~~~

~~~bash
.venv/bin/python backend/run.py
~~~

For reproducible offline setup, prepare the native runtime, weights, platform-specific Python wheelhouse and prebuilt UI before disconnecting. Confirm both `/api/health` and `/api/system/status`. The hardware-independent device information implementation uses Python's standard platform API.

## Touchscreen and kiosk

A normal Chromium tab supports the console's fullscreen button. The following command starts kiosk mode from a graphical session.

~~~bash
chromium --kiosk --no-first-run http://127.0.0.1:8000/
~~~

For unattended boot, use separate unprivileged systemd services for llama-server and FastAPI, absolute paths, a writable application data directory and `Restart=on-failure`. FastAPI can start while the model loads. In a labwc desktop session, launch Chromium from the session user's `~/.config/labwc/autostart` after the backend is reachable. Use the executable name supplied by the installed Chromium package. [Raspberry Pi kiosk guide](https://www.raspberrypi.com/tutorials/how-to-use-a-raspberry-pi-in-kiosk-mode/)

## Hardware acceptance

- Repeat the model acceptance table at 800 × 480 and 1024 × 600 with a physical touchscreen. Check portrait rotation, scrolling, fullscreen, typing, sending and stopping with the on-screen keyboard.
- Record cold-start time, time to first token, tokens per second, peak resident memory, temperature, throttling and battery draw with each candidate quantization.
- Verify correct time after an offline cold boot. The time tool reports the configured device clock and timezone. An RTC and backup power may be needed for the product.
- Test shutdown, unclean power loss and SQLite recovery on test media. Measure storage wear and verify backup/restore with the chosen SD or SSD.
- Run the disconnected-network test before selecting a model or packaging the device.
