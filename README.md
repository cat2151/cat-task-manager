# cat-task-manager

A TUI for managing small, repetitive daily tasks. Keyboard operated. Written in Rust.

※From this point forward, much of the text is AI-generated and may be difficult to read; human correction is planned later.

This app is not a general-purpose TODO application. Instead of handling projects, deadlines, priorities, tags, search, or complex dependencies, it prioritizes seamlessly proceeding with "today's fixed sequence of tasks."

## Philosophy

Daily tasks won't be maintained if their management becomes overly elaborate.
What's needed is for the current task to be immediately visible, for the start time to be recorded, to move to the next task upon completion, and for on-hand operations to be lightweight.

Therefore, cat-task-manager does not include features that encourage adding too many tasks. Tasks are not hierarchical, do not have parallel dependencies, and lack task-specific notes or attributes. To maintain the daily flow, it strongly emphasizes proceeding sequentially.

Tasks and their in-progress status for the current day are solely managed within `tasks.txt`. Tasks are not written in `config`. There is no separate status directory.
Prioritizing the ability to frequently maintain tasks as an ETC principle, all user-facing data is consolidated into a single text file.

```text
- [ ] 朝のルーティン
- [ ] メール確認
- [ ] コードレビュー
```

In this format, there's no need to duplicate table or field names when adding tasks. The top-to-bottom order directly dictates the execution sequence, and if you want to change the order, you simply move the line.

## Background

Everyone has their quirks, and so do I with my daily task management apps.

In an age where "vibe coding" is possible, there's no longer a need to adapt one's quirks to a task management app. It's faster to vibe code a task management app that aligns with one's own habits.

That's why I vibe coded this. It's not intended for others to use. I will frequently make breaking changes without prior notice. Nevertheless, I hope sharing this might offer some hints to someone.

## Implementation Philosophy

Task definitions and current status are consolidated into `tasks.txt`. Regular non-empty lines are task definitions, and the status is read as JSON appended to the end of each task line. The end-of-line JSON is an area managed by the application.

```text
- [x] 朝のルーティン {"date":"2026-05-19","state":"done","started_at":"2026-05-19T09:00:00+09:00","completed_at":"2026-05-19T09:05:00+09:00"}
- [ ] メール確認 {"date":"2026-05-19","state":"in_progress","started_at":"2026-05-19T09:12:00+09:00"}
- [ ] コードレビュー {"date":"2026-05-19","state":"not_started"}
```

This is a compromise to avoid duplicating status into separate files. Even if you fix a typo in a task name, the status won't be lost as long as the end-of-line JSON for that line remains. If lines are added, deleted, or reordered, the status will move along with its respective line.
Tasks marked `- [x]` without end-of-line JSON will be normalized with the detected time as both the started and completed times in the end-of-line JSON. If the end-of-line JSON format is corrupted, it will be treated as an error during startup or reload.

Only past records are written to `records` categorized by date. These are not subject to daily operations but are the results after the day ends. To preserve `tasks.txt` as the Single Source of Truth (SSoT), these records are not used for daily recovery.

The screen is designed to allow focus on the next task while also enabling viewing the entire list when needed. Upon startup, it displays one line; `v` toggles between one-line, uncompleted, and full views. The uncompleted view hides completed tasks, while the full view shows completed tasks as well. For completed tasks, the work time is displayed based on start and end times. Only tasks that are 'not started', 'in progress', or 'on hold' are interactive.

States are kept minimal: not started, in progress, done, on hold, and timed out. If the date changes, any incomplete tasks are treated as 'timed out' in that day's record. This is a compromise not to complicate deadlines or scheduling, but simply to record that 'it wasn't finished that day.'

Operations are primarily keybinding-centric. Starting and completing tasks use the same key, and holding/resuming also toggle with the same key. `tasks.txt` can also be opened via a key command, and the app reloads upon closing the editor. Entrusting editing to a familiar external editor is faster and makes it easier to review content in case of errors, rather than creating forms within the app.

## Configuration Structure

`config.toml` only contains editor candidates and keybindings. Tasks are not written here.

```toml
editors = ["fresh", "zed", "nvim", "code"]

[keybindings]
next = "j"
previous = "k"
advance = "enter"
hold = "p"
quit = "q"
edit = "e"
next_tab = "l"
previous_tab = "h"
toggle_view = "v"
help = "?"
```

Storage locations are consolidated under Windows' `AppData Local`.

```text
%LOCALAPPDATA%\cat-task-manager\config.toml
%LOCALAPPDATA%\cat-task-manager\tasks.txt
%LOCALAPPDATA%\cat-task-manager\records\YYYY-MM-DD.toml
```

`tasks.txt` serves as the Single Source of Truth (SSoT) that both the user and the app read from and write to. `config.toml` holds operational environment settings. `records` is treated as the output destination for past results.

## Design Decisions

This app is built not as a "place to manage tasks," but as a "place to progress through a fixed daily routine."

Flexible TODO management, search, tagging, and deadline management should be handled by other tools. cat-task-manager is designed to start the same way every day, proceed from top to bottom, and record anything unfinished as that day's outcome. Maintaining this simplicity is a top implementation priority.