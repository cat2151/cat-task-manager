# cat-task-manager

A TUI for managing small daily recurring tasks. Operates via keyboard. Written in Rust.

*Note: Much of the following text was AI-generated and will be human-reviewed and corrected later.*

This app is not a general-purpose TODO application. It prioritizes smoothly proceeding with "the same sequence of tasks to do today" over handling projects, deadlines, priorities, tags, search, or complex dependencies.

## Philosophy

Daily tasks often fail to be sustained if their management becomes too cumbersome.
What's needed is immediate visibility of the current task, recording of the start time, the ability to move to the next task upon completion, and light operational overhead.

Therefore, cat-task-manager does not include features that encourage adding too many tasks.
Tasks are not hierarchical. They don't have parallel dependencies. They lack per-task notes or attributes.
To maintain the daily flow, it strongly emphasizes progressing through tasks sequentially.

Task definitions and their current daily status are solely managed within `tasks/*.md`. Tasks are not written in the config.
It does not maintain a separate status directory.
Prioritizing the ETC principle of frequent task maintenance, all user-editable content is consolidated into a single text file.

```text
- [ ] Morning Routine
- [ ] Check Email
- [ ] Code Review
```

With this format, when adding tasks, there's no need to replicate tables or field names.
The top-to-bottom order directly dictates the execution order, and reordering simply involves moving lines.

## Background

Like habits, we all have our quirks, especially with personal task management apps.

In this era of "vibe coding," there's no longer a need to conform your quirks to a task management app;
it's faster to vibe-code a task management app that matches your quirks.

That's why I vibe-coded this.
It's not intended for others to use.
I will frequently make breaking changes without prior notice.
Nevertheless, I hope that sharing this might serve as a hint or inspiration for someone.

## Implementation Approach

Task definitions and their current state are consolidated in `tasks/*.md`.
Regular non-empty lines serve as task definitions, and their state is read as JSON appended to the end of each task line.
The line-end JSON is an area managed by the application.

```text
- [x] Morning Routine {"date":"2026-05-19","state":"done","started_at":"2026-05-19T09:00:00+09:00","completed_at":"2026-05-19T09:05:00+09:00"}
- [ ] Check Email {"date":"2026-05-19","state":"in_progress","started_at":"2026-05-19T09:12:00+09:00"}
- [ ] Code Review {"date":"2026-05-19","state":"not_started"}
```

This is a deliberate choice to avoid duplicating state into separate files.
If you fix a typo in a task name, the state is preserved as long as the line-end JSON remains on that line.
If lines are added, deleted, or reordered, their associated state moves with them.
Tasks marked `- [x]` without line-end JSON will be normalized by adding line-end JSON with the detected time as both start and completion times.
If the line-end JSON format is broken, it will be treated as an error on startup or reload.

Historical data for statistics is read from `tasks/*.md` preserved in git history.
The guideline for the task list in the statistics screen is to exclude outliers using the IQR method from past completion records for each task, and display the average of the most frequent band in a 5-minute histogram.
If multiple bands have the same highest frequency, the median is displayed.
Task information's persistent state resides solely in `tasks/*.md`.

The screen is designed to allow focus on the next task while also permitting a full overview when needed.
Upon startup, it shows a single line; `v` toggles between single-line, uncompleted, and full views.
The uncompleted view excludes completed items, while the full view includes them.
For completed items, the work duration is displayed based on start and end times. Operable tasks are only those that are not started, in progress, on hold, or deferred.

States are kept minimal: not started, in progress, done, on hold, deferred, and timed out.
"On hold" blocks the next task, while "deferred" allows progression to the next task without blocking.
When a task is on hold in the single-line view of a specific tab, it displays a contextual message to guide switching to another tab.
If the date changes, uncompleted tasks are treated as "timed out" for that day's record.
This is a deliberate simplification, not to complicate deadlines or scheduling, but to easily record "it wasn't finished today."

Operations are largely keyboard-centric. Starting and completing use the same key, as do holding (to another tab) and resuming.
Deferring tasks is also toggled by key.
`tasks/*.md` can be opened with a key; closing it triggers a reload.
It's faster to rely on the user's familiar editor than to build forms within the app, and it's easier to verify content if something goes wrong.

## Configuration Structure

`config.toml` only holds settings for editor candidates, keybindings, and startup git snapshots. It does not contain tasks.

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

Only when `startup_git.auto_commit_and_push = true`, `%LOCALAPPDATA%\cat-task-manager` will be git committed and pushed once per day upon startup.

Save locations are consolidated under Windows' `AppData Local`.

```text
%LOCALAPPDATA%\cat-task-manager\config.toml
%LOCALAPPDATA%\cat-task-manager\tasks\tasks.md
```

`tasks/*.md` is the Single Source of Truth that both the user and the application read from and write to. `config.toml` configures the operating environment.

## Deliberate Choices

This application is built as "a place to proceed with a fixed daily routine," not "a place to manage tasks."

For flexible TODO management, search, tagging, and deadline management, other tools are recommended.
cat-task-manager is designed to start the same way every day, proceed from top to bottom, and record anything unfinished as that day's outcome.
Maintaining this simplicity is a top priority in its implementation.