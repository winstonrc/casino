use std::hint::black_box;

use casino_poker::casino_cards::card::{Card, Rank, Suit};
use casino_poker::hand_rankings::{evaluate_five, evaluate_holdem, evaluate_omaha};
use criterion::{criterion_group, criterion_main, Criterion};

fn card(rank: Rank, suit: Suit) -> Card {
    Card::new(rank, suit)
}

fn benchmark_hand_rankings(criterion: &mut Criterion) {
    let five = [
        card(Rank::Ace, Suit::Spade),
        card(Rank::King, Suit::Heart),
        card(Rank::Queen, Suit::Diamond),
        card(Rank::Jack, Suit::Club),
        card(Rank::Ten, Suit::Spade),
    ];
    criterion.bench_function("evaluate five cards", |bencher| {
        bencher.iter(|| evaluate_five(black_box(five)).unwrap())
    });

    let holdem_hole = [card(Rank::Ace, Suit::Spade), card(Rank::King, Suit::Spade)];
    let holdem_board = [
        card(Rank::Queen, Suit::Spade),
        card(Rank::Jack, Suit::Heart),
        card(Rank::Ten, Suit::Diamond),
        card(Rank::Nine, Suit::Club),
        card(Rank::Two, Suit::Heart),
    ];
    criterion.bench_function("evaluate seven-card holdem", |bencher| {
        bencher.iter(|| evaluate_holdem(black_box(holdem_hole), black_box(&holdem_board)).unwrap())
    });

    let omaha_hole = [
        card(Rank::Ace, Suit::Spade),
        card(Rank::King, Suit::Spade),
        card(Rank::Ace, Suit::Heart),
        card(Rank::King, Suit::Heart),
    ];
    let omaha_board = [
        card(Rank::Queen, Suit::Spade),
        card(Rank::Jack, Suit::Spade),
        card(Rank::Ten, Suit::Spade),
        card(Rank::Two, Suit::Diamond),
        card(Rank::Three, Suit::Club),
    ];
    criterion.bench_function("evaluate omaha", |bencher| {
        bencher.iter(|| evaluate_omaha(black_box(omaha_hole), black_box(&omaha_board)).unwrap())
    });

    let showdown_hands = [
        [card(Rank::Ace, Suit::Heart), card(Rank::Ace, Suit::Diamond)],
        [
            card(Rank::King, Suit::Heart),
            card(Rank::King, Suit::Diamond),
        ],
        [
            card(Rank::Queen, Suit::Heart),
            card(Rank::Queen, Suit::Diamond),
        ],
        [
            card(Rank::Jack, Suit::Heart),
            card(Rank::Jack, Suit::Diamond),
        ],
        [card(Rank::Ten, Suit::Heart), card(Rank::Ten, Suit::Diamond)],
        [
            card(Rank::Nine, Suit::Heart),
            card(Rank::Nine, Suit::Diamond),
        ],
        [
            card(Rank::Eight, Suit::Heart),
            card(Rank::Eight, Suit::Diamond),
        ],
        [
            card(Rank::Seven, Suit::Heart),
            card(Rank::Seven, Suit::Diamond),
        ],
        [card(Rank::Six, Suit::Heart), card(Rank::Six, Suit::Diamond)],
    ];
    let showdown_board = [
        card(Rank::Ace, Suit::Club),
        card(Rank::King, Suit::Club),
        card(Rank::Queen, Suit::Club),
        card(Rank::Five, Suit::Spade),
        card(Rank::Two, Suit::Spade),
    ];
    criterion.bench_function("evaluate nine-player showdown", |bencher| {
        bencher.iter(|| {
            showdown_hands
                .iter()
                .map(|hole| {
                    evaluate_holdem(black_box(*hole), black_box(&showdown_board))
                        .unwrap()
                        .value()
                })
                .max()
        })
    });
}

criterion_group!(benches, benchmark_hand_rankings);
criterion_main!(benches);
