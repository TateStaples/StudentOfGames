/// Interactive Liar's Dice game with heuristic AI
///
/// Players: Human (P1) vs AI (P2)
/// Dice: 5 per player, 6-sided
/// Variant: Joker (1 is wild)

use std::io::{self, Write};
use std::env;
use StudentOfGames::games::liars_die::{LiarsDie, LiarsDieAction, Die};
use StudentOfGames::utils::{Game, Player};

fn format_die(d: &Die) -> &'static str {
    match d {
        Die::One => "1",
        Die::Two => "2",
        Die::Three => "3",
        Die::Four => "4",
        Die::Five => "5",
        Die::Six => "6",
    }
}

fn format_action(a: &LiarsDieAction) -> String {
    match a {
        LiarsDieAction::BullShit => "📣 Call Bullshit!".to_string(),
        LiarsDieAction::Deal(_, _) => "🎲 Deal (new game)".to_string(),
        LiarsDieAction::Raise(face, count) => {
            format!("🎯 Bid {}×{}", count, format_die(face))
        }
    }
}

fn print_history(state: &LiarsDie, player: Player) {
    let trace = state.trace(player);
    if trace.bet_history.is_empty() {
        println!("     (no bets yet)");
    } else {
        for (i, action) in trace.bet_history.iter().enumerate() {
            let actor = if i % 2 == 0 { "P1" } else { "P2" };
            println!("     {}: [{}] {}", i + 1, actor, format_action(action));
        }
    }
}

fn score_action(action: &LiarsDieAction, bet_count: usize) -> f32 {
    match action {
        LiarsDieAction::BullShit => {
            // Increasingly attractive after many bets
            if bet_count >= 4 {
                if bet_count % 3 == 0 {
                    10.0 // Very high score when we want to call BS
                } else {
                    2.0
                }
            } else {
                0.5
            }
        }
        LiarsDieAction::Raise(_, count) => {
            // Prefer raises with count <= 5
            if *count <= 5 {
                5.0 + (5.0 - *count as f32) // Slightly prefer lower counts
            } else {
                1.0
            }
        }
        LiarsDieAction::Deal(_, _) => 1.0,
    }
}

fn get_ai_move_with_scores(state: &LiarsDie) -> (LiarsDieAction, Vec<(LiarsDieAction, f32)>) {
    let actions = state.available_actions();
    if actions.is_empty() {
        return (LiarsDieAction::Deal(vec![], vec![]), vec![]);
    }

    let trace = state.trace(Player::P2);
    let bet_count = trace.bet_history.len();

    // Score all actions
    let mut scored: Vec<(LiarsDieAction, f32)> = actions
        .iter()
        .map(|a| (a.clone(), score_action(a, bet_count)))
        .collect();

    // Sort by score (descending)
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Return best action and all scored actions
    if let Some((best_action, _)) = scored.first() {
        (best_action.clone(), scored)
    } else {
        (LiarsDieAction::Deal(vec![], vec![]), scored)
    }
}

