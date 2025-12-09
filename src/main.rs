mod fancap_scraper;
mod gui;

use gui::FancapRipper;
use iced::Theme;

pub fn main() -> iced::Result {
    iced::application("Fancap Ripper", FancapRipper::update, FancapRipper::view)
        .theme(|_| Theme::Dark)
        .run()
}

