use std::collections::{HashMap, LinkedList};

pub enum ClueError {
    NoSuchPlayer,
    NoMatchingCards,
    NotEnoughClues,
    GameOver,
}

pub enum PlayError {
    NoSuchCard,
    GameOver,
}

pub enum DiscardError {
    NoSuchCard,
    MaxClues,
    GameOver,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum Color {
    Red,
    Green,
    White,
    Blue,
    Yellow,
}

const COLOR_ORDER: [Color; 5] = [
    Color::Red,
    Color::Green,
    Color::White,
    Color::Blue,
    Color::Yellow,
];

use std::fmt;
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Color::Red => write!(f, ":red_circle:"),
            Color::Green => write!(f, ":green_apple:"),
            Color::White => write!(f, ":white_medium_square:"),
            Color::Blue => write!(f, ":large_blue_diamond:"),
            Color::Yellow => write!(f, ":yellow_heart:"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Number {
    One,
    Two,
    Three,
    Four,
    Five,
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Number::One => write!(f, ":one:"),
            Number::Two => write!(f, ":two:"),
            Number::Three => write!(f, ":three:"),
            Number::Four => write!(f, ":four:"),
            Number::Five => write!(f, ":five:"),
        }
    }
}

impl Number {
    pub(crate) fn as_usize(&self) -> usize {
        match *self {
            Number::One => 1,
            Number::Two => 2,
            Number::Three => 3,
            Number::Four => 4,
            Number::Five => 5,
        }
    }
}

use std::ops::Add;
impl Add<usize> for Number {
    type Output = Number;
    fn add(self, rhs: usize) -> Self::Output {
        if rhs == 0 {
            return self;
        }
        let next = match self {
            Number::One => Number::Two,
            Number::Two => Number::Three,
            Number::Three => Number::Four,
            Number::Four => Number::Five,
            Number::Five => Number::Five,
        };
        next + (rhs - 1)
    }
}

#[derive(Clone, Copy)]
pub enum Clue {
    Color(Color),
    Number(Number),
}

impl fmt::Display for Clue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Clue::Color(ref c) => write!(f, "{}", c),
            Clue::Number(ref n) => write!(f, "{}", n),
        }
    }
}

pub struct Card {
    color: Color,
    number: Number,
    clues: Vec<(String, Clue)>,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.color, self.number)
    }
}


struct Deck(Vec<Card>);

impl Deck {
    pub(crate) fn draw(&mut self) -> Option<Card> {
        self.0.pop()
    }
}

struct Hand {
    player: String,
    cards: LinkedList<Card>,
}

impl Hand {
    pub(crate) fn new(player: &str) -> Self {
        Hand {
            player: String::from(player),
            cards: LinkedList::default(),
        }
    }

    pub(crate) fn draw(&mut self, deck: &mut Deck) -> bool {
        deck.draw().map(|card| self.cards.push_back(card)).is_some()
    }

    pub(crate) fn clue(&mut self, player: &str, clue: Clue) -> Result<usize, ClueError> {
        let matches = self.cards
            .iter()
            .filter(|card| match clue {
                Clue::Color(ref c) => c == &card.color,
                Clue::Number(ref n) => n == &card.number,
            })
            .count();

        if matches == 0 {
            return Err(ClueError::NoMatchingCards);
        }

        for card in &mut self.cards {
            card.clues.push((player.to_owned(), clue));
        }

        Ok(matches)
    }

    pub(crate) fn remove(&mut self, card: usize) -> Option<Card> {
        if card > self.cards.len() {
            return None;
        }

        let mut after = self.cards.split_off(card);
        let card = after.pop_front();
        self.cards.append(&mut after);
        card
    }
}

impl Default for Deck {
    fn default() -> Self {
        use rand::{thread_rng, Rng};

        let numbers = vec![
            Number::One,
            Number::One,
            Number::One,
            Number::Two,
            Number::Two,
            Number::Three,
            Number::Three,
            Number::Four,
            Number::Four,
            Number::Five,
        ];
        let mut cards: Vec<_> = COLOR_ORDER
            .iter()
            .flat_map(|&color| {
                numbers.iter().map(move |&number| {
                    Card {
                        color,
                        number,
                        clues: Vec::new(),
                    }
                })
            })
            .collect();

        thread_rng().shuffle(&mut cards[..]);
        Deck(cards)
    }
}

