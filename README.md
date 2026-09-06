# cat-task-manager

A TUI for managing small, daily recurring tasks. Keyboard-operated. Written in Rust.

## Background

Just as everyone has their own habits, this applies to daily life task management apps.

In this era of "vibe coding," there's no longer a need to adapt one's habits to a task management app. It's faster to vibe code a task management app that fits one's own habits.

That's what I thought, so I vibe coded it. It is not intended for use by others. Breaking changes will be made frequently and without prior notice. That said, I hope that sharing this might provide some kind of hint or inspiration to someone.

※Note: The following sections contain a lot of AI-generated text, which can be hard to read. I plan to manually revise them later.

## Philosophy

This application is not a general-purpose TODO app.
Rather than handling projects, deadlines, priorities, tags, search, or complex dependencies, it prioritizes smoothly progressing through "the same sequence of tasks to be done today" without hesitation.

Daily tasks don't stick if their management becomes overly elaborate. What's needed is immediate visibility of the single task currently in focus, a record of the start time, the ability to move to the next task upon completion, and lightweight user interaction.

Therefore, cat-task-manager does not have features that encourage adding too many tasks. Tasks are not hierarchical. They don't have parallel dependencies. They don't have per-task notes or attributes. To maintain the daily flow, it strongly emphasizes only sequential progression.

Tasks and their current-day intermediate states are exclusively defined in `tasks/*.md`. Tasks are not written in the config. It does not have a separate status directory. It prioritizes frequent task maintenance based on the ETC principle, limiting user interaction to only the task description Markdown files.

```tasks.md
- [ ] Morning Routine
- [ ] Check Email
- [ ] Code Review
```

With this format, there's no need to duplicate tables or field names when adding tasks. The top-to-bottom order directly dictates the execution sequence, and if you want to change the order, you can simply move lines in a text editor.

## Implementation Philosophy

Task definitions and current states are consolidated in `tasks/*.md`. Normal non-empty lines are task definitions, and the state is read as JSON at the end of each task line. The JSON at the end of the line is the application's writing area.

```text
- [x] Morning Routine {"date":"2026-05-19","state":"done","started_at":"2026-05-19T09:00:00+09:00","completed_at":"2026-05-19T09:05:00+09:00"}
- [ ] Check Email {"date":"2026-05-19","state":"in_progress","started_at":"2026-05-19T09:12:00+09:00"}
- [ ] Code Review {"date":"2026-05-19","state":"not_started"}
```

This is a compromise to avoid duplicating state into separate files. Even if you fix a typo in a task name, the state remains intact as long as the JSON at the end of that line is preserved. If lines are added, deleted, or reordered, the state moves along with the same line. Tasks marked with `- [x]` but lacking end-of-line JSON will be normalized by the application, setting the detected time as both the start and completion time in the end-of-line JSON. If the end-of-line JSON format is corrupted, it will be treated as an error during startup or reload.

Historical data statistics are read from `tasks/*.md` files preserved in git history. For the task list estimate on the statistics screen, outliers are removed using the IQR method from past completion records for each task, and the average of the most frequent bandwidth is displayed in a 5-minute histogram. If multiple bandwidths tie for the most frequent, the median is displayed. The persistent state of task information is stored solely in `tasks/*.md`.

The screen is designed to focus on the next task while allowing the full list to be checked only when necessary. Upon startup, it displays a single line. Pressing `v` toggles between single line, incomplete, and full views. The incomplete view excludes completed items, while the full view displays all items, including completed ones. For completed items, the work duration is displayed based on start and completion times. Operable items are limited to those that are not started, in progress, on hold, or deferred.

States are kept minimal: Not Started, In Progress, Completed, On Hold, Deferred, and Timed Out. On Hold blocks the next task, while Deferred allows progression to the next task without blocking. When a single-line view in an individual tab is on hold, it displays a status explanation to facilitate moving to other tabs. When the date changes, unfinished tasks are treated as 'timed out' in that day's record. This is not to complicate deadlines or scheduling, but rather a compromise to simply record "what wasn't finished on that day."

Operations are primarily keyboard-centric. Starting and completing use the same key, and holding (to other tabs) and resuming also toggle with the same key. Deferring is also toggled with a key press. `tasks/*.md` can also be opened via a key press, and the app reloads when closed. It's faster to delegate task editing to your usual text editor rather than building forms within the app, and it's easier to inspect content in case of errors.

## Configuration Structure

`config.toml` only contains settings for editor candidates, keybindings, startup Git snapshot, and automatic free time start. It does not contain tasks.

```toml
editors = ["fresh", "zed", "nvim", "code"]

[startup_git]
auto_commit_and_push = false

[auto_free_time]
enabled = false
idle_seconds = 60
active_hours = "09:00-17:00"

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

Only when `startup_git.auto_commit_and_push = true` is set, `git commit` and `push` to `%LOCALAPPDATA%\cat-task-manager` once a day upon startup.

Free time, whether started manually or automatically, only accumulates within `active_hours` and stops at the end of that time period. Only when `auto_free_time.enabled = true` is set, if there is no in-progress task within `active_hours` for `idle_seconds`, free time automatically starts. The end time is not included in the time period. Cross-day ranges like `22:00-02:00` can also be specified.

Save locations are consolidated under Windows' `AppData Local`.

```text
%LOCALAPPDATA%\cat-task-manager\config.toml
%LOCALAPPDATA%\cat-task-manager\tasks\tasks.md
```

`tasks/*.md` is the Single Source of Truth (SSoT) that both the user and the app read and write to. `config.toml` contains settings for the operating environment.

## Deliberate Choices

This app is designed not as a "place to manage tasks," but as a "place to progress through a fixed daily routine."

For flexible TODO management, search, tagging, and deadline management, please use other tools. cat-task-manager starts the same way every day, progresses from top to bottom, and records anything unfinished as that day's outcome. Maintaining this simplicity is a top implementation priority.