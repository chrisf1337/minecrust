#![feature(custom_attribute, try_trait)]
#![allow(unknown_lints)]
#![warn(clippy::all)]

use failure::Error;
use minecrust::game::Game;

fn main() -> Result<(), Error> {
    let mut game = Game::new(1024, 768)?;
    game.start()?;
    Ok(())
}
