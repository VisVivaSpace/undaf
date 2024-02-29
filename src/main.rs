#![allow(unused)] // TODO remove later
//TODO remove panic!s

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

    let mut outfile = match  matches.get_one::<PathBuf>("output") {
        Some(ofile) => PathBuf::clone(ofile),
        None => PathBuf::from(r"undaf_output.ron"),
    };

    if outfile.is_dir() {
        outfile.push(r"undaf_output.ron");
    }

    if outfile.try_exists()? {
        panic!("Output file exists: {}",outfile.display());
    }
     
    let extension = match outfile.extension() {
        Some(ext) => match ext.to_str(){
            Some(s) => s,
            None => {panic!("Can't parse output file extension")}
        },
        None => {panic!("Output file must have a valid extension")},
    };

    let mut output = File::create(&outfile)?;

    //TODO make closure for writing to output file based on extension
    let writer = match extension {
        "ron" => |mut daf: DAFFile| -> Result<String> {
            let data = DAFData::from_daffile(&mut daf).unwrap();
            match ron::ser::to_string_pretty(&data,ron::ser::PrettyConfig::default()) {
                Ok(s) => Ok(s),
                Err(e) => Err(anyhow!(e))
            }
        },
        _ => {panic!("Unsuported output file extension: {}",extension)}
    };

    for infile in matches
        .get_many::<PathBuf>("input")
        .expect("Must specify input file(s).")
    {

        match DAFFile::from_file(File::open(&infile)?) {
            Err(why) => panic!("couldn't open {}: {}", infile.to_str().unwrap(), why),
            Ok(mut daf) => {
                //let data = DAFData::from_daffile(&mut daf).unwrap();
                //println!("{}",ron::to_string(&data).unwrap());
                println!("{}",writer(daf).unwrap());
            }
        }
    }

    Ok(())
}