pub struct Game {
    deck: Deck,
    hands: Vec<Hand>,
    played: HashMap<Color, Number>,
    discard: HashMap<Color, Vec<Card>>,
    last_move: String,
    clues: usize,
    lives: usize,
    turn: usize,

    last_turns: Option<usize>,
}

impl Game {
    pub(crate) fn new(players: &[String]) -> Self {
        let mut deck = Deck::default();
        let mut hands: Vec<_> = players
            .into_iter()
            .map(|player| Hand::new(player))
            .collect();
        let cards = match hands.len() {
            0 | 1 => unreachable!(),
            2 | 3 => 5,
            4 | 5 => 4,
            _ => unreachable!(),
        };

        for hand in &mut hands {
            for _ in 0..cards {
                let drew = hand.draw(&mut deck);
                assert!(drew);
            }
        }

        Game {
            hands,
            deck,
            played: Default::default(),
            discard: Default::default(),
            last_move: String::new(),
            clues: 8,
            lives: 3,
            turn: 0,

            last_turns: None,
        }
    }

    pub(crate) fn score(&self) -> usize {
        self.played.iter().map(|(_, num)| num.as_usize()).sum()
    }

    pub(crate) fn players<'a>(&'a self) -> Box<Iterator<Item = &'a String> + 'a> {
        Box::new(self.hands.iter().map(|h| &h.player)) as Box<_>
    }

    pub(crate) fn current_player(&self) -> &str {
        &*self.hands[self.turn].player
    }

    pub(crate) fn clue(&mut self, player: &str, to: &str, clue: Clue) -> Result<usize, ClueError> {
        if self.clues == 0 {
            return Err(ClueError::NotEnoughClues);
        }

        let to = to.trim_left_matches("<@");
        let to = to.trim_right_matches('>');

        if player == to {
            return Err(ClueError::NoSuchPlayer);
        }

        let hands = self.hands.len();
        let hand = if let Some(h) = self.hands.iter_mut().find(|hand| &hand.player == to) {
            h
        } else {
            return Err(ClueError::NoSuchPlayer);
        };

        match hand.clue(player, clue) {
            Ok(num) => {
                self.last_move = format!(
                    "<@{}> clued <@{}> that {} {} {}",
                    player,
                    to,
                    num,
                    if num == 1 { "card is" } else { "cards are" },
                    clue
                );
                self.clues -= 1;
                self.turn = (self.turn + 1) % hands;
                if let Some(ref mut last_turns) = self.last_turns {
                    *last_turns += 1;
                    if *last_turns == hands {
                        return Err(ClueError::GameOver);
                    }
                }
                Ok(num)
            }
            e => e,
        }
    }

    fn discarded(&mut self, card: Card) {
        // insert into sorted discard list for that color
        let mut d = self.discard.entry(card.color).or_insert_with(Vec::new);
        let pos = d.binary_search_by_key(&card.number.as_usize(), |c| c.number.as_usize())
            .unwrap_or_else(|e| e);
        d.insert(pos, card);
    }

    pub(crate) fn play(&mut self, player: &str, card: usize) -> Result<(), PlayError> {
        let hands = self.hands.len();
        let hand = self.hands
            .iter_mut()
            .position(|hand| &hand.player == player)
            .unwrap();
        if let Some(card) = self.hands.get_mut(hand).unwrap().remove(card) {
            if !self.hands.get_mut(hand).unwrap().draw(&mut self.deck) && self.last_turns.is_none()
            {
                self.last_turns = Some(0);
            }

            use std::collections::hash_map::Entry;
            let success = match self.played.entry(card.color) {
                Entry::Vacant(e) => if card.number == Number::One {
                    e.insert(Number::One);
                    true
                } else {
                    false
                },
                Entry::Occupied(mut e) => if card.number == *e.get() + 1 {
                    e.insert(card.number);
                    if card.number == Number::Five {
                        // completed a stack!
                        // get a clue.
                        if self.clues < 8 {
                            self.clues += 1;
                        }
                    }
                    true
                } else {
                    false
                },
            };

            if !success {
                self.lives -= 1;
                self.last_move = format!(
                    "<@{}> played a {} incorrectly :right_anger_bubble:",
                    player,
                    card
                );

                self.discarded(card);

                if self.lives == 0 {
                    return Err(PlayError::GameOver);
                }
            } else {
                self.last_move = format!("<@{}> played a {}", player, card);
            }

            self.turn = (self.turn + 1) % hands;
            if let Some(ref mut last_turns) = self.last_turns {
                *last_turns += 1;
                if *last_turns == hands {
                    return Err(PlayError::GameOver);
                }
            }
            Ok(())
        } else {
            Err(PlayError::NoSuchCard)
        }
    }

    pub(crate) fn discard(&mut self, player: &str, card: usize) -> Result<(), DiscardError> {
        if self.clues == 8 {
            return Err(DiscardError::MaxClues);
        }

        let hands = self.hands.len();
        let hand = self.hands
            .iter_mut()
            .position(|hand| &hand.player == player)
            .unwrap();

        if let Some(card) = self.hands.get_mut(hand).unwrap().remove(card) {
            if !self.hands.get_mut(hand).unwrap().draw(&mut self.deck) && self.last_turns.is_none()
            {
                self.last_turns = Some(0);
            }
            self.last_move = format!("<@{}> discarded a {}", player, card);
            self.discarded(card);
            self.clues += 1;
            self.turn = (self.turn + 1) % hands;
            if let Some(ref mut last_turns) = self.last_turns {
                *last_turns += 1;
                if *last_turns == hands {
                    return Err(DiscardError::GameOver);
                }
            }
            Ok(())
        } else {
            Err(DiscardError::NoSuchCard)
        }
    }

    fn show_known(&self, hand: usize, user: &str, cli: &mut super::MessageProxy, index: bool) {
        let hand: Vec<_> = self.hands[hand]
            .cards
            .iter()
            .enumerate()
            .map(|(i, card)| {
                let know_color = card.clues.iter().any(|&(_, clue)| match clue {
                    Clue::Color(ref c) => c == &card.color,
                    _ => false,
                });
                let know_number = card.clues.iter().any(|&(_, clue)| match clue {
                    Clue::Number(ref n) => n == &card.number,
                    _ => false,
                });

                let mut desc = match (know_color, know_number) {
                    (false, false) => format!(":rainbow: :keycap_star:"),
                    (false, true) => format!(":rainbow: {}", card.number),
                    (true, false) => format!("{} :keycap_star:", card.color),
                    (true, true) => format!("{} {}", card.color, card.number),
                };
                if index {
                    desc = format!("{}: {}", i + 1, desc);
                }
                desc
            })
            .collect();
        cli.send(user, &hand.join("  |  "));
    }

    fn print_game_state(&mut self, hand: usize, cli: &mut super::MessageProxy) {
        let user = &self.hands[hand].player;
        let last = if self.last_turns.is_some() {
            " *last*"
        } else {
            ""
        };

        let setup = if self.turn == hand {
            format!("It's *your*{} turn", last)
        } else {
            format!("It's <@{}>'s{} turn", self.hands[self.turn].player, last)
        };

        // show some states about the general game state
        cli.send(
            user,
            &format!(
                "{}, and there are *{}* :information_source: and {} :bomb: remaining.",
                setup,
                self.clues,
                self.lives
            ),
        );

        let stacks: Vec<_> = COLOR_ORDER
            .iter()
            .map(|&color| if let Some(top) = self.played.get(&color) {
                format!("{} {}", color, top)
            } else {
                format!("{} :zero:", color)
            })
            .collect();
        cli.send(user, &format!("Played: {}", stacks.join(" ")));

        if self.turn == hand {
            // it is our turn.
            // show what we know about our hand, and the hands of the following players

            cli.send(user, "Your hand, as far as you know, is:");
            self.show_known(hand, user, cli, true);

            // we want to use attachments to show other players' hands
            // but we can't yet: https://api.slack.com/bot-users#post_messages_and_react_to_users
            cli.send(user, "The next players' hands are:");
            for i in 0..(self.hands.len() - 1) {
                let i = (self.turn + i) % self.hands.len();
                let h = &self.hands[i];

                if i == hand {
                    // people don't get to see their own hand, but we still want to show their turn
                    cli.send(&user, &format!("*<@{}>*: _that's you!_", h.player));
                } else {
                    let cards: Vec<_> = h.cards.iter().map(|c| format!("{}", c)).collect();
                    cli.send(
                        &user,
                        &format!("*<@{}>*: {}", h.player, cards.join("  |  ")),
                    );
                }
            }

            cli.send(
                user,
                "When you have the time, let me know here what move you want to make next!",
            );
        } else {
            // it is *not* our turn.
            // we want to show the hand of the current player, and what they know.
            cli.send(user, "The current player's hand is:");
            let cards: Vec<_> = self.hands[self.turn]
                .cards
                .iter()
                .map(|c| format!("{}", c))
                .collect();
            cli.send(&user, &format!("{}", cards.join("  |  ")));

            cli.send(user, "They know the following about their hand:");
            self.show_known(self.turn, user, cli, false);
        }
    }

    pub(crate) fn show_clues(&self, user: &str, player: &str, cli: &mut super::MessageProxy) {
        let p = player.trim_left_matches("<@");
        let p = p.trim_right_matches('>');
        let p = self.hands.iter().position(|hand| &hand.player == p);

        if p.is_none() {
            cli.send(
                user,
                &format!("there is no player in this game named {}", player),
            );
            return;
        }

        let p = p.unwrap();
        cli.send(
            user,
            &format!(
                "<@{}> knows the following about their hand:",
                self.hands[p].player
            ),
        );
        self.show_known(p, user, cli, false)
    }

    pub(crate) fn show_discards(&self, user: &str, cli: &mut super::MessageProxy) {
        if self.discard.is_empty() {
            cli.send(user, "The discard pile is empty.");
            return;
        }

        cli.send(user, "The discard pile contains the following cards:");
        for color in &COLOR_ORDER {
            if let Some(cards) = self.discard.get(color) {
                let mut out = format!("{} ", color);
                for card in cards {
                    out.push_str(&format!("{}", card.number));
                }
                cli.send(user, &out);
            }
        }
    }

    pub(crate) fn progress_game(&mut self, cli: &mut super::MessageProxy) -> bool {
        // empty line
        let divider = "--------------------------------------------------------------------------";
        for hand in &self.hands {
            // newline
            cli.send(&hand.player, " ");
            // spacer
            cli.send(&hand.player, divider);
        }

        if !self.last_move.is_empty() {
            for hand in &self.hands {
                let mut m = self.last_move
                    .replace(&format!("<@{}>", hand.player), "you");
                if m.starts_with("you") {
                    m = m.replacen("you", "You", 1);
                }
                cli.send(&hand.player, &m);
                cli.send(&hand.player, divider);
            }
        }

        let points: usize = self.played.iter().map(|(_, num)| num.as_usize()).sum();
        let mut game_over = self.lives == 0;
        if let Some(last_turns) = self.last_turns {
            game_over = game_over || last_turns == self.hands.len();
        }
        if game_over {
            // the game has ended in a loss :'(
            for hand in &self.hands {
                cli.send(
                    &hand.player,
                    &format!(
                        "Game over :slightly_frowning_face:\n\
                         You got {}/25 points.",
                        points
                    ),
                );
            }
            return true;
        }

        if points == 25 {
            // the game has ended in a win \o/
            for hand in &self.hands {
                cli.send(&hand.player, "You won the game with 25/25 points :tada:");
            }
            return true;
        }

        // game is not yet over -- let's print the game state
        for i in 0..self.hands.len() {
            self.print_game_state(i, cli);
        }

        false
    }
}
