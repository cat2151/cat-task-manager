# cat-task-manager

A TUI for managing small, daily recurring tasks. Keyboard-operated. Written in Rust.

## Background

Everyone has their quirks, especially when it comes to life management task apps.

In an era where "vibecoding" is possible, there's no longer a need to adapt one's quirks to a task management app. It's faster to vibecode a task management app that fits one's own habits.

That's why I vibecoded this.
It's not intended for others to use. Destructive changes will be made frequently and without notice.
Nonetheless, I hope that by sharing this, someone might gain some inspiration.

※From this point forward, many sections are AI-generated, making them less readable. Manual corrections are planned for later.

## Philosophy

This app is not a general-purpose TODO application.
Rather than handling projects, deadlines, priorities, tags, search, or complex dependencies, it prioritizes seamlessly advancing through "daily, fixed sequences of tasks."

Daily tasks tend to not stick if their management becomes overly elaborate.
What's needed is immediate visibility of only the one task to be addressed now, retention of the start time, easy progression to the next task upon completion, and light user interaction.

Therefore, cat-task-manager does not possess features designed to encourage adding too many tasks.
Tasks are not hierarchical. They don't have parallel dependencies. They also lack individual notes or attributes.
To maintain the daily flow, it strongly emphasizes only sequential progression.

Tasks and their current-day interim states are exclusively defined by `tasks/*.md`. Tasks are not written in the config file.
It does not maintain a separate status directory.
Prioritizing frequent task maintenance as per the ETC principle (Easy To Change), the scope of user interaction is limited solely to the task description Markdown files.

```tasks.md
- [ ] 朝のルーティン
- [ ] メール確認
- [ ] コードレビュー
```

This format eliminates the need to duplicate tables or field names when adding tasks. The top-to-bottom order directly dictates execution sequence, and reordering simply involves moving lines in a text editor.

## Implementation Approach

Task definitions and current states are consolidated in `tasks/*.md`. Regular non-empty lines serve as task definitions, and the state is read as a JSON object at the end of each task line. The end-of-line JSON is an area managed by the application.

```text
- [x] 朝のルーティン {"date":"2026-05-19","state":"done","started_at":"2026-05-19T09:00:00+09:00","completed_at":"2026-05-19T09:05:00+09:00"}
- [ ] メール確認 {"date":"2026-05-19","state":"in_progress","started_at":"2026-05-19T09:12:00+09:00"}
- [ ] コードレビュー {"date":"2026-05-19","state":"not_started"}
```

This is a deliberate design choice to avoid duplicating state into separate files.
Even if you correct a typo in a task name, the state remains intact as long as the end-of-line JSON for that line is preserved.
If lines are added, deleted, or reordered, the state moves along with its respective line.
Tasks marked with `- [x]` but lacking end-of-line JSON are normalized by setting the detected time as both the start and completion times in the end-of-line JSON.
If the end-of-line JSON format is corrupted, it is treated as an error during startup or reload.

Statistics for historical data are read from `tasks/*.md` retained in the Git history.
For task list estimates on the statistics screen, outliers are removed from past completion records for each task using the IQR method, and the average of the modal band is displayed in a 5-minute histogram. If multiple modal bands have the same frequency, the median is displayed.
The persistent state of task information is stored solely in `tasks/*.md`.

The display is designed to allow users to focus on the next task while still being able to view the overall list when needed.
Upon startup, it defaults to a single-line display; `v` toggles between single-line, incomplete, and full displays.
The incomplete display excludes completed items, while the full display includes them.
For completed items, the working time is shown based on start and completion times. Operable tasks are limited to those that are not started, in progress, on hold, or deferred.

States are kept minimal: Not Started, In Progress, Completed, On Hold, Deferred, and Expired.
On hold tasks block the next task, while deferred tasks allow progression to the next task without blocking.
When a task is on hold in a single-line view within its individual tab, a status description is shown to explain why it might be necessary to switch to another tab.
When the date changes, uncompleted tasks are treated as 'expired' for that day's record.
This is a deliberate design choice not to complicate deadlines or scheduling, but simply to record that a task "wasn't finished on that day."

Operations are primarily keyboard-centric. Starting and completing tasks use the same key for progression; putting tasks on hold (moving to another tab) and resuming also use the same key for toggling.
Deferring tasks is also toggled via key commands.
`tasks/*.md` can also be opened via a key command, and reloaded upon closing.
Entrusting editing to the user's preferred editor is faster and makes it easier to inspect contents in case of error, compared to building forms within the app itself.

## Configuration Structure

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

Only when `startup_git.auto_commit_and_push = true`, the application will perform a `git commit` and `push` of `%LOCALAPPDATA%\cat-task-manager` once a day upon startup.

Storage location is consolidated under Windows' `AppData Local` directory.

```text
%LOCALAPPDATA%\cat-task-manager\config.toml
%LOCALAPPDATA%\cat-task-manager\tasks\tasks.md
```

`tasks/*.md` serves as the Single Source of Truth (SSoT) that both the user and the application read from and write to. `config.toml` provides operational environment settings.

## Design Choices

This application is built not as a "place to manage tasks," but as a "place to progress through fixed daily routines."

For flexible TODO management, searching, tagging, and deadline management, other tools should be used.
cat-task-manager is designed to start the same way every day, proceed from top to bottom, and record anything unfinished as that day's outcome.
Maintaining this simplicity is a top priority in its implementation.