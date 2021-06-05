// This file is generated automatically. Do not edit it directly.
// See the Contributing section in README on how to make changes to it.
use rand::Rng;
use std::cmp;
use tcod::colors::*;
use tcod::console::*;
use tcod::map::{ FovAlgorithm, Map as FovMap };

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 15;

// actual size of the window
const SCREEN_WIDTH: i32 = 120;
const SCREEN_HEIGHT: i32 = 80;
// size of the map
const MAP_WIDTH: i32 = 120;
const MAP_HEIGHT: i32 = 60;
const ROOM_MAX_SIZE: i32 = 20;
const ROOM_MIN_SIZE: i32 = 5;
const MAX_ROOMS: i32 = 30;
const MAX_ROOM_MONSTERS = 2;

const LIMIT_FPS: i32 = 20; // 20 frames-per-second maximum
const PLAYER: usize = 0;

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
const COLOR_LIGHT_GROUND: Color = Color {
    r: 70,
    g: 140,
    b: 40,
};
const COLOR_LIGHT_TELEPORT: Color = Color { 
    r: 0, 
    g: 0, 
    b: 225, 
};
const COLOR_DARK_TELEPORT: Color = Color {
    r: 0, 
    g: 0, 
    b: 130, 
};


struct Tcod {
    root: Root,
    con: Offscreen,
    fov: FovMap,
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
    explored: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            perimeter: false,
            teleport: false,
            explored: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            perimeter: false,
            teleport: false,
            explored: false,
        }
    }
    pub fn perimeter() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            perimeter: true,
            teleport: false,
            explored: false,
        }
    }
    pub fn teleport() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            perimeter: false,
            teleport: true,
            explored: false,
        }
    }
    pub fn is_teleportable_to(&self) -> bool {
        if self.blocked == false && self.teleport == false {
            true
        } else {
            false
        }
    }
    pub fn is_empty(&self) -> bool {
        !self.blocked
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

/// This is a generic object: the player, a monster, an item, the stairs...
/// It's always represented by a character on screen.
#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    char: char,
    color: Color,
    name: String,
    blocks: bool,
    alive: bool,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object { 
            x: x,
            y: y,
            char: char,
            color: color,
            name: name.into(),
            blocks: blocks,
            alive: false,
        }
    }


    /// set the color and then draw the character that represents this object at its position
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }
    
    /// returns the current position of the object
    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    /// sets the position of an object
    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // checks if the tile is blocking
    if map[x as usize][y as usize].blocked {
        return true;
    }

    // checks if an object/item is blocking
    let blocking: bool = false;

    for object in object.iter()
        if object.blocks && object.pos() == (x, y) {
            blocking = true;
            break;
        }
    blocking
}

/// move by the given amount, if the destination is not blocked
pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

fn make_map(objects: &mut Vec<Object>) -> Map {
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
            place_objects(new_room, objects);
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
    // Get a random room to place the player in
    let mut random_room_number = rand::thread_rng().gen_range(0, rooms.len());
    let mut center: (i32, i32) = rooms[random_room_number].center();
    objects[PLAYER].set_pos(center.0, center.1);

    // Get a random room to place a teleport tile in
    let mut random_room_number = rand::thread_rng().gen_range(0, rooms.len());
    center = rooms[random_room_number].center();
    map[center.0 as usize][center.1 as usize] = Tile::teleport();

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
        map[x as usize][y as usize] = Tile::empty();
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
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {
    if fov_recompute {
        // recompute FOV if necessary
        let player = &objects[PLAYER];
        tcod.fov
            .compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
    }
    // go through all tiles, and set their background color
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);
            let wall = game.map[x as usize][y as usize].block_sight;
            let perimeter = game.map[x as usize][y as usize].perimeter;
            let teleport = game.map[x as usize][y as usize].teleport;

            let color = match (visible, wall, perimeter, teleport) {
                // Outside player's FOV
                (false, true, true, false) => COLOR_DARK_PERIMETER,
                (false, true, false, false) => COLOR_DARK_WALL,
                (false, false, false, true) => COLOR_DARK_TELEPORT,
                (false, false, false, false) => COLOR_DARK_GROUND,
                // Inside player's FOV
                (true, true, true, false) => COLOR_LIGHT_PERIMETER,
                (true, true, false, false) => COLOR_LIGHT_WALL,
                (true, false, false, true) => COLOR_LIGHT_TELEPORT,
                (true, false, false, false) => COLOR_DARK_GROUND,
                _ => COLOR_DARK_PERIMETER,
            };

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

    // draw all objects in the list
    for object in objects {
        if tcod.fov.is_in_fov(object.x, object.y) {
            object.draw(&mut tcod.con);
        }
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

fn check_teleport(map: &mut Map, player: &mut Object) {
    // Check if player is on a teleport tile
    if map[player.x as usize][player.y as usize].teleport == true {
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
        place_rand_teleport_tile(map, player);
    }
}

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

fn place_rand_teleport_tile(map: &mut Map, player: &Object) {
    loop {
        let tile: (i32, i32) = get_tile_non_passage_blocking(map);
        if map[tile.0 as usize][tile.1 as usize].is_teleportable_to() {
            map[tile.0 as usize][tile.1 as usize].teleport = true;
            break;
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

fn place_object(room: Rect, object: &mut Vec<Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        let mut monster = if rand::random::<f32>() < 0.8 {
            // create orc
            Object::new(x, y, 'o', colors::DESATURATED_GREEN)
        } else {
            Object::new(x, y, 'T', colors::DARKER_GREEN)
        };

        objects.push(monster);
    }
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

    let mut tcod = Tcod { 
        root, 
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    };

    // create object representing the player
    let player = Object::new(0, 0, '@', WHITE);

    // the list of objects with those two
    let mut objects = [player, npc];

    let mut game = Game {
        // generate map (at this point it's not drawn to the screen)
        map: make_map(&mut objects),
    };
    
    // populate the FOV map, according to the generated map
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

    let mut previous_player_position = (-1, -1);

    while !tcod.root.window_closed() {
        // clear the screen of the previous frame
        tcod.con.clear();

        // render the screen
        let fov_recompute = previous_player_position != (objects[PLAYER].x, objects[PLAYER].y);
        render_all(&mut tcod, &mut game, &objects, fov_recompute);

        tcod.root.flush();

        // handle keys and exit game if needed
        let player = &mut objects[PLAYER];

        // get the current position of the player before a potential move in fn 'handle_keys'
        previous_player_position = (player.x, player.y);

        let exit = handle_keys(&mut tcod, &game, player);
        if exit {
            break;
        }
        check_teleport(&mut game.map, player);
    }
}
