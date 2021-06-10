use rand::Rng;
use std::cmp;
use tcod::colors::*;
use tcod::console::*;
use tcod::map::{FovAlgorithm, Map as FovMap};

// Field of View
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 15;

// The entire size of the window
const SCREEN_WIDTH: i32 = 120;
const SCREEN_HEIGHT: i32 = 80;

// Dimensions of the map, and rooms
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 43;
const INVENTORY_WIDTH: i32 = 50;
const ROOM_MAX_SIZE: i32 = 20;
const ROOM_MIN_SIZE: i32 = 5;

// Room numbers and contents
const MAX_ROOMS: i32 = 30;
const MAX_ROOM_MONSTERS: i32 = 2;
const MAX_ROOM_ITEMS: i32 = 1;

// Misc properties
const CORPSE_CONSUME_HP: i32 = 2;
const HEAL_AMOUNT: i32 = 10;
const RING_RANGE: i32 = 4;
const FIRE_RING_DAMAGE: i32 = 20;

// Panel and messaging interface
const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;
const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

// FPS
const LIMIT_FPS: i32 = 20; // 20 frames-per-second maximum

// Player is always 0 in Objects
const PLAYER: usize = 0;

// RGB data for various Tile states
const COLOR_DARK_WALL: Color = Color {
    r: 120,
    g: 120,
    b: 120,
};
const COLOR_LIGHT_WALL: Color = Color {
    r: 255,
    g: 205,
    b: 105,
};
const COLOR_LIGHT_PERIMETER: Color = Color {
    r: 100,
    g: 100,
    b: 100,
};
const COLOR_DARK_PERIMETER: Color = Color {
    r: 40,
    g: 40,
    b: 40,
};
const COLOR_DARK_GROUND: Color = Color {
    r: 65,
    g: 90,
    b: 50,
};
const COLOR_LIGHT_TELEPORT: Color = Color { r: 0, g: 0, b: 225 };
const COLOR_DARK_TELEPORT: Color = Color { r: 0, g: 0, b: 130 };

// Main struct for passing game states root, con, panel and FOV
struct Tcod {
    root: Root,
    con: Offscreen,
    panel: Offscreen,
    fov: FovMap,
}

// The map is a vector of Tile vectors and each Tile is accessed as in 'map[x][y]'
type Map = Vec<Vec<Tile>>;

// Game struct contains the map, messages, and inventory
struct Game {
    map: Map,
    messages: Messages,
    inventory: Vec<Object>,
}

// Player action can for each game tick can be one of three actions
#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

// A Tile is a single square on the Map which contains a number of properties
#[derive(Clone, Copy, Debug)]
struct Tile {
    blocked: bool,
    block_sight: bool,
    perimeter: bool,
    teleport: bool,
    explored: bool,
    has_corpse: bool,
    on_fire: (bool, i32),
}

// Items can be used or their use can be cancelled if there was an error is usage
enum UseResult {
    UsedUp,
    Cancelled,
}

// Items which are non-fighting objects
#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
    Heal,
    FireRing,
}

