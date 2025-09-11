use StudentOfGames::games::rps::Rps;
use StudentOfGames::obscuro::Obscuro;
use StudentOfGames::utils::{Game, Player};

fn main() {
    let game = Rps::new();
    let mut obscuro: Obscuro<Rps> = Obscuro::default();
    obscuro.make_move(game.trace(Player::P1), Player::P1);
    // plot_test()
}

fn plot_test() {
    use pgfplots::{axis::plot::Plot2D, Engine, Picture};

    let mut plot = Plot2D::new();
    plot.coordinates = (-100..100)
        .into_iter()
        .map(|i| (f64::from(i), f64::from(i*i)).into())
        .collect();

    Picture::from(plot).show_pdf(Engine::PdfLatex);
}