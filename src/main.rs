
fn main() -> Result<(), ()> {
    let mut app: lens::App = lens::App::new();
    let file = "testfiles/smb.nes";
    println!("Loading {}!", file);
    let rom_data: Vec<u8> = std::fs::read("testfiles/smb.nes").expect("Failed to read ROM file");
    println!("Successfully loaded {} bytes!", rom_data.len());
    let err = app.load_rom(rom_data);
    if let Err(e) = err {
        println!("Err! {:?}", e);
        return Err(());
    }
    app.init();
    while true {
        app.step();
    }
    Ok(())
}