/// Increases a fighter object's HP by HEAL_AMOUNT
///
fn cast_heal(
    _inventory_id: usize,
    _tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [Object],
) -> UseResult {
    // heal the player
    if let Some(fighter) = objects[PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            game.messages.add("You are already at full health", RED);
            return UseResult::Cancelled;
        }
        game.messages.add(
            format!("You have healed yourself for {} HP", HEAL_AMOUNT),
            LIGHT_VIOLET,
        );
        objects[PLAYER].heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

/// Casts a ring of fire around the player which causes direct damage if a fighter object is in
/// range or indirect if a fighter object comes into its range and the tile is still "hot"
///
fn cast_fire_ring(
    _inventory_id: usize,
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [Object],
) -> UseResult {
    // target all monsters within the RING_RANGE of the player
    let monster_ids = get_monsters_in_range(tcod, objects, RING_RANGE);
    let mut no_effect: bool = true;
    for monster_id in monster_ids {
        if let Some(monster_id) = monster_id {
            game.messages.add(
                format!(
                    "Fire ring conflagrated the {} for {} hit points",
                    objects[monster_id].name, FIRE_RING_DAMAGE
                ),
                LIGHT_BLUE,
            );
            no_effect = false;
            objects[monster_id].take_damage(FIRE_RING_DAMAGE, game);
        }
    }

    // no effect message if there are no monsters in the player's ring range
    if no_effect {
        game.messages.add("Fire ring caused no direct damage", RED);
    }

    // set tiles on fire with range of cast
    set_tiles_on_fire(game, objects, RING_RANGE);
    UseResult::UsedUp
}

/// Tiles can be set on fire, for example, after the Fire Ring spell is casted. The tiles
/// eventually return to a normal state after 10 game ticks or if a monster steps on the tile that
/// is on fire
///
fn set_tiles_on_fire(game: &mut Game, objects: &mut [Object], range: i32) {
    let pos = objects[PLAYER].pos();
    for y in (-range)..range {
        for x in pos.0..=(pos.0 + range) {
            if game.map[x as usize][(pos.1 + y) as usize].perimeter
                || game.map[x as usize][(pos.1 + y) as usize].blocked
            {
                break;
            }
            game.map[x as usize][(pos.1 + y) as usize].on_fire = (true, 20_i32);
        }
        for x in (pos.0 - range + 1)..pos.0 {
            if game.map[x as usize][(pos.1 + y) as usize].perimeter
                || game.map[x as usize][(pos.1 + y) as usize].blocked
            {
                break;
            }
            game.map[x as usize][(pos.1 + y) as usize].on_fire = (true, 20_i32);
        }
    }
}

/// Determines all the monsters that are in a certain range. They are returned as Option(fighters)
/// in the vector
///
fn get_monsters_in_range(
    tcod: &mut Tcod,
    objects: &mut [Object],
    range: i32,
) -> Vec<Option<usize>> {
    let mut monsters_in_range = vec![];
    for (id, object) in objects.iter().enumerate() {
        if (id != PLAYER)
            && object.fighter.is_some()
            && object.ai.is_some()
            && tcod.fov.is_in_fov(object.x, object.y)
        {
            let distance = objects[PLAYER].distance_to(object);
            if distance <= range as f32 {
                monsters_in_range.push(Some(id));
            }
        }
    }
    monsters_in_range
}

/// Uses an item in inventory, and removes in from the inventory by inventory_id if sucessfully
/// used. Cancelled otherwise.
///
fn use_item(inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
    use Item::*;

    if let Some(item) = game.inventory[inventory_id].item {
        let on_use = match item {
            Heal => cast_heal,
            FireRing => cast_fire_ring,
        };

        match on_use(inventory_id, tcod, game, objects) {
            UseResult::UsedUp => {
                // destroy after use
                game.inventory.remove(inventory_id);
            }
            UseResult::Cancelled => {
                game.messages.add("Cancelled", WHITE);
            }
        }
    } else {
        game.messages.add(
            format!("The {} cannot be used", game.inventory[inventory_id].name),
            WHITE,
        );
    }
}

/// Picks up an item. The inventory is capped at 26 items
///
fn pick_item_up(object_id: usize, game: &mut Game, objects: &mut Vec<Object>) {
    if game.inventory.len() >= 26 {
        game.messages.add(
            format!(
                "Your inventory is full, cannot pick up {}",
                objects[object_id].name
            ),
            RED,
        );
    } else {
        let item = objects.swap_remove(object_id);
        game.messages
            .add(format!("You picked up a {}!", item.name), GREEN);
        game.inventory.push(item);
    }
}

// A message containing a string and text color for output
//
struct Messages {
    messages: Vec<(String, Color)>,
}

impl Messages {
    /// Creates a new vector to hold the messages
    pub fn new() -> Self {
        Self { messages: vec![] }
    }
    /// Adds a new message and color
    pub fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.messages.push((message.into(), color));
    }
    /// iterator for messages
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(String, Color)> {
        self.messages.iter()
    }
}

