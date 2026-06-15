# Casino

Casino is a library that provides the backend for playing various card games including poker. The library comprises a deck of cards as well as the implementations (hand ranking, betting, etc.) for the included games. The goal of this library is to provide well-tested code that can be safely relied upon for building card games on top of it.

## Crates

- [casino_cards](https://github.com/winstonrc/casino/tree/main/crates/casino_cards): A library that provides a deck of playing cards that can be used for various card games.
- [casino_poker](https://github.com/winstonrc/casino/tree/main/crates/casino_poker): A library that provides hand ranking & the backend for poker games.

## Using the library

Add `casino_poker` (which re-exports `casino_cards`) to your project and drive the
`TexasHoldEm` engine with a thin loop, supplying a `PokerAgent` per player. The
engine owns all money and rules; your code decides how each player acts. See the
[`casino_poker` README](crates/casino_poker/README.md) for a complete, runnable
example covering hand evaluation, playing a hand, and observing the event stream.

```console
$ cargo test --workspace      # run the engine + card-library test suites
$ cargo doc --no-deps --open  # browse the API docs
```

## Architecture notes

- The poker engine (`casino_poker`) owns all money and rules: per-player
  contributions, betting, all-ins, and **side pots** (computed at showdown by
  layering total contributions, so any number of unequal all-ins resolve
  correctly). Hand strength is evaluated with a kicker-correct `ComparableHand`,
  cross-checked in tests against an independent brute-force oracle.
- Players act through a `PokerAgent` trait that receives an owned `PlayerView`
  snapshot — including the opponent roster (`seats`) and button — and can `observe`
  the hero-redacted `public_events()` stream, which keys players by a stable
  `PlayerRef` (`id` + name) for per-opponent modeling. Humans and any AI — including a future model-backed
  opponent — implement the same trait, so a smarter agent can be swapped in without
  engine changes, and the owned/serializable view can be handed to an external model.
- The engine is **I/O-free**: instead of printing, it emits serializable
  `GameEvent`s to a perspective-aware `GameObserver`. Network callers use
  `public_events()` or `client_view()` for enforced private-card filtering.

## Rules simplifications

The button advances by tracking the player on it; when that player busts, the
button falls to the first remaining seat rather than implementing full
dead-button rules.
