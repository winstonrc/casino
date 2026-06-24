use casino_poker::agent::{AgentError, PlayerView, PokerAgent};
use casino_poker::betting::{LegalAction, PlayerAction};
use casino_poker::casino_cards::card::{Card, Rank, Suit};
use casino_poker::events::GameEvent;
use casino_poker::games::texas_hold_em::{
    HandEventCursor, HandProgressStep, SeatView, TexasHoldEm,
};
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

fn choose_public_action(legal_actions: &[LegalAction]) -> PlayerAction {
    if legal_actions
        .iter()
        .any(|action| matches!(action, LegalAction::Check))
    {
        PlayerAction::Check
    } else if legal_actions
        .iter()
        .any(|action| matches!(action, LegalAction::Call(_)))
    {
        PlayerAction::Call
    } else {
        PlayerAction::Fold
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

#[test]
fn external_consumers_can_drive_public_hand_progress_without_leaking_prompts() {
    let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 42);
    for name in ["Ada", "Bert", "Cyd"] {
        let player = game.new_player_with_chips(name, 100);
        game.add_player(player).unwrap();
    }

    let mut cursor = HandEventCursor::default();
    let first_step = game.drive_hand_progress(&mut cursor);
    assert!(matches!(
        first_step,
        HandProgressStep::Event(GameEvent::HandStarted { .. })
    ));

    let encoded_cursor = serde_json::to_string(&cursor).unwrap();
    let mut cursor: HandEventCursor = serde_json::from_str(&encoded_cursor).unwrap();
    assert_eq!(cursor.next_event(), 1);
    let mut public_stream = Vec::new();

    let (actor, decision_id) = loop {
        match game.drive_hand_progress(&mut cursor) {
            HandProgressStep::Event(GameEvent::HoleCardsDealt { hero }) => {
                assert!(
                    hero.is_none(),
                    "progress events use the public/redacted event stream"
                );
                public_stream.push(GameEvent::HoleCardsDealt { hero });
            }
            HandProgressStep::Event(event) => public_stream.push(event),
            HandProgressStep::AwaitingPlayer {
                player,
                decision_id,
            } => break (player, decision_id),
            HandProgressStep::HandComplete => panic!("hand should await a preflop action"),
            HandProgressStep::CannotStart(error) => panic!("hand should start: {error}"),
            _ => panic!("unexpected future hand progress step"),
        }
    };

    let pending = game
        .pending_action()
        .expect("the private prompt is available through pending_action");
    assert_eq!(pending.player, actor);
    assert_eq!(pending.decision_id, decision_id);
    assert_eq!(pending.view.you, actor);
    assert_eq!(pending.view.hole.len(), 2);

    let actor_view = game.client_view(actor);
    let actor_prompt = actor_view
        .pending_action
        .expect("the actor can fetch their private prompt");
    assert_eq!(actor_prompt.decision_id, decision_id);
    assert!(actor_view.hole.is_some());

    let spectator = game
        .seats()
        .iter()
        .copied()
        .find(|&player| player != actor)
        .unwrap();
    let spectator_view = game.client_view(spectator);
    assert!(spectator_view.pending_action.is_none());
    assert!(spectator_view.hole.is_some());

    let action = choose_public_action(&actor_prompt.view.legal_actions);
    let next = game
        .submit_hand_progress_action(&mut cursor, actor, decision_id, action)
        .unwrap();
    match next {
        HandProgressStep::Event(event) => {
            if let GameEvent::ActionTaken { player, .. } = &event {
                assert_eq!(player.id, actor);
            }
            public_stream.push(event);
        }
        HandProgressStep::AwaitingPlayer { .. } | HandProgressStep::HandComplete => {}
        HandProgressStep::CannotStart(error) => panic!("hand should continue: {error}"),
        _ => panic!("unexpected future hand progress step"),
    }
    assert!(
        public_stream.iter().any(
            |event| matches!(event, GameEvent::ActionTaken { player, .. } if player.id == actor)
        ),
        "consumers must preserve the post-submit step returned by submit_hand_progress_action"
    );
}
