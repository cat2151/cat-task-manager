# cat-task-manager

A TUI for managing small, daily repetitive tasks. It's keyboard-driven and written in Rust.

*Note: Much of the following text is AI-generated and will be manually refined later.*

This app is not a general-purpose TODO app. Instead of managing projects, deadlines, priorities, tags, search, or complex dependencies, it prioritizes seamlessly progressing through "the pre-defined sequence of tasks for today."

## Philosophy

Daily tasks won't stick if their management becomes too elaborate. What's needed is immediate visibility of the current task, recording the start time, seamless progression to the next task upon completion, and lightweight manual operation.

Therefore, cat-task-manager lacks features that encourage excessive task creation. Tasks are not hierarchical, do not have parallel dependencies, and lack per-task notes or attributes. To maintain a consistent daily flow, it strictly focuses on sequential task progression.

Only `tasks.txt` is considered the source of truth for tasks and their current daily status. Tasks are not stored in the config file, nor is there a separate status directory. Prioritizing frequent task maintenance as per the ETC principle, all user-editable content is consolidated into a single text file.

```text
- [ ] 朝のルーティン
- [ ] メール確認
- [ ] コードレビュー
```

With this format, you don't need to duplicate tables or field names when adding tasks. The top-to-bottom order directly dictates the execution sequence, and reordering tasks simply involves moving lines.

## Background

Everyone has their quirks, especially concerning daily task management applications.

In the modern era, where "vibe coding" is possible, there's no longer a need to adapt one's habits to a task management app. Instead, it's faster to "vibe code" a task manager tailored to one's own quirks.

So, I "vibe coded" this. It is not intended for use by others, and I will frequently make breaking changes without prior notice. Nevertheless, I hope that by sharing this, someone might find some inspiration or hints.

## Implementation Approach

Task definitions and their current status are consolidated into `tasks.txt`. Regular non-empty lines represent task definitions, with the status read as end-of-line JSON for each task. This end-of-line JSON is an application-managed area.

```text
- [x] 朝のルーティン {"date":"2026-05-19","state":"done","started_at":"2026-05-19T09:00:00+09:00","completed_at":"2026-05-19T09:05:00+09:00"}
- [ ] メール確認 {"date":"2026-05-19","state":"in_progress","started_at":"2026-05-19T09:12:00+09:00"}
- [ ] コードレビュー {"date":"2026-05-19","state":"not_started"}
```

This design is a trade-off to avoid duplicating state into a separate file. Even if a typo in a task name is corrected, the state persists as long as the end-of-line JSON for that line remains. If lines are added, deleted, or reordered, the state moves with its corresponding line. Tasks marked with `- [x]` but without end-of-line JSON are normalized by recording the detection time as both start and completion times in the JSON. If the end-of-line JSON format is corrupted, it will be treated as an error during startup or reload.

Only past records are written to the `records` directory, organized by date. These are not intended for daily operations but serve as a record of results after the day concludes. To preserve `tasks.txt` as the Single Source of Truth (SSoT), these records are not used for restoring the current day's state.

The display is designed to focus on the next task, allowing the full list to be viewed only when necessary. Upon startup, it defaults to a single-line display, and `v` toggles between single-line, incomplete tasks, and full display. The incomplete view hides completed items, while the full display includes them. For completed items, the work duration is shown based on their start and completion times. Only tasks that are not started, in progress, or on hold are interactive.

The number of states is kept minimal: Not Started, In Progress, Completed, On Hold, and Timed Out. When the date changes, any incomplete tasks are marked as "Timed Out" in that day's record. This is not intended to complicate deadlines or scheduling, but rather a deliberate simplification to easily record tasks that "weren't finished on that day."

Operations are heavily keybinding-centric. Starting and completing tasks use the same key, as do holding and resuming. `tasks.txt` can also be opened via a key command and reloaded upon closing. Rather than implementing forms within the application, it's faster and easier to inspect content (e.g., in case of errors) by relying on your preferred external editor.

## Configuration Format

`config.toml` only contains settings for editor candidates, keybindings, and startup Git snapshots. It does not store tasks.

```toml
editors = ["fresh", "zed", "nvim", "code"]

[startup_git]
auto_commit_and_push = false

[keybindings]
j = "next"
down = "next"
k = "previous"
up = "previous"
enter = "advance"
space = "advance"
p = "hold"
q = "quit"
e = "edit"
l = "next_tab"
right = "next_tab"
h = "previous_tab"
left = "previous_tab"
v = "toggle_view"
"?" = "help"
```

Only when `startup_git.auto_commit_and_push = true` is set, the `%LOCALAPPDATA%\cat-task-manager` directory will be Git committed and pushed once a day upon startup.

The storage location is consolidated under Windows' `AppData Local`.

```text
%LOCALAPPDATA%\cat-task-manager\config.toml
%LOCALAPPDATA%\cat-task-manager\tasks.txt
%LOCALAPPDATA%\cat-task-manager\records\YYYY-MM-DD.toml
```

`tasks.txt` serves as the SSoT (Single Source of Truth) that both the user and the application read from and write to. `config.toml` defines the operational environment settings. The `records` directory is treated as the output destination for historical results.

## Deliberate Simplifications

This app is not designed as "a place to manage tasks," but rather as "a place to progress through a fixed daily routine."

For flexible TODO management, searching, tagging, or deadline management, rely on other tools. cat-task-manager is designed to start the same way every day, proceed tasks from top to bottom, and record anything not finished as a result for that day. Preserving this simplicity is a top implementation priority.