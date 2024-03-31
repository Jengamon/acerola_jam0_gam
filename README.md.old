# Acerola Jam 0 Game - Bevy edition

I like Rust _way_ too much.

Anyways, Bevy.

## What's the game?

Arc words: _You are just a number._

Story hints:
- Beating the game without using a single card: achievement *A Number that Crushed You All*.

If you beat the game while using 1+ cards, you can name your character, otherwise
you are only identified by the ULID generated at the start of your run.

In universe, you are a "aberration" in that you aren't acting in your place,
and it is negative from the in-universe perspective because everyone must
be in their place in order for the society that operates to function...

The idea is a card battler roguelike.
You are a character running around SOMEPLACE (using WASD or a joystick).
You have 3 actions other than movement:

- someway of toggling or activating a faster movement (running)
- light attack (Z)
- heavy attack (X)
  The only way to do special moves is through your deck, which
  Left and Right arrows switch the card in your hand, and Up uses the currently
  selected card. Double-pressing down will _exhaust_ your hand, discarding
  all the cards within it.

Stats:

- damage - how much damage does your light attack do?
  heavy attack is 1.5x - 2x this, and all card damage scales off this
  or is set.
- defence - how much is incoming damage multiplied by? (0.0 would be 
  immune to damage, 1.0 is equal to amount afflicted)
- draw - the number of seconds it takes after you have exhausted your hand
  to draw a new, full hand
- shuffle - the number of seconds it takes after your deck has been
  exhausted to shuffle your discards and create a new deck
- size - the number of cards you can hold in your hand. If
  this is _greater_ than the number of cards in your deck, you will get
  a draw/shuffle time multiplier that maxes out at x0.5 when size is 10x
  your deck size.
- health - determines the max amount of health you can have
- stamina - some cards require certain amounts, also, you 
do not regain stamina while dashing (and will not be able to 
dash if this is below critical)

but the stats above, *every* character has. Your player has their own set
of additional stats which they can change and which the above are based on for them.

Player stats:
- Kan - generally deals with vitality/health
- Ren - generally deals with mind
- Souh - generally deals with dexerity
- Phim - generally deals with spirit

### Control Schemes

The "default" one is:

- Movement (WASD)
- Attacks (light, heavy) (Z, X)
- Dash (press) (shift)
- Hand Switch (Left, Right) (LeftArrow, RightArrow)
- Hand Use (Up)
- Hand Exhaust (Hold DownArrow)

An alternative scheme I'd want to also support is:

- Movement (left mouse button, using delta from center)
- Attacks (light, heavy) (A, S)
- Dash (press) (right mouse button, instead of left)
- Hand Switch (Left, Right) (Q, E)
- Hand Use (W)
- Hand Exhaust (Hold D)

### Card Ideas

Ideally, the card's display should only consist of a colored box
with some representative symbols on it. The only common symbols are:

- an arrow, this represents that the character must be moving
  in some direction in order to use the card. marked by (M)

The available colors are:

- red - offensive, tends to move forward {R}
- blue - defensive, tends to lock in place {B}
- yellow - avoiding, tends to move backwards {Y}
  These are meant to give you an idea at a glance what kind of
  movement can be expected of your character when using the card.

The first card you get is:

- (M) {R} Dash - move forward 5 units, dealing 400% damage to anyone
  in the way

A more complicated card might be:

- {B} Marryuk's Shield - Lock 3, (Dizzy) if hit 4 times. If you were
  hit exactly 3 times when unlocking, deal 600% damage to all within
  9 units.

### Stealing Cards

A central mechanic is stealing cards. (This is my impl of the theme in
gameplay, as you are the only one in game that is capable of
doing this _without using a card_.)

Opponents will drop a card when you defeat them. You may choose to
pick it up or not, but for bosses, you will automatically record
the card they drop.
The only technically "roguelite" element of the game
so far is that we will keep a record of which cards you got from which
enemies. _This is the only way to have information about cards._ Cards
you haven't seen will be unknown to you.

The card you steal is a random card that opponent was using, and you can only
steal specific cards by attacking an opponent with a light attack _while
they are using that card_. If you defeat an enemy, the card you get if
you decide to pick it up _is random_.

### More ideas
- If you steal all the cards of a boss, you instead make them Enraged, restoring
their health, and restarting the fight, except they use more strings of cards,
and they have all armor.
- Armor: instead of only light attacks stealing, you can steal with *any* attack, but:
  - the target must be using a card (that's the card you'll steal)
  - they can't have armor to the attack
    - light armor works against light attacks, and heavy armor for heavy attacks
    - all armor is immunity from stealing (but doesn't really show up except for 
    Enraged bosses)

Card moves are split into 3 phases:
- Windup
  - has a movement multiplier (X and Y so vec2)
  - characters have implicit all armor here
- Action
  - has a movement modify multiplier (0 - card controls how you move, 1 - you and card do)
  - armor as determined by the card
- Cooldown
  - has a movement modifier
  - card is considered active (stealable) until the end of this phase

### States
Cards may afflict states on creatures. The state lasts for a certain duration.
Cards may specify something to happen when the state is released.

- Lock - movement 0, you cannot move.
- Dizzy x - movement 0, defence +x (remember a defence > 1.0 *multiplies* incoming damage) (x defaults to 1)
