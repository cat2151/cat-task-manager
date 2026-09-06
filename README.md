# cat-task-manager

A TUI for managing small daily recurring tasks. Keyboard-operated. Written in Rust.

## Background

Daily task management apps, much like personal habits, vary widely.

In an age where vibe coding is possible, there's no longer a need to adapt one's habits to a task management app; it's faster to vibe code a task management app that fits one's own habits.

That's why I decided to vibe code this.
It is not intended for use by others.
I frequently make breaking changes without notice.
Nevertheless, I hope that sharing this might provide some inspiration to someone.

※Note: Much of the following text was AI-generated and may be hard to read, so I plan to manually revise it later.

## Philosophy

This application is not a general-purpose TODO app.
Rather than handling projects, deadlines, priorities, tags, search, or complex dependencies, it prioritizes seamlessly progressing through "daily, fixed sequences of tasks."

Daily tasks don't last long if their management becomes overly complex.
What's needed is immediate visibility of only the single task to be worked on now, retention of the start time, easy progression to the next task upon completion, and light user operations.

Therefore, cat-task-manager does not have features designed to encourage adding too many tasks.
Tasks are not hierarchical. They don't have parallel dependencies. They also don't have per-task notes or attributes.
To maintain the daily flow, it strongly emphasizes progressing through tasks in sequence.

Tasks and their current day's interim state are solely determined by `tasks/*.md`. Tasks are not written in the config.
It does not maintain a separate status directory.
Prioritizing frequent task maintenance as an ETC principle (Easy To Change), the user's interaction is limited to only the task description Markdown files.

```tasks.md
- [ ] 朝のルーティン
- [ ] メール確認
- [ ] コードレビュー
```

With this format, you don't need to duplicate tables or field names when adding tasks.
The top-to-bottom order directly becomes the execution order, and if you want to change the order, you can simply move lines in a text editor.

## Implementation Approach

Task definitions and their current state are consolidated in `tasks/*.md`.
Normal non-empty lines are task definitions, and the state is read as JSON at the end of each task line.
The end-of-line JSON is an area managed by the application.

```text
- [x] 朝のルーティン {"date":"2026-05-19","state":"done","started_at":"2026-05-19T09:00:00+09:00","completed_at":"2026-05-19T09:15:00+09:00","pauses":[{"paused_at":"2026-05-19T09:05:00+09:00","resumed_at":"2026-05-19T09:10:00+09:00"}]}
- [ ] メール確認 {"date":"2026-05-19","state":"in_progress","started_at":"2026-05-19T09:12:00+09:00"}
- [ ] コードレビュー {"date":"2026-05-19","state":"not_started"}
```

This is a deliberate decision to avoid duplicating state into separate files.
Even if you fix a typo in a task name, the state won't disappear as long as the end-of-line JSON for that line remains.
If lines are added, deleted, or reordered, the state moves along with the same line.
If an in-progress task is put on hold or deferred, `paused_at` is added to `pauses` in the end-of-line JSON.
When resumed, `resumed_at` is recorded in the same pause. Only the last currently paused entry will not have `resumed_at`.
If a task is deferred without starting it, no pause is added because measurement hasn't begun yet.
Tasks marked with `- [x]` but lacking end-of-line JSON will be normalized by adding end-of-line JSON with the detected time as both start and completion times.
If the end-of-line JSON format is corrupted, it will be treated as an error during startup or reload.

Statistics for past data are read from `tasks/*.md` files preserved in git history.
For the task list estimate on the statistics screen, outliers are removed from past completion records for each task using the IQR method, and the average of the modal band is displayed in a 5-minute histogram.
If multiple modal bands have the same frequency, the median is displayed.
The persistent state of task information is stored solely in `tasks/*.md`.

The screen design allows focusing on the next task while also enabling a full overview when needed.
Upon startup, it defaults to a single-line view, and `v` toggles between single-line, incomplete tasks, and full views.
The incomplete view excludes completed tasks, while the full view includes them.
For completed tasks, the actual working time (start to finish, minus all pauses) is displayed. Operations are only applicable to tasks that are not started, in progress, on hold, or deferred.

States are kept minimal: not started, in progress, completed, on hold, deferred, and timed out.
On hold blocks the next task, while deferred allows progression to the next task without blocking.
When a task is on hold in a single-line view of a specific tab, a status explanation is shown for moving to other tabs.
When the date changes, unfinished tasks are treated as timed out for that day's record.
This is a deliberate decision not to complicate deadlines or scheduling, but simply to record that "it wasn't finished on that day."

Operations are primarily keybinding-centric. Start and complete use the same key, and hold (to other tabs) and resume also use the same key to toggle.
Deferring is also toggled via key operations.
If an in-progress task is automatically put on hold when free time is manually started, it is also recorded as a pause.
`tasks/*.md` can also be opened via key operations, and upon closing, it reloads.
It's faster to rely on your usual editor than to create forms within the app, and it's easier to review content in case of errors.

## Configuration Structure

`config.toml` only contains settings for editor candidates, keybindings, startup git snapshot, and automatic free time initiation. It does not contain tasks.

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

Only when `startup_git.auto_commit_and_push = true` is set, `git commit` and `push` the `%LOCALAPPDATA%\cat-task-manager` directory once per day upon startup.

Free time, whether started manually or automatically, accumulates only within `active_hours` and stops at the end of the time slot.
Only when `auto_free_time.enabled = true` is set, if there are no in-progress tasks within `active_hours` for `idle_seconds`, free time will automatically start.
The end time is not included in the time slot. Cross-day periods like `22:00-02:00` can also be specified.

The save location is consolidated under Windows' `AppData Local`.

```text
%LOCALAPPDATA%\cat-task-manager\config.toml
%LOCALAPPDATA%\cat-task-manager\tasks\tasks.md
```

`tasks/*.md` serves as the Single Source of Truth (SSoT) that both the user and the application read from and write to. `config.toml` is for operational environment settings.

## Deliberate Decisions

This app is designed not as a place to "manage tasks," but as a place to "progress through a fixed daily routine."

For flexible TODO management, search, tagging, and deadline management, rely on other tools.
cat-task-manager starts the same way every day, proceeds in order from top to bottom, and records unfinished tasks as that day's outcome.
Maintaining this simplicity is a top implementation priority.