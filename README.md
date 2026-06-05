# cat-task-manager

A TUI for managing small, repetitive daily tasks. Keyboard-operated. Written in Rust.

This application is not a general-purpose TODO app.
Rather than handling projects, deadlines, priorities, tags, search, or complex dependencies,
it prioritizes helping you navigate your "fixed sequence of tasks you do every day" without hesitation.

Note: Much of the following text was AI-generated and may be difficult to read. It will be manually revised later.

## Philosophy

Daily tasks tend to not stick if their management becomes too cumbersome.
What's needed is to immediately see only the one task you need to focus on now, to record its start time, to move to the next task when finished,
and for user operations to be lightweight.

Therefore, cat-task-manager does not include features designed to proliferate tasks.
Tasks are not hierarchical. They don't have parallel dependencies. They don't have notes or attributes per task.
To maintain the daily flow, it strongly emphasizes only progressing through tasks sequentially.

Only `tasks/*.md` is considered the source of truth for tasks and their current daily status. Tasks are not written in `config.toml`.
It does not maintain a separate status directory.
Prioritizing the ability to frequently maintain tasks as an ETC principle,
it narrows down what the user interacts with to only the task description Markdown files.

```tasks.md
- [ ] Morning routine
- [ ] Check emails
- [ ] Code review
```

In this format, you don't need to duplicate tables or field names when adding tasks.
The top-to-bottom order directly becomes the execution order, and if you want to change the sequence, you just move the lines in a text editor.

## Background

Like the Japanese proverb "no one is without their peculiarities," this is a task management app for daily life.

In an age where "vibe coding" has become possible,
there's no longer a need to adapt your habits to a task management app;
it's faster to "vibe code" a task management app that fits your habits instead.

That's what I thought, so I "vibe coded" this.
It's not intended for others to use.
I will frequently make breaking changes without prior notice.
Nonetheless, I hope that sharing this might provide some inspiration or hints to someone.

## Implementation Philosophy

Task definitions and current status are consolidated in `tasks/*.md`.
Normal non-empty lines are task definitions, and the status is read as JSON at the end of each task line.
The end-of-line JSON is the area where the application writes.

```text
- [x] Morning routine {"date":"2026-05-19","state":"done","started_at":"2026-05-19T09:00:00+09:00","completed_at":"2026-05-19T09:05:00+09:00"}
- [ ] Check emails {"date":"2026-05-19","state":"in_progress","started_at":"2026-05-19T09:12:00+09:00"}
- [ ] Code review {"date":"2026-05-19","state":"not_started"}
```

This is a deliberate design choice to avoid duplicating state into separate files.
Even if you correct a typo in a task name, the state remains intact as long as the end-of-line JSON for that line is preserved.
If lines are added, deleted, or reordered, the state moves along with its corresponding line.
Tasks with `- [x]` but no end-of-line JSON will be normalized by writing the detected time as both the start and completion time into the end-of-line JSON.
If the end-of-line JSON format is corrupted, it will be treated as an error during startup or reload.

Statistics for past data are read from `tasks/*.md` preserved in git history.
For the task list summary on the statistics screen, outliers are removed from past completion records for each task using the IQR method, and the average of the modal band is displayed in a 5-minute histogram.
If there are ties for the modal band, the median is displayed.
The persistent state of task information is stored solely in `tasks/*.md`.

The screen is designed to allow focus on the next task, while also enabling a full overview only when needed.
Upon startup, it defaults to a single-line view. Pressing `v` toggles between single-line, incomplete tasks, and full display.
The incomplete view excludes completed items, while the full display includes completed items.
For completed items, the work time is displayed based on the start and completion times. Operable tasks are limited to those that are not started, in progress, on hold, or deferred.

The number of states is kept small: Not Started, In Progress, Done, On Hold, Deferred, and Expired.
On Hold blocks the next task, while Deferred allows progressing to the next task without blocking.
When a task is "On Hold" in a single-line view within its tab, a contextual explanation is displayed to facilitate switching to other tabs.
When the date changes, any incomplete tasks are treated as "Expired" for that day's record.
This is not to complicate deadlines or scheduling,
but a deliberate choice to simply record that a task "was not completed on that day."

Operations are heavily keybinding-centric. Start and complete use the same key to advance, and hold (to other tabs) and resume also toggle with the same key.
Deferring tasks can also be toggled with a key.
`tasks/*.md` can also be opened via key command, and the application reloads upon closing the editor.
Entrusting editing to your familiar editor is faster than creating forms within the app, and it's easier to inspect content if something goes wrong.

## Configuration Structure

`config.toml` only holds configurations for editor candidates, keybindings, and startup git snapshots. It does not contain tasks.

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

Only when `startup_git.auto_commit_and_push = true`, the application will git commit and push `%LOCALAPPDATA%\cat-task-manager` once a day upon startup.

The save location is consolidated under Windows' `AppData Local`.

```text
%LOCALAPPDATA%\cat-task-manager\config.toml
%LOCALAPPDATA%\cat-task-manager\tasks\tasks.md
```

`tasks/*.md` is the Single Source of Truth (SSoT) that both the user and the application read from and write to. `config.toml` configures the operational environment.

## Deliberate Choices

This application is designed not as a "place to manage tasks," but as a "place to progress through a fixed daily routine."

For needs requiring flexible TODO management, search, tagging, or deadline management, other tools should be used.
cat-task-manager starts the same way every day, progresses sequentially from top to bottom, and records any uncompleted tasks as that day's outcome.
Maintaining this simplicity is a top implementation priority.