Jacob Ferretti
Rust Programming (CS 510)
Spring 2021
Professor Bart Massey

#  Roguelike in Rust

## Description
The Roguelike in Rust project is intended to be a fun game that was and will be helpful in learning the Rust programming language. This roguelike is built using the tcod crate and currently implements a procedurally generated initial dungeon with spawned monsters which have their own basic AI. Other features include a combat and spell casting system, a player inventory for pick-ups, and a user interface.

The "Roguelike Tutorial in Rust + tcod" by Tomas Sedovic was very helpful in starting the project and understanding the functionality of the tcod crate. This tutorial and crate provided the building blocks for the game. Some code was necessarily adopted as the library did not seem to give too much in terms of documentation (such as the user interface). A good deal of time was spent on simply figuring out how the library worked and how the tutorial was using it. The remainder was spent making unique content, and developing various game mechanics and features along the way.

## How to Build
- Open Terminal
- Clone the repository
```
git clone https://github.com/jfer2/roguelike.git
```
- Navigate to roguelike/rlt
```
cd roguelike/rlt
```
- Build and run using Cargo
```
cargo run
```
## Playing the Game
### Basic Controls:
- Movement: Arrow keys
- Action key: Shift
- Inventory menu: Tab
- Exit game: Esc
### Movement
- Use the Up, Down, Right, and Left arrows to move your player around the dungeon. You are only able to walk around rooms and through corridors.
### Attacking
- Close Combat
	- Press the arrow key in the direction of the monster to attack. Repeatedly press the key to the monster's current direction to continue attacking. The monster will turn to a '%' when its hit point (HP) meter reaches 0.
### Pick-ups
- Healing Potion (ASCII - "I")
	- Press Shift to pick up a healing potion. Press Tab to access the inventory and press the relevant key in the menu to use the potion to recover HP.
- Fire Ring Scroll (ASCII - "#")
	- Press Shift to pick up the scroll. Press Tab to access the inventory and press the relevant key to cast the Fire Ring spell. The Fire Ring does a great deal of damage within a four tile range on a direct hit, and continues to smolder for some time dealing slight damage to monsters that walk on those tiles that are still smoldering.
### Teleporting
- Tiles that teleport the player are blue in color and move the player to another room in the dungeon.

### HP Regeneration from the Dead
- Energy can be regained from the conquered monster's corpse. Press Shift over the corpse to regain a slight amount of HP. The corpse will vanish and the ASCII character "_" will appear.

### Monsters
- Drudges are weak, but have a decent attack ability.
- Goblins are a bit tougher then drudges in attack, defense and, hit points.
- The White Rabbit is the toughest of foes. Perhaps, it would be best to gather a couple a couple healing potions and Fire Ring scrolls prior to doing combat with this monster.

## Future Work
- Additional testing
- Parse code from main.rs into smaller, more organized files based on functionality
- Quest system
- Level and Experience system
- Additional pick-ups
- Targeting for spell casting and long-range combat

## Notes
- The other two directories in this repository are other roguelike games and tutorials that I had tried out. I may switch to another roguelike library as tcod is no longer supported.


