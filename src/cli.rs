use std::path::PathBuf;

use clap::Parser;

use crate::convert_0_1::convert_0_1;
use crate::gui::init::launch_gui;

pub fn cli() {
    let args = Args::parse();

    if args.convert_0_1 {
        convert_0_1(args);
    } else {
        launch_gui(args);
    }
}

/// mzd2
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Convert given mzd 0.1 map to current format. Does not launch GUI
    #[arg(long="convert-0.1")]
    pub convert_0_1: bool,

    /// Asset to open (map or tileset)
    #[arg()]
    pub load_paths: Vec<PathBuf>,
}
