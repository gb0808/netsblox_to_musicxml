use std::io::Read;

fn main() {
    // read in an xml file whose path is given as a command line argument
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        panic!("usage: {} [xml file]", &args[0]);
    }
    let mut xml = String::new();
    std::fs::File::open(&args[1]).expect("failed to open file").read_to_string(&mut xml).expect("failed to read file");

    println!("{}", netsblox_to_musicxml::netsblox_to_musicxml(&xml));    
    
}