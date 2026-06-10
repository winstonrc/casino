[![crates.io](https://img.shields.io/crates/v/casino_games.svg)](https://crates.io/crates/casino_games) [![Casino Test](https://github.com/winstonrc/casino/actions/workflows/casino_games.yml/badge.svg?branch=main)](https://github.com/winstonrc/casino/actions/workflows/casino_games.yml)

# casino_games

Play casino games in your terminal.

**Note:** This program is still a work-in-progress and is being actively developed. Expect the usage below to change.

## Games

#### Current Games:

- Texas hold 'em

#### Planned Games:

- Blackjack

## Usage

Run the game and pick a game by number (or just press Enter — there's only one
for now). Enter `quit` or `q` at any prompt to quit.

```console
$ cargo run -p casino_games
Welcome to the casino!
Enter 'q' at anytime to quit.

Games
1. Texas hold 'em
Select a game by number, or press Enter to play Texas hold 'em.
Game: 1

**********************
* ♠ Texas hold 'em ♠ *
**********************
Tables
1. 1/3 No-limit
2. 2/5 No-limit
3. Custom
Table: 1
Enter your name: Alice
...
```

On your turn you choose an action by a single-letter shortcut or its full word:
`(f)old`, `(x) check`, `(c)all`, `(r)aise to <amount>`, `(a)ll-in` (check uses
`x` so `c` is always call). The available actions and amounts are shown each turn;
`raise to N` sets your total bet this street to `N`.

### Hand history output

Each hand is narrated to **stdout** as a **PokerStars-format hand history** —
header, seats, blinds, `*** HOLE CARDS ***`, action lines, street markers,
`*** SHOW DOWN ***`, and `*** SUMMARY ***`. In the default text card mode the
cards are PokerStars codes (`As`, `Td`), so the output is a real hand history that
standard tools (PokerTracker, Hold'em Manager, open-source parsers) can ingest; in
glyph mode the same layout prints with Unicode card glyphs (`🂡`) as flair.

Interactive prompts (your turn, setup, the leaderboard) are written to **stderr**,
so they stay out of the history. Redirect stdout to capture a clean log:

```console
$ cargo run -p casino_games > session.txt   # session.txt is parseable hand history
$ cargo run -p casino_games | tee session.txt   # play normally AND save a clean copy
```

Each session is also **saved automatically** to a timestamped file in the data
directory — `~/.local/share/casino/history/<YYYY-MM-DD_HH-MM-SS>.txt` on Linux —
so you always have a clean history without redirecting (the path is printed at
launch). One file per session keeps each log small and easy to prune, and the file
is created only when the first hand is dealt (quitting at setup leaves nothing
behind). The saved log is **always** written with parseable PokerStars card codes,
even if you play with glyph cards on screen — the file is for tooling.

```text
PokerStars Hand #1: Hold'em No Limit (1/3) - 2026/06/10 14:32:01 ET
Table 'Casino' 6-max Seat #1 is the button
Seat 1: Alice (190 in chips)
...
*** HOLE CARDS ***
Dealt to Alice [Kd Js]
Alice: raises 6 to 9
...
*** SHOW DOWN ***
Player 6: shows [Jc Tc] (two pair, Jacks and Fives)
Player 6 collected 55 from pot
*** SUMMARY ***
Total pot 55 | Rake 0
Board [5d 5c 2s Qh Jd]
```

### Card display

At launch you choose how cards are drawn: parseable **text** (`As`, `Td` — the
default, PokerStars codes so the history is machine-readable) or Unicode
**glyphs** (`🂡` — prettier flair, not parseable). The choice is saved with your
profile.

The glyphs live in Unicode's *Playing Cards* block (U+1F0A0–U+1F0FF), which many
terminal fonts render tiny or not at all. For them to look right your terminal
needs a font with good coverage of that block, for example:

- **Noto Sans Symbols 2** (Linux: the `noto-fonts-extra` / `ttf-noto-symbols`
  package) — the most reliable cross-platform option.
- **Symbola**.
- OS symbol fonts — *Segoe UI Symbol* (Windows), *Apple Symbols* (macOS).

Configure your terminal to use (or fall back to) one of these. If the glyphs
still look too small, stick with the text option.

### Opponents

Computer opponents implement the `PokerAgent` trait from `casino_poker`. Most
opponents play a strength-aware heuristic that evaluates their hole cards and the
board and weighs the result against the pot odds; one "loose" opponent plays more
randomly for variety. Because everything goes through `PokerAgent`, a future
model-backed opponent can be dropped in without engine changes.

### Saved progress

Your name, chip balance, and basic stats (hands played and won) are saved as JSON
in the platform's standard data directory (e.g. `~/.local/share/casino/profile.json`
on Linux) after every hand. On launch you're offered the chance to resume.

## Todo

- richer opponent AI (e.g. plug in a small local model via `PokerAgent`)
- add a terminal user interface (tui)
- online multiplayer
