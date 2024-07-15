[![crates.io](https://img.shields.io/crates/v/casino_games.svg)](https://crates.io/crates/casino_games) [![Casino Test](https://github.com/winstonrc/casino/actions/workflows/casino_games.yml/badge.svg?branch=main)](https://github.com/winstonrc/casino/actions/workflows/casino_games.yml)

# casino_games

Play casino games in your terminal.

**Note:** This program is still a work-in-process and is being actively developed. Expect the usage below to change.

## Games

- Texas hold 'em

## Usage

Enter `quit` or `q` at anytime you can input text to quit the game.

```console
$ cargo run
Welcome to the casino!
Enter 'q' at anytime to quit.

Games
1. Texas hold 'em
Enter the number of the game you would like to play.
> Game: 1

**********************
* ♠ Texas hold 'em ♠ *
**********************
Tables
1. 1/3 No-limit
2. 2/5 No-limit
3. Custom
Enter the table number.
> Table: 1
> Enter your name:

Welcome Player 1! Are you happy with this name?
> yes/no [Y/n]:

You do not have enough chips to play at this table.
Current chips amount: 0
Required chips amount: 100
Additional chips needed: 100
How many chips would you like to buy?
> Amount (USD) of chips to buy: 100
```

## todo

- implement betting & folding
- add a terminal user interface (tui)
