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
`raise to N` sets your total bet this street to `N`. At showdown each remaining
player's hole cards are revealed.

### Card display

At launch you choose how cards are drawn: portable **text** (`A♠`, `10♦` — the
default, readable in any terminal) or Unicode **glyphs** (`🂡`). The choice is
saved with your profile.

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
