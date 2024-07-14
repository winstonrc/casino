[![Poker Test](https://github.com/winstonrc/casino/actions/workflows/casino_poker.yml/badge.svg?branch=main)](https://github.com/winstonrc/casino/actions/workflows/casino_poker.yml)

# Poker

A library that implements the backend for playing poker games including Texas hold 'em. This includes tested hand ranking functions.

**Note:** This library is still a work-in-process. Following the usage code below will result in a full game being simulated without user interaction. This includes a lack of betting or folding hands, which are currently being worked on.

## Todo

- Implement betting & folding
- Add computer opponent ai
- Implement [limit](https://en.wikipedia.org/wiki/Betting_in_poker#Fixed_limit)/[no-limit](https://en.wikipedia.org/wiki/Betting_in_poker#No_limit) logic
- Add training tools like calculating [pot odds](https://en.wikipedia.org/wiki/Pot_odds)

## Usage

### Texas hold 'em

```rust
use poker::games::texas_hold_em::TexasHoldEm;

const MINIMUM_TABLE_BUY_IN_CHIPS_AMOUNT: u32 = 100;
const MAXIMUM_TABLE_PLAYERS: usize = 10;
const SMALL_BLIND: u32 = 1;
const BIG_BLIND: u32 = 3;
const LIMIT: bool = false;

fn main() {
    let mut texas_hold_em_1_3_no_limit = TexasHoldEm::new(
        MINIMUM_TABLE_BUY_IN_CHIPS_AMOUNT,
        MAXIMUM_TABLE_PLAYERS,
        SMALL_BLIND,
        BIG_BLIND,
        LIMIT,
    );

    // A Player can be created without chips.
    let mut player1 = texas_hold_em_1_3_no_limit.new_player("Player 1");

    // But the Player must have the minimum_table_buy_in_chips_amount before they can be added to the table.
    player1.add_chips(100);

    // add_player() returns a Result, which can be handled.
    match texas_hold_em_1_3_no_limit.add_player(player1) {
        Ok(()) => {}
        Err("The player does not have enough chips to play at this table.") => {
            eprintln!("The player does not have enough chips to play at this table.")
        }
        Err(_) => {
            eprintln!("Unable to add player to the table. Reason unknown.");
        }
    }

    // A Player can also be created with chips.
    let player2 = texas_hold_em_1_3_no_limit.new_player_with_chips("Player 2", 100);

    // You can try to add a player without handling the result.
    texas_hold_em_1_3_no_limit.add_player(player2).unwrap();

    // A tournament can be played, which iterates through rounds until there is only one player remaining.
    texas_hold_em_1_3_no_limit.play_tournament();

    // Or a single round can be run.
    // The dealer's seat index must be provided in order to determine the order of dealing and the small and big blinds.
    let dealer_seat_index: usize = 0;
    texas_hold_em_1_3_no_limit.play_round(dealer_seat_index);
}
```