// A Fighter is an object such as a monster or player that can attack, be attacked, and die
//
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fighter {
    max_hp: i32,
    hp: i32,
    defense: i32,
    power: i32,
    on_death: DeathCallback,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    fn callback(self, object: &mut Object, game: &mut Game) {
        use DeathCallback::*;
        let callback = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object, game);
    }
}

fn player_death(player: &mut Object, game: &mut Game) {
    // game over
    game.messages.add("You Died!!", RED);
    player.char = '%';
    player.color = DARK_RED;
}

fn monster_death(monster: &mut Object, game: &mut Game) {
    // monster has died, and becomes an ASCII '%' on the tile where it was killed by the player
    game.messages
        .add(format!("{} is dead!", monster.name), ORANGE);
    monster.char = '%';
    monster.color = DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);

    // corpse position and tile update for corpse regen HP
    let corpse = monster.pos();
    game.map[corpse.0 as usize][corpse.1 as usize].has_corpse = true;
}

#[derive(Clone, Debug, PartialEq)]
enum Ai {
    Basic,
}

// The various tile states
impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            perimeter: false,
            teleport: false,
            explored: false,
            has_corpse: false,
            on_fire: (false, 0),
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            perimeter: false,
            teleport: false,
            explored: false,
            has_corpse: false,
            on_fire: (false, 0),
        }
    }
    pub fn perimeter() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            perimeter: true,
            teleport: false,
            explored: false,
            has_corpse: false,
            on_fire: (false, 0),
        }
    }
    pub fn teleport() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            perimeter: false,
            teleport: true,
            explored: false,
            has_corpse: false,
            on_fire: (false, 0),
        }
    }
    pub fn is_teleportable_to(&self) -> bool {
        !self.blocked && !self.teleport
    }
    pub fn is_empty(&self) -> bool {
        !self.blocked
    }
}

// A room on the map marked by x and y coordinates
//
#[derive(Clone, Copy, Debug)]
struct Room {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

/// Example of a room:
///
/// (x1,y1)
///     |-- width--|
///     ------------ ---
///     |          |  |
///     |          | height
///     |          |  |
///     |__________| _|_
///               (x2,y2)
///               
impl Room {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Room {
            x1: x,
            y1: y,
            x2: x + width,
            y2: y + height,
        }
    }
    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn room_overlaps(&self, r: &Room) -> bool {
        (self.x1 <= r.x2) && (self.x2 >= r.x1) && (self.y1 <= r.y2) && (self.y2 >= r.y1)
    }
}

fn create_room(room: Room, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

/// A basic object type which has coordinates and a ASCII char that represents it along with
/// additional properties which give it added functionality if required
///
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
    fighter: Option<Fighter>,
    ai: Option<Ai>,
    item: Option<Item>,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object {
            x,
            y,
            char,
            color,
            name: name.into(),
            blocks,
            alive: false,
            fighter: None,
            ai: None,
            item: None,
        }
    }

    /// Draws the object to the screen with the correct color and coordinates
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    /// Returns the current position of the object
    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    /// Sets the position of an object
    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    /// Returns the distance from the invoking object to the another object
    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    /// A Fighter Object takes an amount of damage
    pub fn take_damage(&mut self, damage: i32, game: &mut Game) {
        // incur damage to health meter
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
            }
        }
    }
    /// Player regains hp from the corpse of a slain monster
    pub fn consume_corpse(&mut self, hp: i32, game: &mut Game) {
        // increase hp if possible
        if let Some(fighter) = self.fighter.as_mut() {
            if fighter.hp + hp >= fighter.max_hp {
                fighter.hp = fighter.max_hp;
            } else {
                fighter.hp += hp
            }
        }
        game.messages.add(
            format!("You consumed a corpse and gained {} HP", CORPSE_CONSUME_HP),
            LIGHT_VIOLET,
        );
    }
    /// Increases invoking objects HP by a specific amount
    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }
    /// Invoking object attacks another object
    pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
        // Damage formula for computing damage based on power and defense attributes
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            // target takes an amount of damage
            game.messages.add(
                format!(
                    "{} attacks {} for {} hit points",
                    self.name, target.name, damage
                ),
                WHITE,
            );
            target.take_damage(damage, game);
        } else {
            game.messages.add(
                format!(
                    "{} attacks {} but it has not effect!",
                    self.name, target.name
                ),
                WHITE,
            );
        }
    }
}

