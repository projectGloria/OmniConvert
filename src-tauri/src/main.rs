// Release builds use the GUI subsystem so no console window pops up behind the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    omniconvert_lib::run()
}
