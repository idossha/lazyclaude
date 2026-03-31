# VHS tape for recording lazyclaude demo GIF
# Install VHS: brew install charmbracelet/tap/vhs
#
# Before recording, run:
#   ./demo/setup.sh
#   vhs demo/demo.tape

Output demo/lazyclaude-demo.gif
Set FontSize 14
Set Width 1400
Set Height 800
Set Theme "Catppuccin Mocha"
Set TypingSpeed 78ms
Set Padding 10

# Launch with demo data (hidden so GIF starts clean with the TUI)
Hide
Type "lazyclaude --claude-dir demo/claude-home --project-dir demo/project"
Enter
Sleep 1.95s
Show

# Panel 1: Projects — already selected
Sleep 1.3s

# Panel 2: Config — show CLAUDE.md files and rules
Type "2"
Sleep 1.95s
# Navigate down to see entries
Type "l"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 1.3s

# Panel 3: Memory — show memory files
Type "h"
Sleep 0.39s
Type "3"
Sleep 1.95s
Type "l"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 1.3s

# Panel 4: Skills — show skills and commands
Type "h"
Sleep 0.39s
Type "4"
Sleep 1.95s
Type "l"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 1.3s

# Panel 5: Agents
Type "h"
Sleep 0.39s
Type "5"
Sleep 1.95s
Type "l"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 1.3s

# Panel 6: MCP — show servers
Type "h"
Sleep 0.39s
Type "6"
Sleep 1.95s
Type "l"
Sleep 0.65s
Type "j"
Sleep 0.65s
Type "j"
Sleep 1.3s

# Panel 7: Settings — show permissions, hooks, keybindings
Type "h"
Sleep 0.39s
Type "7"
Sleep 1.95s
Type "l"
Sleep 0.65s
# Scroll through settings sections
Type "j"
Sleep 0.39s
Type "j"
Sleep 0.39s
Type "j"
Sleep 0.39s
Type "j"
Sleep 0.39s
Type "j"
Sleep 0.39s
Type "j"
Sleep 0.39s
Type "j"
Sleep 0.39s
Type "j"
Sleep 1.3s

# Panel 8: Sessions
Type "h"
Sleep 0.39s
Type "8"
Sleep 1.95s

# Panel 9: Stats — show full-year heatmap + hourly chart
Type "9"
Sleep 3.9s

# Panel 0: Plugins
Type "0"
Sleep 1.95s

# Show fuzzy filter
Type "/"
Sleep 0.65s
Type "brave"
Sleep 1.3s
Escape
Sleep 0.65s

# Back to MCP, show filter working
Type "6"
Sleep 0.65s
Type "/"
Sleep 0.39s
Type "git"
Sleep 1.3s
Escape
Sleep 0.65s

# Quit
Type "q"
Sleep 0.65s
