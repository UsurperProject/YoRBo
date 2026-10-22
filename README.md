<img width="1686" height="592" alt="YoRBo" src="https://github.com/user-attachments/assets/a5b58d05-6def-408a-bd9b-e131605377bc" />

# YoRBo: You only ReBoot once

---

YoRBo is a Windows-based tool that aims to make booting operating system images directly from your existing system simple and convenient.

Boot ISO, IMG, VHDX and etc. images without needing to prepare a bootable USB drive. YoRBo handles the boot process so you can focus on what you actually want to run.

## How It Works

YoRBo relies heavily on [Ventoy](https://github.com/ventoy/Ventoy) for its boot infrastructure, building on Ventoy's image-booting capabilities rather than reinventing an entire bootloader stack.

This also means YoRBo inherits much of Ventoy's existing operating system compatibility out of the box. Where YoRBo's workflow differs from Ventoy's standard use cases, the project aims to actively bridge the gap through additional integration and development.

## Goals

- **Boot without removable media** — launch supported images directly from your existing system.
- **Minimal overhead** — avoid unnecessary image extraction, duplication, and installation steps.
- **Broad compatibility** — leverage Ventoy's established compatibility while working toward closing the remaining gaps.
- **A simpler workflow** — make booting another operating system as straightforward as selecting an image and rebooting.

## Current Limitations

- **MBR boot** is not currently supported.
- **Secure Boot** is not currently supported.

Compatibility and functionality are expected to grow as the project matures.

## Roadmap

- [ ] **WIP ⚠️** Windows installer
- [ ] Linux support and installer
- [ ] Secure Boot
- [ ] MBR Supprt
- [ ] Built-in linux Virtual Disk Container (.raw, .vhd) creation and vtoyboot setup support

## UsurperProject

YoRBo is part of [UsurperProject](https://github.com/UsurperProject), an open-source initiative focused on making operating system installation and booting more accessible through reusable tools and infrastructure.

## Status

YoRBo is under active development. Features, compatibility, and supported configurations may change as the project evolves.

---

*You only ReBoot once.*
