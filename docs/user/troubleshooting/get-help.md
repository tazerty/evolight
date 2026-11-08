---
title: Get help with a problem
description: What to include when you ask for help with a EvoLight bug, and where to post it.
order: 30
verified: 0.7.1
---

Open an issue in the [issue tracker](https://github.com/tazerty/evolight/issues), or ask in [Discord](https://evora.st). Include your version, your system, what you did, and what you expected.

## What to include

1. **Your version.** Choose **Help** > **About EvoLight** (on macOS, **EvoLight** > **About EvoLight**).
2. **Your system.** For example Windows 11, 64-bit. **Help** > **System Info…** shows details about your computer.
3. **The steps.** What you clicked, in order, and what happened.
4. **The camera and file type**, if the problem is with a photo.
5. **A sample file**, if a raw or other photo renders wrong. Maintainers ask for the original file, not an export. A link to a shared folder works. Uploading raws to [raw.pixls.us](https://raw.pixls.us) also helps, because every file there becomes a permanent test sample for that camera.
6. **A screenshot** of EvoLight next to the camera JPEG or the other app, for colour problems.
7. **The log file.** Choose **Help** > **Open Log Folder** and attach `evolight.log`.

## Where the log is

EvoLight writes its log to `logs/evolight.log` in its settings folder. On Windows that is `%APPDATA%\EvoLight\logs\`. Each start moves the previous log to `evolight.1.log`, and that one to `evolight.2.log`, so the log of a run that crashed survives the next start.

## A more detailed log

When thumbnails or previews don't appear, appear slowly, or keep the computer busy, a log of every picture EvoLight draws helps. Start EvoLight with the environment variable `RUST_LOG` set to `warn,evolight*=info,evolight_ui_egui::render=debug`. On Windows, in PowerShell:

```powershell
$env:RUST_LOG = "warn,evolight*=info,evolight_ui_egui::render=debug"; & "C:\Program Files\EvoLight\evolight.exe"
```

The log then has a line for each picture EvoLight asks for, with its file, and usually a line for how it ended: drawn, or dropped and why (for example, scrolled out of view, or replaced by a newer request). A few requests that are dropped while they wait, such as when photos are removed or EvoLight quits, have no ending line. A file that can't be read is still a warning. So the log shows whether a file was asked for once or many times. `EVOLIGHT_LOG=debug` instead turns on the detailed log of all of EvoLight.

This log names the files EvoLight draws, with their folders. Look through it before you share it, and remove any names you'd rather keep private.

## Still stuck?

Ask in [Discord](https://evora.st).
