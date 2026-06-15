use casino_poker::agent::{AgentError, PlayerView, PokerAgent};
use casino_poker::betting::PlayerAction;
use casino_poker::casino_cards::card::{Card, Rank, Suit};
use casino_poker::hand_rankings::{evaluate_holdem, EvaluatedHand, HandCategory};

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