/// Parses two elements from slice and mutably borrows
///
fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    let split_as_index = cmp::max(first_index, second_index);
    let (first_slice, second_slice) = items.split_at_mut(split_as_index);
    if first_index < second_index {
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        (&mut second_slice[0], &mut first_slice[second_index])
    }
}

/// Returns true if the tile is blocking otherwise returns false
///
fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // checks if the tile is blocking
    if map[x as usize][y as usize].blocked {
        return true;
    }

    // checks if an object/item is blocking
    objects
        .iter()
        .any(|object| object.blocks && object.pos() == (x, y))
}

/// Used for movement towards a specific location
///
fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    // vector form this object to the target, and distance
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

/// move by the given amount, if the destination is not blocked
///
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

/// Initial function to generate dungeon, spawn monsters, drop items, and place the player
fn make_map(objects: &mut Vec<Object>) -> Map {
    // fill map with "unblocked" tiles
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    // create walled perimeter in game map
    for tile in 0..MAP_WIDTH {
        map[tile as usize][0_usize] = Tile::perimeter();
        map[tile as usize][(MAP_HEIGHT - 1) as usize] = Tile::perimeter();
    }
    for tile in 0..MAP_HEIGHT {
        map[0_usize][tile as usize] = Tile::perimeter();
        map[(MAP_WIDTH - 1) as usize][tile as usize] = Tile::perimeter();
    }

    /* Testing for room functionality
    let room1 = Room::new(20, 15, 10, 15);
    let room2 = Room::new(50, 15, 10, 15);
    let room3 = Room::new(115, 55, 5, 5);
    create_room(room1, &mut map);
    create_room(room2, &mut map);
    create_room(room3, &mut map);
    create_horizontal_passage(25, 55, 23, &mut map);
    */

    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        // random width and height
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        // random position without going out of the boundaries of the map
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);
        let new_room = Room::new(x, y, w, h);

        let overlap = rooms
            .iter()
            .any(|other_room| new_room.room_overlaps(other_room));

        if !overlap {
            create_room(new_room, &mut map);
            place_objects(new_room, &map, objects);
            let (new_x, new_y) = new_room.center();
            if !rooms.is_empty() {
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();
                if rand::random() {
                    // first move horizontally, then vertically
                    create_horizontal_passage(prev_x, new_x, prev_y, &mut map);
                    create_vertical_passage(prev_y, new_y, new_x, &mut map);
                } else {
                    // first move vertically, then horizontally
                    create_vertical_passage(prev_y, new_y, prev_x, &mut map);
                    create_horizontal_passage(prev_x, new_x, new_y, &mut map);
                }
            }
            rooms.push(new_room);
        }
    }
    // Get a random room and plae the player in it
    let random_room_number = rand::thread_rng().gen_range(0, rooms.len());
    let mut center: (i32, i32) = rooms[random_room_number].center();
    objects[PLAYER].set_pos(center.0, center.1);

    // Place a Fire Ring Scroll in the vicinty of the player's starting position
    let mut object = Object::new(
        objects[PLAYER].x + 1,
        objects[PLAYER].y + 1,
        '#',
        "Fire Ring Scroll",
        LIGHT_YELLOW,
        false,
    );
    object.item = Some(Item::FireRing);
    objects.push(object);

    // Get a random room and place a teleport tile in it
    let random_room_number = rand::thread_rng().gen_range(0, rooms.len());
    center = rooms[random_room_number].center();
    map[center.0 as usize][center.1 as usize] = Tile::teleport();

    map
}

