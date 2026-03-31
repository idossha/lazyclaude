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
Set TypingSpeed 60ms
Set Padding 10

# Launch with demo data (hidden so GIF starts clean with the TUI)
Hide
Type "lazyclaude --claude-dir demo/claude-home --project-dir demo/project"
Enter
Sleep 1.5s
Show

# Panel 1: Projects — already selected
Sleep 1s

# Panel 2: Config — show CLAUDE.md files and rules
Type "2"
Sleep 1.5s
# Navigate down to see entries
Type "l"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 1s

# Panel 3: Memory — show memory files
Type "h"
Sleep 0.3s
Type "3"
Sleep 1.5s
Type "l"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 1s

# Panel 4: Skills — show skills and commands
Type "h"
Sleep 0.3s
Type "4"
Sleep 1.5s
Type "l"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 1s

# Panel 5: Agents
Type "h"
Sleep 0.3s
Type "5"
Sleep 1.5s
Type "l"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 1s

# Panel 6: MCP — show servers
Type "h"
Sleep 0.3s
Type "6"
Sleep 1.5s
Type "l"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 1s

# Panel 7: Settings — show permissions, hooks, keybindings
Type "h"
Sleep 0.3s
Type "7"
Sleep 1.5s
Type "l"
Sleep 0.5s
# Scroll through settings sections
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 0.3s
Type "j"
Sleep 1s

# Panel 8: Sessions
Type "h"
Sleep 0.3s
Type "8"
Sleep 1.5s

# Panel 9: Stats — show full-year heatmap + hourly chart
Type "9"
Sleep 3s

# Panel 0: Plugins
Type "0"
Sleep 1.5s

# Show fuzzy filter
Type "/"
Sleep 0.5s
Type "brave"
Sleep 1s
Escape
Sleep 0.5s

# Back to MCP, show filter working
Type "6"
Sleep 0.5s
Type "/"
Sleep 0.3s
Type "git"
Sleep 1s
Escape
Sleep 0.5s

# Quit
Type "q"
Sleep 0.5s
