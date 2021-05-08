
const MAP_WIDTH: i32 = 140;
const MAP_HEIGHT: i32 = 40;

struct Game {
    map: Map,
}

#[derive(Clone, Copy, Debug)]
struct Tile {
    ascii: char,
    walkable: bool,
}

impl Tile {
    fn empty() -> Self {
        Tile {
            ascii: '.',
            walkable: true,
        }
    }
}


type Map = Vec<Vec<Tile>>;

fn make_map() -> Map {
    let mut map: Map = vec![vec![Tile::empty(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
    map
}

fn render(game: Game) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let curr_tile = game.map[x as usize][y as usize];
            print!("{}", curr_tile.ascii);
        }
        println!("");
    }
}


fn main() {
    let game = Game {
        map: make_map(),
    };
    render(game);

}