/// Creats a horizontal passage to from x1 to x2 at y on y-axis
///
fn create_horizontal_passage(x1: i32, x2: i32, y: i32, map: &mut Map) {
    let passage: (i32, i32) = {
        if x1 <= x2 {
            (x1, x2)
        } else {
            (x2, x1)
        }
    };

    for x in passage.0..passage.1 + 1 {
        map[x as usize][y as usize] = Tile::empty();
    }
}

/// Creats a vertical passage from y1 to y2 at x on x-axis
///
fn create_vertical_passage(y1: i32, y2: i32, x: i32, map: &mut Map) {
    let passage: (i32, i32) = {
        if y1 <= y2 {
            (y1, y2)
        } else {
            (y2, y1)
        }
    };
    for y in passage.0..passage.1 + 1 {
        map[x as usize][y as usize] = Tile::empty();
    }
}

/// main function to render the game state, objects in FOV, and map
fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {
    if fov_recompute {
        // If necessary FOV needs to be updated
        let player = &objects[PLAYER];
        tcod.fov
            .compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }

    // Set Tile background colors with pattern matching
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);
            let wall = game.map[x as usize][y as usize].block_sight;
            let perimeter = game.map[x as usize][y as usize].perimeter;
            let teleport = game.map[x as usize][y as usize].teleport;
            let on_fire = game.map[x as usize][y as usize].on_fire;

            // Check if Tile is on fire. Decrement count if still on fire. If count is 0, then
            // update Tile's status
            if on_fire.0 {
                if game.map[x as usize][y as usize].on_fire.1 == 0 {
                    game.map[x as usize][y as usize].on_fire.0 = false;
                } else {
                    game.map[x as usize][y as usize].on_fire.1 -= 1;
                }
            }

            let color = match (visible, wall, perimeter, teleport, on_fire.0) {
                // Outside player's FOV
                (false, true, true, false, false) => COLOR_DARK_PERIMETER,
                (false, true, false, false, false) => COLOR_DARK_WALL,
                (false, false, false, true, false) => COLOR_DARK_TELEPORT,
                (false, false, false, false, false) => COLOR_DARK_GROUND,
                // Inside player's FOV
                (true, true, true, false, false) => COLOR_LIGHT_PERIMETER,
                (true, true, false, false, false) => COLOR_LIGHT_WALL,
                (true, false, false, true, false) => COLOR_LIGHT_TELEPORT,
                (true, false, false, false, false) => COLOR_DARK_GROUND,
                (_, _, _, _, true) => LIGHTER_RED,
                _ => COLOR_DARK_PERIMETER,
            };

            // Explored and unexplored tiles update
            let explored = &mut game.map[x as usize][y as usize].explored;
            if visible {
                *explored = true;
            }
            if *explored {
                tcod.con
                    .set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }
    let mut to_draw: Vec<_> = objects
        .iter()
        .filter(|o| tcod.fov.is_in_fov(o.x, o.y))
        .collect();
    to_draw.sort_by(|o1, o2| o1.blocks.cmp(&o2.blocks));
    for object in &to_draw {
        object.draw(&mut tcod.con);
    }

    // blit is a special tcod function to push the contents of "con" to the root console
    blit(
        &tcod.con,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );

    tcod.panel.set_default_background(BLACK);
    tcod.panel.clear();

    // Print messages to UI panel
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.messages.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    // Render the player's attributes (health meter)
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(
        &mut tcod.panel,
        1,
        1,
        BAR_WIDTH,
        "HP",
        hp,
        max_hp,
        LIGHT_RED,
        DARKER_RED,
    );

    // Output the panel with blit
    blit(
        &tcod.panel,
        (0, 0),
        (SCREEN_WIDTH, PANEL_HEIGHT),
        &mut tcod.root,
        (0, PANEL_Y),
        1.0,
        1.0,
    );
}

