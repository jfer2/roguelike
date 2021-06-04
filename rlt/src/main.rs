// This file is generated automatically. Do not edit it directly.
// See the Contributing section in README on how to make changes to it.
use tcod::colors::*;
use tcod::console::*;
use rand::Rng;
use std::cmp;

// actual size of the window
const SCREEN_WIDTH: i32 = 120;
const SCREEN_HEIGHT: i32 = 80;

// size of the map
const MAP_WIDTH: i32 = 120;
const MAP_HEIGHT: i32 = 60;
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

const LIMIT_FPS: i32 = 20; // 20 frames-per-second maximum

const COLOR_DARK_WALL: Color = Color { r: 15, g: 15, b: 15 };
const COLOR_PERIMETER: Color = Color { r: 100, g: 100, b: 100 };
const COLOR_TELEPORT: Color = Color {r: 0, g: 0, b: 225};
const COLOR_DARK_GROUND: Color = Color {
    r: 40,
    g: 120,
    b: 70,
};

struct Tcod {
    root: Root,
    con: Offscreen,
}

type Map = Vec<Vec<Tile>>;

struct Game {
    map: Map,
}

/// A tile of the map and its properties
#[derive(Clone, Copy, Debug)]
struct Tile {
    blocked: bool,
    block_sight: bool,
    perimeter: bool,
    teleport: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            perimeter: false,
            teleport: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            perimeter: false,
            teleport: false,
        }
    }
    pub fn perimeter() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            perimeter: true,
            teleport: false,
        }
    }
    pub fn teleport() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            perimeter: false,
            teleport: true,
        }
    }
    pub fn is_teleportable_to(&self) -> bool {
        if self.blocked == false && self.teleport == false {
            true
        } else {
            false
        }
    }
}

/// A room on the map marked by x and y coordinates
///
///
/// 
#[derive(Clone, Copy, Debug)]
struct Room {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}


/// Room Example:
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
    pub fn new(x: i32, y: i32, width:i32, height: i32) -> Self {
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
        (self.x1 <= r.x2)
            && (self.x2 >= r.x1)
            && (self.y1 <= r.y2)
            && (self.y2 >= r.y1)
    }
}

fn create_room(room: Room, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

/// This is a generic object: the player, a monster, an item, the stairs...
/// It's always represented by a character on screen.
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, color: Color) -> Self {
        Object { x, y, char, color }
    }

    /// move by the given amount, if the destination is not blocked
    pub fn move_by(&mut self, dx: i32, dy: i32, game: &Game) {
        if !game.map[(self.x + dx) as usize][(self.y + dy) as usize].blocked {
            self.x += dx;
            self.y += dy;
        }
    }

    /// set the color and then draw the character that represents this object at its position
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }
}

