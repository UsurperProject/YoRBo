# Installing and using YoRBo on Windows

> **Early Alpha:** The Windows installer is in Early Alpha. Expect rough edges,
> and please report issues you run into.

This guide walks through installing YoRBo on Windows, from prerequisites to
your first boot.

---

## Contents

- [Before you begin](#before-you-begin)
- [Requirements](#requirements)
- [Installation](#installation)
- [Usage](#usage)
  - [Booting Images](#booting-images)
  - [Booting Into Ventoy](#booting-into-ventoy)
  
---
## Before you begin
> [!IMPORTANT]
> Alpha Build. Expect quirks and unpolished edges.

> [!Note]
> - YoRBo in general should not cause any sort of data corruption. However, backing up your files beforehand is still recommended during the testing phase.
> - Secure Boot is **not** currently supported — see the README's Current Limitations.
> - MBR boot is **not** currently supported — UEFI systems only.
> - YoRBo does not bring any visible changes to your system boot order. However, it does add a safe invisible boot entry to your system UEFI; make sure you are  comfortable with that. You should note that you *CAN* still see, modify or delete the entry using tools such as bcdedit (windows), efibootmgr (linux) and efivarcli (windows/linux).

---

## Requirements

- UEFI firmware required (GPT disk, UEFI boot mode)
- Secure Boot must be disabled.
- Any modern Windows with UEFI and GPT support. 
- 100MB of free space
- Administrator rights required

---

## Installation

1. Download the latest installer from [release](https://github.com/UsurperProject/YoRBo/releases/latest)
2. Run as Administrator.
3. Choose a path for the runner and hit install.
> [!NOTE]
> This is where the non-bootloader related .exe files and configs will live.

<img width="640" height="460" alt="image" src="https://github.com/user-attachments/assets/3217fd36-dd4d-46e7-8a65-7b4a66a780dc" />

4. Select a FAT32 partition to install the bootloader files to.
> [!TIP]
> Try to avoid using the Windows EFI partition if possible for long term stability.
> If the only FAT32 you have is Windows EFI partition, you might want to go with step 5 as an alternative.

<img width="637" height="456" alt="image" src="https://github.com/user-attachments/assets/5fc536ef-2d9d-4abe-9178-2f9e87e1af97" />

5. Alternatively (and preferably, if your only FAT32 partition is the Windows EFI partition), you can have the installer create a new 100MB FAT32 partition for you from one of the existing partitions.
> [!NOTE]
> For this step, your selected partition will need to be able to shrink by at least 100MB.

> [!TIP]
>  If you choose to create new partition from `C:\`, it can take a minute or two to complete. Try not to panic.

6. Let the process finish.
---

## Usage

> [!CAUTION]
> Remember to close all browser tabs and save any work you were doing before proceeding to prevent loss of unsaved work due to reboot.

### Booting Images

On windows, you can right click any [.iso, .img, .vhd, .vhdx, .wim, .efi or .vtoy] file and the option to reboot into them should be there.

<img width="629" height="494" alt="image" src="https://github.com/user-attachments/assets/f1d55a4b-d9ac-423e-a2e1-abac8095d0c8" />


> [!NOTE]
> - .efi files can only be booted off of FAT32 partitions because that's some UEFI works.
> - Your mileage with the .iso files may vary depending on the ventoy support for them. Read [Ventoy compatible ISOs](https://www.ventoy.net/en/isolist.html)
> - Your mileage with non-iso files may vary even more depending on what they contain. Windows images will generally boot flawlessly. Linux images may require the image to have vtoyboot installed; read [vtoyboot](https://www.ventoy.net/en/plugin_vtoyboot.html)

Alternatively, you can use installed `yorbo.exe` cli directly. 
For booting an image (any image type), the command is:

> [!NOTE]
> Using cli requires cd'ing into the install directory for now.

```ps
./yorbo -b iso -f "FULL-PATH-TO-IMAGE"
```

This will add an entry and reboot you into the specified image upon your next boot. 
If you want to reboot immediately , you can also add the `-r` flag:

```ps
./yorbo -b iso -f "FULL-PATH-TO-IMAGE" -r 
```

### Booting Into Ventoy

Alternatively, you just want to reboot into ventoy, you can use the shortcut on your desktop.

<img width="76" height="110" alt="image" src="https://github.com/user-attachments/assets/e522bdef-69d2-4272-94e3-e7c2ced9b32f" />

If you prefer the cli, the command is:

```ps
./yorbo -b ventoy
```

or if you would like to reboot immediately,

```ps
./yorbo -b ventoy -r
```

> [!NOTE]
> Using cli requires cd'ing into the install directory for now.