/// Checks if a the player is a on a Tile that teleports. Changes player's position if true,
/// otherwise there is no effect
fn check_teleport(map: &mut Map, player: &mut Object) {
    // Check if player is on a teleport tile
    if map[player.x as usize][player.y as usize].teleport {
        let prev = (player.x, player.y);
        // Randomly selects a tiles, checks if its suitable to teleport to and changes
        // player's current location to that tile when found
        loop {
            let x = rand::thread_rng().gen_range(1, MAP_WIDTH - 1);
            let y = rand::thread_rng().gen_range(1, MAP_HEIGHT - 1);
            if map[x as usize][y as usize].is_teleportable_to() {
                player.x = x;
                player.y = y;
                map[prev.0 as usize][prev.1 as usize].teleport = false;
                break;
            }
        }
        place_rand_teleport_tile(map);
    }
}

/// Returns the coordinates of a randomly generated map Tile that is not in a corridor or
/// blocking the entrance to or exit from a corridor
///
fn get_tile_non_passage_blocking(map: &Map) -> (i32, i32) {
    // Get random (x,y) coord for a tile
    let mut x: i32 = rand::thread_rng().gen_range(3, MAP_WIDTH - 3);
    let mut y: i32 = rand::thread_rng().gen_range(3, MAP_HEIGHT - 3);

    // Check if random gen (x,y) is blocking a passage
    loop {
        if map[(x + 1) as usize][y as usize].is_empty()
            & map[(x - 1) as usize][y as usize].is_empty()
            & map[x as usize][(y + 1) as usize].is_empty()
            & map[x as usize][(y - 1) as usize].is_empty()
            & map[(x + 1) as usize][(y + 1) as usize].is_empty()
            & map[(x + 1) as usize][(y - 1) as usize].is_empty()
            & map[(x - 1) as usize][(y + 1) as usize].is_empty()
            & map[(x - 1) as usize][(y - 1) as usize].is_empty()
        {
            break;
        } else {
            x = rand::thread_rng().gen_range(3, MAP_WIDTH - 3);
            y = rand::thread_rng().gen_range(3, MAP_HEIGHT - 3);
        }
    }
    (x, y)
}

/// Places a randomly generated teleport Tile in the game map
///
fn place_rand_teleport_tile(map: &mut Map) {
    loop {
        let tile: (i32, i32) = get_tile_non_passage_blocking(map);
        if map[tile.0 as usize][tile.1 as usize].is_teleportable_to() {
            map[tile.0 as usize][tile.1 as usize].teleport = true;
            break;
        }
    }
}

/// Key controls for player movement and gameplay
fn handle_keys(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) -> PlayerAction {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;
    use PlayerAction::*;

    let key = tcod.root.wait_for_keypress(true);
    let player_alive = objects[PLAYER].alive;
    match (key, key.text(), player_alive) {
        (
            Key {
                code: Enter,
                alt: true,
                ..
            },
            _,
            _,
        ) => {
            // Alt+Enter: toggle fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        }
        (Key { code: Escape, .. }, _, _) => Exit, // exit game

        // movement keys
        (Key { code: Up, .. }, _, true) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        }
        (Key { code: Down, .. }, _, true) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        }
        (Key { code: Left, .. }, _, true) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        }
        (Key { code: Right, .. }, _, true) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        }
        (Key { code: Shift, .. }, _, true) => {
            // pick up item
            let position = objects[PLAYER].pos();
            let item_id = objects
                .iter()
                .position(|object| object.pos() == objects[PLAYER].pos() && object.item.is_some());
            if let Some(item_id) = item_id {
                pick_item_up(item_id, game, objects);
            }
            if game.map[position.0 as usize][position.1 as usize].has_corpse {
                objects[PLAYER].consume_corpse(CORPSE_CONSUME_HP, game);
                game.map[position.0 as usize][position.1 as usize].has_corpse = false;
                for object in objects {
                    if object.pos() == (position.0, position.1) {
                        object.char = '_';
                    }
                }
            }
            DidntTakeTurn
        }
        (Key { code: Tab, .. }, _, true) => {
            let inventory_index = inventory_menu(
                &game.inventory,
                "Press the key next to an item to use it, or any other to cancel.\n",
                &mut tcod.root,
            );
            if let Some(inventory_index) = inventory_index {
                use_item(inventory_index, tcod, game, objects);
            }
            DidntTakeTurn
        }
        _ => DidntTakeTurn,
    }
}

