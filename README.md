# Casino

Casino is a library that provides the backend for playing various card games including poker. The library comprises a deck of cards as well the implementations (hand ranking, betting, etc.) for the included games. The goal of this library is to provide well-tested code that can be safely relied upon for building card games on top of it.

## Crates

- [casino_games](https://github.com/winstonrc/casino/tree/main/crates/casino_games): Play casino games in your terminal.
- [casino_cards](https://github.com/winstonrc/casino/tree/main/crates/casino_cards): A library that provides a deck of playing cards that can be used for various card games.
- [casino_poker](https://github.com/winstonrc/casino/tree/main/crates/casino_poker): A library that provides hand ranking & the backend for poker games.

## Playing

```console
$ cargo run -p casino_games
```

Then pick a game from the menu (press Enter or `1` — Texas Hold'em is the only
game for now). You buy in, play hands against a mix of heuristic and loose AI
opponents, and your chip balance and stats persist between sessions.

## Architecture notes

- The poker engine (`casino_poker`) owns all money and rules: per-player
  contributions, betting, all-ins, and **side pots** (computed at showdown by
  layering total contributions, so any number of unequal all-ins resolve
  correctly). Hand strength is evaluated with a kicker-correct `ComparableHand`,
  cross-checked in tests against an independent brute-force oracle.
- Players act through a `PokerAgent` trait that receives an owned `PlayerView`
  snapshot. Humans, the heuristic AI, and any future model-backed opponent all
  implement the same trait — this is the seam for the local-model stretch goal,
  and the owned/serializable view can be handed to an external model.

## Rules simplifications

The button advances by tracking the player on it; when that player busts, the
button falls to the first remaining seat rather than implementing full
dead-button rules.
