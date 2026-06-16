use casino_poker::agent::{AgentError, PlayerView, PokerAgent};
use casino_poker::betting::PlayerAction;
use casino_poker::casino_cards::card::{Card, Rank, Suit};
use casino_poker::games::texas_hold_em::SeatView;
use casino_poker::hand_rankings::{evaluate_holdem, EvaluatedHand, HandCategory};
use casino_poker::uuid::Uuid;

struct ExternalAgent;

impl PokerAgent for ExternalAgent {
    fn decide(&mut self, _view: &PlayerView) -> Result<PlayerAction, AgentError> {
        Err(AgentError::Failure("provider unavailable".to_owned()))
    }
}

fn classify_agent_error(error: &AgentError) -> &'static str {
    match error {
        AgentError::Quit => "quit",
        AgentError::Eof => "eof",
        AgentError::InvalidView => "invalid-view",
        AgentError::Failure(_) => "failure",
        _ => "future-error",
    }
}

#[test]
fn external_consumers_can_evaluate_serialize_and_implement_agents() {
    let hole = [
        Card::new(Rank::Ace, Suit::Heart),
        Card::new(Rank::Two, Suit::Heart),
    ];
    let board = [
        Card::new(Rank::Five, Suit::Heart),
        Card::new(Rank::Nine, Suit::Heart),
        Card::new(Rank::King, Suit::Heart),
        Card::new(Rank::King, Suit::Spade),
        Card::new(Rank::Three, Suit::Club),
    ];

    let evaluated = evaluate_holdem(hole, &board).unwrap();
    assert_eq!(evaluated.value().category(), HandCategory::Flush);
    assert_eq!(evaluated.cards().len(), 5);

    let encoded = serde_json::to_string(&evaluated).unwrap();
    let decoded: EvaluatedHand = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, evaluated);

    let mut agent = ExternalAgent;
    let view = PlayerView::builder()
        .hole(hole)
        .board(board.to_vec())
        .build();
    let error = agent.decide(&view).unwrap_err();
    assert_eq!(classify_agent_error(&error), "failure");
    assert_eq!(error.to_string(), "provider unavailable");
    let _: &dyn std::error::Error = &error;
}

#[test]
fn external_consumers_can_build_seat_views_for_player_view_tests() {
    let hero = Uuid::from_u128(1);
    let villain = Uuid::from_u128(2);
    let seats = vec![
        SeatView::builder()
            .id(hero)
            .name("Hero")
            .chips(98)
            .committed_this_street(2)
            .contributed_this_hand(2)
            .build(),
        SeatView::builder()
            .id(villain)
            .name("Villain")
            .chips(100)
            .contributed_this_hand(4)
            .folded(true)
            .all_in(true)
            .build(),
    ];

    let view = PlayerView::builder()
        .you(hero)
        .name("Hero")
        .players_remaining(2)
        .seats(seats)
        .button_seat(Some(0))
        .build();

    assert_eq!(view.seats.len(), 2);
    assert_eq!(view.button_seat, Some(0));
    assert_eq!(view.seats[0].id, hero);
    assert_eq!(view.seats[0].name, "Hero");
    assert_eq!(view.seats[0].chips, 98);
    assert_eq!(view.seats[0].committed_this_street, 2);
    assert_eq!(view.seats[0].contributed_this_hand, 2);
    assert!(!view.seats[0].folded);
    assert!(!view.seats[0].all_in);
    assert_eq!(view.seats[1].id, villain);
    assert_eq!(view.seats[1].name, "Villain");
    assert_eq!(view.seats[1].chips, 100);
    assert_eq!(view.seats[1].committed_this_street, 0);
    assert_eq!(view.seats[1].contributed_this_hand, 4);
    assert!(view.seats[1].folded);
    assert!(view.seats[1].all_in);
}
