---
name: standup
description: Generate a standup summary from recent git activity
---

Look at `git log --oneline --since="yesterday"` and summarize:
- What was done (completed commits)
- What's in progress (uncommitted changes)
- Any blockers (failed tests, TODO comments)

Format as a brief standup update.