fn get_ai_move(state: &LiarsDie) -> LiarsDieAction {
    let (action, _) = get_ai_move_with_scores(state);
    action
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let verbose = args.iter().any(|arg| arg == "--verbose");
    
    if verbose {
        env::set_var("VERBOSE_SELFPLAY", "1");
    }
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("  🎲 Interactive Liar's Dice 🎲");
    if verbose {
        println!("  Verbose mode: ON 👁️");
    }
    println!("═══════════════════════════════════════════════════════════════");
    println!("\nYou are Player 1, AI is Player 2");
    println!("Start with 5 dice each. First to lose all dice loses!\n");

    let mut game_count = 0;
    let mut player1_wins = 0;

    loop {
        game_count += 1;
        println!("\n{}", "=".repeat(63));
        println!("Game #{}", game_count);
        println!("{}\n", "=".repeat(63));

        let mut state = LiarsDie::new();
        state = state.play(&LiarsDieAction::Deal(vec![], vec![]));

        let mut turn = 0;

        loop {
            if state.is_over() {
                let reward = state.evaluate();
                let p1_trace = state.trace(Player::P1);
                let p2_trace = state.trace(Player::P2);
                
                println!("\n═══════════════════════════════════════════════════════════════");
                println!("Final Dice:");
                print!("  Your dice: ");
                for d in &p1_trace.my_dice {
                    print!("{} ", format_die(d));
                }
                println!("(count: {})", p1_trace.my_dice.len());
                
                print!("  AI dice:   ");
                for d in &p2_trace.my_dice {
                    print!("{} ", format_die(d));
                }
                println!("(count: {})", p2_trace.my_dice.len());
                
                println!();
                if reward > 0.0 {
                    println!("🎉 YOU WIN! Final score: {:.1}", reward);
                    player1_wins += 1;
                } else if reward < 0.0 {
                    println!("😔 You lost. AI wins! Final score: {:.1}", reward);
                } else {
                    println!("🤝 Draw! Final score: {:.1}", reward);
                }
                println!("═══════════════════════════════════════════════════════════════");
                break;
            }

            turn += 1;
            println!("\n──────────────────────────────────────────────────────────────");
            println!("Turn {}", turn);
            println!("──────────────────────────────────────────────────────────────");

            match state.active_player() {
                Player::P1 => {
                    // Player's turn
                    let trace = state.trace(Player::P1);
                    print!("Your dice: ");
                    for d in &trace.my_dice {
                        print!("{} ", format_die(d));
                    }
                    println!();

                    println!("\nBet history:");
                    print_history(&state, Player::P1);

                    let actions = state.available_actions();
                    println!("\nAvailable actions:");
                    for (i, action) in actions.iter().enumerate() {
                        println!("  [{}] {}", i, format_action(action));
                    }

                    loop {
                        print!("\nYour choice [0-{}] or 'q' to quit: ", actions.len() - 1);
                        io::stdout().flush().unwrap();

                        let mut input = String::new();
                        if io::stdin().read_line(&mut input).is_err() {
                            continue;
                        }

                        let input = input.trim();
                        if input.eq_ignore_ascii_case("q") {
                            println!("\nThanks for playing!");
                            return;
                        }

                        match input.parse::<usize>() {
                            Ok(idx) if idx < actions.len() => {
                                let chosen = &actions[idx];
                                println!("You: {}", format_action(chosen));
                                state = state.play(chosen);
                                break;
                            }
                            _ => println!("❌ Invalid input. Try again."),
                        }
                    }
                }

                Player::P2 => {
                    // AI's turn
                    println!("AI is thinking...");
                    std::thread::sleep(std::time::Duration::from_secs(5));

                    let (action, scored_actions) = get_ai_move_with_scores(&state);

                    let verbose = env::var("VERBOSE_SELFPLAY").is_ok();
                    if verbose {
                        let ai_trace = state.trace(Player::P2);
                        println!("\n[VERBOSE] AI's hidden dice:");
                        print!("  ");
                        for d in &ai_trace.my_dice {
                            print!("{} ", format_die(d));
                        }
                        println!("(count: {})", ai_trace.my_dice.len());

                        // Calculate probabilities from scores
                        let total_score: f32 = scored_actions.iter().map(|(_, s)| s).sum();
                        println!("[VERBOSE] AI action probabilities (top 5):");
                        for (i, (act, score)) in scored_actions.iter().enumerate() {
                            if i >= 5 {
                                break;
                            }
                            let prob = (score / total_score) * 100.0;
                            if prob > 0.1 {
                                println!("  [{:5.1}%] {}", prob, format_action(act));
                            }
                        }
                    }

                    println!("AI: {}", format_action(&action));
                    state = state.play(&action);
                }

                Player::Chance => {
                    // Deal phase
                    println!("🎲 Dealing new hands...");
                    state = state.play(&LiarsDieAction::Deal(vec![], vec![]));
                }
            }
        }

        // Ask to play again
        loop {
            print!("\nPlay again? (y/n): ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                continue;
            }

            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => break,
                "n" | "no" => {
                    println!("\n═══════════════════════════════════════════════════════════════");
                    println!("Game Stats:");
                    println!("  Games played: {}", game_count);
                    println!("  Your wins: {}", player1_wins);
                    println!("  AI wins: {}", game_count - player1_wins);
                    println!("  Win rate: {:.1}%", (player1_wins as f32 / game_count as f32) * 100.0);
                    println!("═══════════════════════════════════════════════════════════════");
                    println!("\nThanks for playing! 🎉");
                    return;
                }
                _ => println!("Please enter 'y' or 'n'."),
            }
        }
    }
}
