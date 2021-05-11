extern crate ncurses;
use ncurses::*;
use std::io::{self, Read};

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
    fn wall() -> Self {
        Tile {
            ascii: '#',
            walkable: false,
        }
    }
}


type Map = Vec<Vec<Tile>>;

fn make_map() -> Map {
    let mut map: Vec<Vec<Tile>> =  Vec::new();

    for _ in 0..=MAP_HEIGHT {
        map.push(Vec::new());
    }
    for x in 0..=MAP_HEIGHT {
        for y in 0..=MAP_WIDTH {
            if x == 0 || y == 0 || x == MAP_HEIGHT || y == MAP_WIDTH {
                map[x as usize].push(Tile::wall());
            } else {
                map[x as usize].push(Tile::empty());
            }
        }
    }
    map
}

fn render(game: Game) {
    for y in 0..=MAP_HEIGHT {
        for x in 0..=MAP_WIDTH {
            let curr_tile = game.map[y as usize][x as usize];
            print!("{}", curr_tile.ascii);
        }
        println!("");
    }
}


fn main() {
    /*
    let mut game = Game {
        map: make_map(),
    };
    render(game);
    */

    /* Print to the back buffer. */


    let mut v: Vec<i32> = Vec::new();
    let mut count = 0;
        initscr();
    while count < 5 {
        /* Update the screen. */
        refresh();
        println!("hellllloooooo,world!!!"); 

        /* Wait for a key press. */
        let p = getch();
        refresh();
        v.push(p);
        count += 1;
        
        /* Terminate ncurses. */
    }
        endwin();
    println!("{:?}", v);



    

}