fn make_map(player: &mut Object) -> Map {
    // fill map with "unblocked" tiles
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    // create walled perimeter in game map
    for tile in 0..MAP_WIDTH {
        map[tile as usize][0 as usize] = Tile::perimeter();
        map[tile as usize][(MAP_HEIGHT - 1) as usize] = Tile::perimeter();
    }
    for tile in 0..MAP_HEIGHT {
        map[0 as usize][tile as usize] = Tile::perimeter();
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
            let (new_x, new_y) = new_room.center();
            if !rooms.is_empty() {
                // all rooms after the first:
                // connect it to the previous room with a tunnel
                // center coordinates of the previous room
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                // toss a coin (random bool value -- either true or false)
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
    // Get a random room to place the main player in 
    let random_room_number = rand::thread_rng().gen_range(0, rooms.len());
    let center: (i32, i32) = rooms[random_room_number].center();
    player.x = center.0;
    player.y = center.1;
    map[(player.x + 1) as usize][(player.y + 1) as usize] = Tile::teleport();

    map
}

fn create_horizontal_passage(x1: i32, x2: i32, y: i32, map: &mut Map) {
    let passage: (i32, i32) = {
        if x1 <= x2 {
            (x1, x2)
        } else {
            (x2, x1)
        }
    };

    for x in passage.0..passage.1 + 1 {
        map[x as usize][y as usize] = Tile:: empty();
    }
}

fn create_vertical_passage(y1: i32, y2: i32, x: i32, map: &mut Map) {
    let passage: (i32, i32) = {
        if y1 <= y2 {
            (y1, y2)
        } else {
            (y2, y1)
        }
    };
    for y in passage.0..passage.1 + 1 {
        map[x as usize][y as usize] = Tile:: empty();
    }
}


fn render_all(tcod: &mut Tcod, game: &Game, objects: &[Object]) {
    // go through all tiles, and set their background color
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {

            let wall = game.map[x as usize][y as usize].block_sight;
            let perimeter = game.map[x as usize][y as usize].perimeter;
            let teleport = game.map[x as usize][y as usize].teleport;

            if perimeter {
                tcod.con
                    .set_char_background(x, y, COLOR_PERIMETER, BackgroundFlag::Set);
            }
            else if wall {
                tcod.con
                    .set_char_background(x, y, COLOR_DARK_WALL, BackgroundFlag::Set);
            } else if teleport {
                tcod.con
                    .set_char_background(x, y, COLOR_TELEPORT, BackgroundFlag::Set);
            }
            else {
                tcod.con
                    .set_char_background(x, y, COLOR_DARK_GROUND, BackgroundFlag::Set);
            }
        }
    }

    // draw all objects in the list
    for object in objects {
        object.draw(&mut tcod.con);
    }

    // blit the contents of "con" to the root console
    blit(
        &tcod.con,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );
}

fn check_teleport(game: &Game, player: &mut Object) {
    // Check if player is on a teleport tile
    if game.map[player.x as usize][player.y as usize].teleport == true {
        // Randomly selects a tiles, checks if its suitable to teleport to and changes
        // player's current location to that tile when found
        loop {
            let x = rand::thread_rng().gen_range(1, MAP_WIDTH - 1);
            let y = rand::thread_rng().gen_range(1, MAP_HEIGHT - 1);
            if game.map[x as usize][y as usize].is_teleportable_to() {
                player.x = x;
                player.y = y;
                break;
            }
        }
    }
}

fn handle_keys(tcod: &mut Tcod, game: &Game, player: &mut Object) -> bool {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    let key = tcod.root.wait_for_keypress(true);
    match key {
        Key {
            code: Enter,
            alt: true,
            ..
        } => {
            // Alt+Enter: toggle fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
        }
        Key { code: Escape, .. } => return true, // exit game

        // movement keys
        Key { code: Up, .. } => player.move_by(0, -1, game),
        Key { code: Down, .. } => player.move_by(0, 1, game),
        Key { code: Left, .. } => player.move_by(-1, 0, game),
        Key { code: Right, .. } => player.move_by(1, 0, game),

        _ => {}
    }

    false
}

fn main() {
    tcod::system::set_fps(LIMIT_FPS);

    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust/libtcod tutorial")
        .init();

    let con = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);

    let mut tcod = Tcod { root, con };

    // create object representing the player
    let player = Object::new(0, 0, '@', WHITE);

    // create an NPC
    let npc = Object::new(SCREEN_WIDTH / 2 - 5, SCREEN_HEIGHT / 2, '@', YELLOW);

    // the list of objects with those two
    let mut objects = [player, npc];

    let game = Game {
        // generate map (at this point it's not drawn to the screen)
        map: make_map(&mut objects[0]),
    };

    while !tcod.root.window_closed() {
        // clear the screen of the previous frame
        tcod.con.clear();

        // render the screen
        render_all(&mut tcod, &game, &objects);

        tcod.root.flush();

        // handle keys and exit game if needed
        let player = &mut objects[0];
        let exit = handle_keys(&mut tcod, &game, player);
        if exit {
            break;
        }
        check_teleport(&game, player);
    }
}

