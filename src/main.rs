#![allow(unused)] // TODO remove later

use crate::prelude::*;

use undaf::{DAFData, DAFFile, DAFHeader, DAFSegment};

mod error;
mod prelude;

fn main() -> std::io::Result<()> {
    let input_files = Arg::new("input")
        .value_name("FILE(S)")
        .value_parser(value_parser!(PathBuf))
        .required(true)
        .num_args(1..);

    let output_file = Arg::new("output")
        .value_parser(value_parser!(PathBuf))
        .long("output")
        .short('o');

    let app = Command::new("maud-dib")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Utility for converting NAIF SPICE files to other formats")
        .arg(input_files)
        .arg(output_file);

    let matches = app.get_matches();

    let outfile = matches.get_one::<PathBuf>("output");
    //TODO  logic here to open output file handle

    //TODO  switch to throwing error instead of expect
    for infile in matches
        .get_many::<PathBuf>("input")
        .expect("Must specify input file(s).")
    {

        match DAFFile::from_file(File::open(&infile)?) {
            Err(why) => panic!("couldn't open {}: {}", infile.to_str().unwrap(), why),
            Ok(mut daf) => {
                let data = DAFData::from_daffile(&mut daf).unwrap();
                println!("{}",ron::to_string(&data).unwrap());
            }
        }
    }

    Ok(())
}
