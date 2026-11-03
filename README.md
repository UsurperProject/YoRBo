<img width="1686" height="592" alt="YoRBo" src="https://github.com/user-attachments/assets/a5b58d05-6def-408a-bd9b-e131605377bc" />

# YoRBo: You only ReBoot once


YoRBo is a tool that aims to make booting operating system images directly from your existing system simple and convenient.

Boot ISO, IMG, VHDX and etc. images without needing to prepare a bootable USB drive. YoRBo handles the boot process so you can focus on what you actually want to run.

> **New here?** Start with the [Windows manual](docs/MANUAL-WINDOWS.md) for requirements, installation and usage.

## Contents

- [How It Works](#how-it-works)
- [Goals](#goals)
- [Getting Started](#getting-started)
- [Current Limitations](#current-limitations)
- [Roadmap](#roadmap)
- [UsurperProject](#usurperproject)
- [Status](#status)

## How It Works

YoRBo relies heavily on [Ventoy](https://github.com/ventoy/Ventoy) for its boot infrastructure, building on Ventoy's image-booting capabilities rather than reinventing an entire bootloader stack.

This also means YoRBo inherits much of Ventoy's existing operating system compatibility out of the box. Where YoRBo's workflow differs from Ventoy's standard use cases, the project aims to actively bridge the gap through additional integration and development.

## Goals

- **Boot without removable media** — launch supported images directly from your existing system.
- **Minimal overhead** — avoid unnecessary image extraction, duplication, and installation steps.
- **Broad compatibility** — leverage Ventoy's established compatibility while working toward closing the remaining gaps.
- **A simpler workflow** — make booting another operating system as straightforward as selecting an image and rebooting.

## Getting Started

YoRBo currently supports **Windows** (UEFI firmware, GPT disk). The [Windows manual](docs/MANUAL-WINDOWS.md) walks through everything:

- [Before you begin](docs/MANUAL-WINDOWS.md#before-you-begin): what the installer changes and what to expect from an alpha build
- [Requirements](docs/MANUAL-WINDOWS.md#requirements)
- [Installation](docs/MANUAL-WINDOWS.md#installation)
- [Usage](docs/MANUAL-WINDOWS.md#usage): booting images and booting into Ventoy

In short: download the installer from the [latest release](https://github.com/UsurperProject/YoRBo/releases/latest), run it as Administrator, then right-click a supported image (`.iso`, `.img`, `.vhd`, `.vhdx`, `.wim`, `.efi` or `.vtoy`) to boot it.

## Current Limitations

- **MBR boot** is not currently supported.
- **Secure Boot** is not currently supported.

More caveats are listed in the manual's [Before you begin](docs/MANUAL-WINDOWS.md#before-you-begin) section. Compatibility and functionality are expected to grow as the project matures.

## Roadmap

- [x] [Windows installer](docs/MANUAL-WINDOWS.md) (Early Alpha)
- [ ] Linux support and installer
- [ ] Secure Boot
- [ ] MBR Support
- [ ] Built-in linux Virtual Disk Container (.raw, .vhd) creation and vtoyboot setup support


## UsurperProject

YoRBo is part of [UsurperProject](https://github.com/UsurperProject), an open-source initiative focused on making operating system installation and booting more accessible through reusable tools and infrastructure.

## Status

YoRBo is under active development. Features, compatibility, and supported configurations may change as the project evolves.


### AI Disclosure
- Use of AI has been made in this project primarily for designing and handling UI logic of the installer. They will be gradually replaced too.
- The core logic of the project and any part that touches your system are most manually coded by a human developer.

---

*You only ReBoot once.*
