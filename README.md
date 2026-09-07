# cat-task-manager

A TUI for managing small, recurring daily tasks. Keyboard-operated. Written in Rust.

## Background

Just as everyone has their own quirks, so do daily task management apps.

In an era where "vibe coding" is possible, there's no longer a need to adapt one's habits to a task management app; it's faster to vibe-code a task management app that fits one's own habits.

That's why I vibe-coded this. It is not intended for use by others. Frequent destructive changes will be made without notice. Nevertheless, I hope that sharing this might provide some hints or inspiration to someone.

※Note: The following sections contain a lot of AI-generated text, which can be hard to read, so I plan to revise them manually later.

## Philosophy

This app is not a general-purpose TODO application. Rather than handling projects, deadlines, priorities, tags, searches, or complex dependencies, it prioritizes seamlessly progressing through "daily, fixed sequences of tasks."

Daily tasks are less likely to stick if their management becomes overly elaborate. What's needed is the immediate visibility of the single task to focus on, recording the start time, advancing to the next task upon completion, and minimal user interaction.

Therefore, cat-task-manager does not have features to encourage adding too many tasks. Tasks are not hierarchical. They have no parallel dependencies. They also lack per-task notes or attributes. To maintain the daily flow, it strongly emphasizes only progressing in sequence.

The `tasks/*.md` files are the sole source of truth for tasks and their current daily status. Tasks are not written in the config. It does not maintain a separate status directory. Prioritizing frequent task maintenance as per the ETC (Easy To Change) principle, it narrows the scope of user interaction to only the task definition Markdown files.

```tasks.md
- [ ] Morning routine
- [ ] Check email
- [ ] Code review
```

In this format, there's no need to duplicate tables or field names when adding tasks. The top-to-bottom order directly dictates the execution sequence, and to change the order, one simply moves lines in a text editor.

## Implementation Details

Task definitions and current status are consolidated in `tasks/*.md`. Normal non-empty lines are task definitions, and the state is read as JSON at the end of each task line. The JSON at the end of the line is an area written by the application.

```text
- [x] 朝のルーティン {"date":"2026-05-19","state":"done","started_at":"2026-05-19T09:00:00+09:00","completed_at":"2026-05-19T09:15:00+09:00","pauses":[{"paused_at":"2026-05-19T09:05:00+09:00","resumed_at":"2026-05-19T09:10:00+09:00"}]}
- [ ] メール確認 {"date":"2026-05-19","state":"in_progress","started_at":"2026-05-19T09:12:00+09:00"}
- [ ] コードレビュー {"date":"2026-05-19","state":"not_started"}
```

This is a compromise to avoid duplicating the state into separate files. Even if a typo in a task name is corrected, the state remains intact as long as the line-end JSON is preserved. If lines are added, deleted, or reordered, the state moves along with its respective line. When an in-progress task is put on hold or deferred, `paused_at` is added to the `pauses` array in the line-end JSON. Upon resuming, `resumed_at` is recorded in the same pause entry. Only the last currently paused entry will not have a `resumed_at`. If a task is deferred without being started, no pause is added as tracking hasn't begun. For tasks marked `- [x]` without line-end JSON, the system normalizes them by adding line-end JSON with the detected time as both start and completion times. If the line-end JSON format is corrupted, it will be treated as an error during startup or reload.

Historical task statistics are read from `tasks/*.md` preserved in the Git history. On the statistics screen, the task list benchmark is determined by filtering outliers from past completion records for each task using the IQR method, then displaying the average of the modal band from a 5-minute histogram. If multiple modal bands have the same frequency, the median is displayed. The persistent state of task information resides solely in `tasks/*.md`.

The screen is designed to allow focusing on the next task while also enabling a full overview when needed. Upon startup, it defaults to a single-line view, with `v` toggling between single-line, uncompleted, and full views. The uncompleted view excludes finished tasks, while the full view includes them. For completed tasks, the actual working time (start to completion minus all pauses) is displayed. Operations are only permitted on tasks that are not started, in progress, on hold, or deferred.

The number of states is kept small: Not Started, In Progress, Done, On Hold, Deferred, and Timed Out. On hold blocks the next task, whereas deferred allows proceeding to the next task without blocking. When a task is on hold in a single-line view of a specific tab, a situational explanation is displayed to facilitate moving to other tabs. When the date changes, any unfinished tasks are treated as 'timed out' for that day's record. This is not to complicate deadlines or scheduling, but a compromise to simply record that a task "was not completed on that day."

Operations are primarily keybinding-centric. Starting and completing use the same key, as do holding (to other tabs) and resuming. Deferring is also toggled with key operations. If an in-progress task is automatically put on hold when free time is manually started, this is also recorded as a pause. `tasks/*.md` can also be opened via key command, and reloaded upon closing. It's faster to rely on one's usual editor than to build forms within the app, and easier to inspect content in case of errors.

## Configuration

`config.toml` holds settings for editor candidates, keybindings, startup Git snapshots, automatic free time initiation, and external event notifications. It does not contain tasks.

```toml
editors = ["fresh", "zed", "nvim", "code"]

[startup_git]
auto_commit_and_push = false

[auto_free_time]
enabled = false
idle_seconds = 60
active_hours = "09:00-17:00"

[external_event]
# interface_file = "C:/path/to/external-task-event.toml"

[keybindings]
j = "next"
down = "next"
k = "previous"
up = "previous"
enter = "advance"
space = "advance"
p = "hold"
d = "defer"
q = "quit"
e = "edit"
l = "next_tab"
right = "next_tab"
h = "previous_tab"
left = "previous_tab"
v = "toggle_view"
s = "stats"
"?" = "help"
```

Only when `startup_git.auto_commit_and_push = true`, the `%LOCALAPPDATA%\cat-task-manager` directory will be Git committed and pushed once a day upon startup.

Free time, whether started manually or automatically, accumulates only within `active_hours` and stops at the end of the time slot. Only when `auto_free_time.enabled = true`, if there are no in-progress tasks within `active_hours` for `idle_seconds`, free time will start automatically. The end time is not included in the time slot. Cross-day periods like `22:00-02:00` can also be specified.

If `external_event.interface_file` is specified, task start/completion events triggered by key operations will be notified to a TOML file. Notifications are atomically replaced after `tasks/*.md` is successfully saved, retaining only the latest single event. This TOML is a transient event for external applications and is not the SSoT (Single Source of Truth) for task status.

Storage locations are consolidated under Windows' `AppData Local`.

```text
%LOCALAPPDATA%\cat-task-manager\config.toml
%LOCALAPPDATA%\cat-task-manager\tasks\tasks.md
```

`tasks/*.md` is the SSoT that both the user and the app read and write to. `config.toml` contains settings for the operating environment.

## Trade-offs

This application is built not as a "place to manage tasks," but as a "place to progress through fixed daily routines."

For flexible TODO management, search, tagging, and deadline management, other tools should be used. cat-task-manager starts the same way every day, progresses from top to bottom, and leaves unfinished tasks as part of that day's record. Maintaining this simplicity is a top implementation priority.