# LoMux

LoMux is a lightweight cross-platform GUI for [FFmpeg](https://ffmpeg.org/). Its goal is to make common audio/video conversions as simple, free, and open source as possible for the layman. Driven by the extremely powerful CLI driven FFmpeg, LoMux comes with prebuilt executables for 64-bit macOS, Linux, and Windows. If you prefer not to install FFmpeg system-wide, you can drop the static `ffmpeg` and `ffprobe` binaries into the `bin/<platform>/` folders.

---

## Table of Contents
1. [Repository](#repository)
2. [Dependencies](#dependencies)  
3. [Usage](#usage)
    1. [macOS](#macos)
    2. [Linux](#linux)
    3. [Windows](#windows)
4. [Building from Source](#building-from-source)
5. [Contributing](#contributing)
6. [License](#license)

---

## 1. Repository
```bash
LoMux/
├── .gitignore
├── LICENSE
├── README.md
├── LoMux.py
├── assets/
│ ├── lomux.icns
│ └── lomux.png
├── bin/
│ ├── mac/ ← (empty) ffmpeg/ffprobe here if not installed
│ ├── linux/ ← (empty) ffmpeg/ffprobe here if not installed
│ └── windows/ ← (empty) ffmpeg/ffprobe here if not installed
├── dist/
│ ├── mac/
│ │ └── LoMux.app ← macOS .app bundle
│ ├── linux/
│ │ └── lomux ← linux ELF executable (chmod +x)
│ └── windows/
│ └── LoMux.exe ← windows .exe
└─── spec/
  ├── mac.spec
  ├── linux.spec
  └── win.spec
```

---

## 2. Dependencies
### Use

- **FFmpeg / FFprobe** installed globally or the core exes dropped into `bin/<platform>/`(if on Linux, make them chmod +x).

### Build & Use

- **Python 3.11+**  
- **FFmpeg / FFprobe** installed globally or the core exes dropped into `bin/<platform>/`(if on Linux, make them chmod +x).

#### Installing FFmpeg
Homebrew (macOS): 
```bash
brew install ffmpeg
```
APT, DNF, Pacman (Linux)
```bash
sudo apt-get install ffmpeg
sudo dnf install ffmpeg
sudo pacman -S ffmpeg
```
Winget (Windows)
```bash
winget install ffmpeg
```
Alternativly you can download and install a static build `or` you may drag ffmpeg + ffprobe exes into `bin/<platform>/` (if on Linux, make them chmod +x). LoMux will look both in the system path and the bin folder for versions of FFmpeg to use.

#### If you plan to build, LoMux itself depends on:
- [`Pyinstaller`](https://github.com/pyinstaller/pyinstaller)
- [`ttkthemes`](https://github.com/TkinterEP/ttkthemes)
- [`rich`](https://github.com/Textualize/rich)  
- [`Pillow`](https://python-pillow.org/)

Install them via your python package manger:

```bash
pip install pyinstaller \
ttkthemes \
rich \
pillow

apt install python-pyinstaller \
python-tk \
python-rich \
python-pillow

sudo pacman -S python-pyinstaller \
python-ttkthemes \
python-rich \
python-pillow tk
```

---

## 3. Usage
### i. macOS (Intel 64-bit)
- Install FFmpeg (See above)
- Move `LoMux/` folder wherever you plan on storing it or delete it if FFmpeg is installed. The only file you need with ffmpeg installed is the prebuilt `LoMux.app`. Otherwise you will need the `/bin` folder to store ffmpeg/ffprobe exe files.
- Double-click LoMux.app in Finder.
  - If macOS Gatekeeper “verifies” it, wait a moment. If it still closes on first run, open Terminal and run:
    ```bash 
    open /path/to/dist/LoMux.app/
    ```
  - (Optional) You may attempt to bypass gatekeeper from closing the app by `cd /path/to/Lomux/` and using:
    ```bash
    codesign --force --deep --sign - dist/LoMux.app
    xattr -dr com.apple.quarantine dist/LoMux.app
    ```
### ii. Linux (64-bit)
#### (Ubuntu / Debian / Mint / Arch / Fedora)
- Install FFmpeg (See above)
- Move `LoMux/` folder wherever you plan on storing it or delete it if FFmpeg is installed. The only file you need with ffmpeg installed is the prebuilt `LoMux` ELF in `dist` folder. Otherwise you will need the `/bin` folder to store ffmpeg/ffprobe exe files.
- chmod +x dist/linux/lomux
- Run:
  ```bash
  ./path/to/lomux
  ```
### iii. Windows (64-bit)
- Install FFmpeg (See above)
- Move `LoMux/` folder wherever you plan on storing it or delete it if FFmpeg is installed. The only file you need with ffmpeg installed is the prebuilt `LoMux.exe`. Otherwise you will need the `/bin` folder to store ffmpeg/ffprobe exe files.
- Double-click LoMux.exe in File Explorer.
  - If Windows Defender complains, “More info - Run anyway”.

---

## 4. Building from Source
### Install Dependencies

- **Python 3.11+**
- **FFmpeg / FFprobe** installed globally or the core exes dropped into `bin/<platform>/`(if on Linux, make them chmod +x). (See above)
- **Python Modules** pyinstaller, ttkthemes, rich, pillow (See above)

### Pyinstaller

#### macOS
```bash
cd /path/to/LoMux/
```
```bash
pyinstaller spec/mac.spec
```
or
```bash
pyinstaller \
  --clean \
  --windowed \
  --name LoMux.app \
  --icon assets/lomux.icns \
  --strip \
LoMux.py
```
Result: dist/`LoMux.app`

(Optional) You may attempt to bypass gatekeeper from closing the app by cd /path/to/Lomux/ and using:
```bash
codesign --force --deep --sign - dist/LoMux.app
xattr -dr com.apple.quarantine dist/LoMux.app
```
Refer to [Usage](#usage) once build is complete

#### Linux
##### (Ubuntu / Debian / Mint / Arch / Fedora)
```bash
cd /path/to/LoMux/
```
```bash
pyinstaller spec/linux.spec
```
or
```bash
pyinstaller \
  --clean \
  --windowed \
  --name LoMux \
  --strip \
  LoMux.py
```
Result: dist/`LoMux` ELF

Refer to [Usage](#usage) once build is complete

#### Windows
```bash
cd /path/to/LoMux/
```
```bash
pyinstaller spec/windows.spec
```
or
```bash
pyinstaller `
  --clean `
  --windowed `
  --name LoMux.exe `
  --icon assets\lomux.ico `
  --strip `
  LoMux.py
```
Result: dist\ `LoMux.exe`

Refer to [Usage](#usage) once build is complete

## Contributing

Contribution is welcome in the form of:
- Forking this repo
- Submiting a Pull Request
- Bug reports and feature requests

Please ensure your code follows the existing style (tabs for indentation) and that any new dependencies get added to the README.

## License

This project is licensed under the MIT License (see LICENSE for details).


## Thank you for your attention.
If you hit any issues, feel free to open an issue on GitHub.
Happy converting.