/// tcod menu setup primarly reused from tcodlib menu tutorial  
fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
    assert!(
        options.len() <= 26,
        "Cannot have a menu with more than 26 options."
    );

    let header_height = root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header);
    let height = options.len() as i32 + header_height;

    let mut window = Offscreen::new(width, height);

    window.set_default_foreground(WHITE);
    window.print_rect_ex(
        0,
        0,
        width,
        height,
        BackgroundFlag::None,
        TextAlignment::Left,
        header,
    );

    // print all the options
    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(
            0,
            header_height + index as i32,
            BackgroundFlag::None,
            TextAlignment::Left,
            text,
        );
    }

    // blit the contents of "window" to the root console
    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    blit(&window, (0, 0), (width, height), root, (x, y), 1.0, 0.7);

    // present the root console to the player and wait for a key-press
    root.flush();
    let key = root.wait_for_keypress(true);

    // convert the ASCII code to an index; if it corresponds to an option, return it
    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    // how a menu with each item of the inventory as an option
    let options = if inventory.is_empty() {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| item.name.clone()).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    // if an item was chosen, return it
    if !inventory.is_empty() {
        inventory_index
    } else {
        None
    }
}

/// Determines if a player is moving or attacking based on game state
fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game, objects: &mut [Object]) {
    // The coords to where the player is moving to
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    // Determine if there is a Fighter Object at the coords by iterating through Objects and
    // checking their position
    let target_id = objects
        .iter()
        .position(|object| object.fighter.is_some() && object.pos() == (x, y));

    // If a Fighter Object was found, then attack this Fighter
    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
            player.attack(target, game);
        }
        // If no Fighter found then move to this tile
        None => {
            move_by(PLAYER, dx, dy, &game.map, objects);
        }
    }
}

/// Spawns monsters throughout the dungeon
///
fn place_objects(room: Room, map: &Map, objects: &mut Vec<Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            let mut monster = if rand::random::<f32>() < 0.8 {
                // Create a goblin
                let mut goblin = Object::new(x, y, 'G', "goblin", DESATURATED_GREEN, true);
                goblin.fighter = Some(Fighter {
                    max_hp: 9,
                    hp: 9,
                    defense: 2,
                    power: 3,
                    on_death: DeathCallback::Monster,
                });
                goblin.ai = Some(Ai::Basic);
                goblin
            } else if rand::random::<f32>() < 0.5 {
                // Create a drudge
                let mut drudge = Object::new(x, y, 'D', "drudge", DARKER_RED, true);
                drudge.fighter = Some(Fighter {
                    max_hp: 3,
                    hp: 3,
                    defense: 1,
                    power: 2,
                    on_death: DeathCallback::Monster,
                });
                drudge.ai = Some(Ai::Basic);
                drudge
            } else {
                // Create a white rabbit
                let mut white_rabbit = Object::new(x, y, 'W', "white rabbit", WHITE, true);
                white_rabbit.fighter = Some(Fighter {
                    max_hp: 50,
                    hp: 50,
                    defense: 2,
                    power: 5,
                    on_death: DeathCallback::Monster,
                });
                white_rabbit.ai = Some(Ai::Basic);
                white_rabbit
            };
            monster.alive = true;
            objects.push(monster);
        }
    }

    let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

    for _ in 0..num_items {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        // Find a random spot for this items
        let mut again: bool = true;
        while again {
            x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
            y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
            if !is_blocked(x, y, map, objects) {
                again = false;
            }
        }

        // Randomly place item pick-ups
        let dice = rand::random::<f32>();
        let item = if dice < 0.6 {
            // A Healing Potion
            let mut object = Object::new(x, y, '!', "healing potion", VIOLET, false);
            object.item = Some(Item::Heal);
            object
        } else {
            // A Fire Ring Scroll
            let mut object = Object::new(x, y, '#', "Fire ring spell", LIGHT_YELLOW, false);
            object.item = Some(Item::FireRing);
            object
        };
        objects.push(item);
    }
}

