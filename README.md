# cat-task-manager

A TUI for managing small, repetitive daily tasks. Keyboard-operated. Written in Rust.

※Note: Much of the following text was AI-generated and may be difficult to read; human revision is planned later.

This app is not a general-purpose TODO application. It prioritizes enabling users to proceed through a fixed sequence of daily tasks without hesitation, rather than handling projects, deadlines, priorities, tags, search, or complex dependencies.

## Philosophy

Daily tasks tend to fall by the wayside if their management becomes overly elaborate. What's needed is immediate visibility of the current task, a record of its start time, the ability to move to the next task upon completion, and lightweight operation.

Therefore, cat-task-manager does not include features that encourage over-populating tasks. Tasks are not hierarchical, do not have parallel dependencies, and do not have per-task notes or attributes. To maintain a smooth daily flow, it strongly emphasizes sequential task progression.

Tasks and their in-progress status for the current day are considered valid only within `tasks.txt`. Tasks are not written in the config. There is no separate status directory. Prioritizing the ability to frequently maintain tasks, adhering to the Easy To Change (ETC) principle, the system consolidates all user-facing task data into a single text file.

```text
- [ ] 朝のルーティン
- [ ] メール確認
- [ ] コードレビュー
```

This format eliminates the need to duplicate tables or field names when adding tasks. The top-to-bottom order directly defines the execution sequence, and reordering tasks simply involves moving lines.

## Background

Like the saying "Everyone has their quirks," so too with personal task management apps.

In this age of "vibe coding," there's no longer a need to adapt one's habits to a task management app; it's faster to vibe-code a task management app that fits one's own quirks.

With that in mind, I vibe-coded this. It's not intended for others to use. I will frequently make breaking changes without prior notice. Nevertheless, I hope that by sharing this, someone might find some useful hints.

## Implementation Philosophy

Task definitions and their current state are consolidated into `tasks.txt`. Regular non-empty lines are task definitions, and the state is read as JSON at the end of each task line. The end-of-line JSON is an area managed by the application.

```text
- [x] 朝のルーティン {"date":"2026-05-19","state":"done","started_at":"2026-05-19T09:00:00+09:00","completed_at":"2026-05-19T09:05:00+09:00"}
- [ ] メール確認 {"date":"2026-05-19","state":"in_progress","started_at":"2026-05-19T09:12:00+09:00"}
- [ ] コードレビュー {"date":"2026-05-19","state":"not_started"}
```

This is a deliberate decision to avoid duplicating state into separate files. If you fix a typo in a task name, the state remains intact as long as the end-of-line JSON is preserved. When lines are added, deleted, or reordered, the state moves with its respective line. Tasks marked with `- [x]` but lacking end-of-line JSON will be normalized with end-of-line JSON using the detection time as both start and completion times. If the end-of-line JSON format is broken, it will be treated as an error on startup or reload.

Only past records are written to `records` on a per-date basis. These are not subject to daily operations but rather are the results after the day ends. To maintain `tasks.txt` as the Single Source of Truth (SSoT), these records are not used for daily recovery.

The screen is designed to focus on the next task while allowing the full list to be viewed only when necessary. On startup, it defaults to a single-line view, and `v` toggles between single-line, incomplete tasks, and full display. The incomplete view excludes completed items, while the full display includes them. For completed items, the work time is displayed based on start and completion times. Only unstarted, in-progress, and on-hold tasks are subject to operations.

The number of states is kept minimal: not started, in progress, completed, on hold, and timed out. When the date changes, any incomplete tasks are marked as timed out in that day's record. This is a deliberate simplification, not to complicate deadlines or scheduling, but to simply record "not finished on that day".

Operations are primarily keybinding-centric. Start and complete use the same key, and hold and resume also toggle with the same key. `tasks.txt` can also be opened via a key command, and the app reloads upon closing the editor. Entrusting editing to a familiar external editor is faster and easier for reviewing content in case of errors, rather than building forms within the app.

## Configuration Structure

`config.toml` only holds editor candidates and keybindings. It does not contain tasks.

```toml
editors = ["fresh", "zed", "nvim", "code"]

[keybindings]
next = "j"
previous = "k"
advance = "enter"
hold = "h"
edit = "e"
toggle_view = "v"
quit = "q"
help = "?"
```

Storage locations are consolidated under Windows' `AppData Local`.

```text
%LOCALAPPDATA%\cat-task-manager\config.toml
%LOCALAPPDATA%\cat-task-manager\tasks.txt
%LOCALAPPDATA%\cat-task-manager\records\YYYY-MM-DD.toml
```

`tasks.txt` serves as the SSoT, where both the user and the app read and write the same content. `config.toml` configures the operating environment. `records` is treated as the output destination for historical results.

## Design Trade-offs

This app is designed not as a place to "manage tasks," but as a place to "progress through a fixed daily routine."

For flexible TODO management, search, tagging, or deadline management, other tools should be used. cat-task-manager is designed to start the same way every day, proceed from top to bottom, and record anything unfinished as that day's result. Preserving this simplicity is a top priority in its implementation.