fn render_bar(
    panel: &mut Offscreen,
    x: i32,
    y: i32,
    total_width: i32,
    name: &str,
    value: i32,
    maximum: i32,
    bar_color: Color,
    back_color: Color,
) {
    // Compute bar width
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

    // Set the background
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    // Set the foreground and print the text on top
    panel.set_default_foreground(WHITE);
    panel.print_ex(
        x + total_width / 2,
        y,
        BackgroundFlag::None,
        TextAlignment::Center,
        &format!("{}: {}/{}", name, value, maximum),
    );
}

/// Allows the Fighter Object to take a turn which is either an attack or a movement
///
fn ai_take_turn(monster_id: usize, tcod: &Tcod, game: &mut Game, objects: &mut [Object]) {
    let (monster_x, monster_y) = objects[monster_id].pos();
    if tcod.fov.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
            // Object Fighter moves towards the PLAYER
            let (player_x, player_y) = objects[PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &game.map, objects);
        } else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            // Object Fighter attacks the PLAYER
            let (monster, player) = mut_two(monster_id, PLAYER, objects);
            monster.attack(player, game);
        }
    }
    // Checks if the Tile the Fighter is currently standing on is on fire and if so deals damage to
    // that Figher
    let tile_state = game.map[monster_x as usize][monster_y as usize].on_fire;
    if tile_state.0 {
        let fire_damage = match tile_state.1 {
            1..=5 => 1,
            _ => 2,
        };
        objects[monster_id].take_damage(fire_damage, game);
        game.messages.add(
            format!(
                "Your smoldering fire ring singed the {} for {} HP",
                objects[monster_id].name, fire_damage
            ),
            LIGHT_BLUE,
        );
        // Tile cools down to normal after dealing damage to a monster
        game.map[monster_x as usize][monster_y as usize].on_fire.0 = false;
    }
}

/// Main game loop for testing
///
fn main() {
    tcod::system::set_fps(LIMIT_FPS);

    // Root setup
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust/libtcod tutorial")
        .init();

    // Tcod struct setup
    let mut tcod = Tcod {
        root,
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    };

    // Create the PLAYER
    let mut player = Object::new(0, 0, '@', "player", WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter {
        max_hp: 40,
        hp: 40,
        defense: 2,
        power: 5,
        on_death: DeathCallback::Player,
    });

    // Vector for all game objects
    let mut objects = vec![player];

    // Game struct with map, messages, inventory
    let mut game = Game {
        map: make_map(&mut objects),
        messages: Messages::new(),
        inventory: vec![],
    };

    // Initial map setup
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x,
                y,
                !game.map[x as usize][y as usize].block_sight,
                !game.map[x as usize][y as usize].blocked,
            );
        }
    }

    // Initialize PLAYER's previous position for later use
    let mut previous_player_position = (-1, -1);

    // Welcome message
    game.messages.add("Welcome to Roguelike!", BLUE);

    // Game loop
    while !tcod.root.window_closed() {
        // Clear previous frame
        tcod.con.clear();

        // Render the current game state
        let fov_recompute = previous_player_position != (objects[PLAYER].pos());
        render_all(&mut tcod, &mut game, &objects, fov_recompute);
        tcod.root.flush();

        // Handle PLAYER keys for movement and attacking
        previous_player_position = objects[PLAYER].pos();
        let player_action = handle_keys(&mut tcod, &mut game, &mut objects);
        if player_action == PlayerAction::Exit {
            break;
        }

        // Monster turn
        if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &tcod, &mut game, &mut objects);
                }
            }
        }

        // Ensures that the PLAYER is a '@' if still alive
        if objects[PLAYER].alive {
            objects[PLAYER].char = '@';
        }

        // Check if PLAYER has moved to a teleporting Tile on game map
        check_teleport(&mut game.map, &mut objects[PLAYER]);
    }
}
