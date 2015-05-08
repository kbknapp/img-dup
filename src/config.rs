use serialize::json::{ToJson, Json};

use std::borrow::ToOwned;

use std::collections::BTreeMap;

use std::fmt::{Show, Formatter};
use std::fmt::Result as FormatResult;

use std::io::fs::PathExtensions;

use std::os;

use clap::{App, Arg, ArgMatches};

#[derive(Copy, Clone)]
pub struct ProgramSettings {
    pub threads: uint,
    pub dir: Path,
    pub recurse: bool,
    pub exts: Vec<String>,
    pub hash_size: u32,
    pub threshold: f32,
    pub fast: bool,
    pub outfile: Option<Path>,
    pub dup_only: bool,
    pub limit: uint,
    pub json: JsonSettings,
	pub gui: bool,
}

unsafe impl Send for ProgramSettings {}

impl ProgramSettings {

    fn new() -> ArgMatches {

        App::new("img-dup")
            .version(&format!("v{}", crate_version!()))
            .about("Tool for finding duplicate images")
            .args_from_usage("-t --threads [THREADS] 'How many threads the program should use \
                                                      to process images.\n\
                                                      Defaults to the number of cores reported \
                                                      by the OS.'
                             -d --dir [DIR]          'The directory the program should search \
                                                      in.\n\
                                                      Default is the current working \
                                                      directory.'
                            -r --recursive           'If present, the program will search \
                                                      subdirectories.'
                            -h --hash-size           'Helps the program decide the number of bits \
                                                      to use for the hash.\n\
                                                      A higher number means more detail, but \
                                                      greater memory usage.\n\
                                                      Default is 8'
                            -s --threashold [THRESHOLD] 'The amount in percentage that an image \
                                                      must be different from another to qualify \
                                                      as unique.\n\
                                                      Default is 3'
                            -f --fast                'Use a faster, less accurate algorithm. \
                                                      Really only useful for finding duplicates.\n\
                                                      Using a low threshold and/or a larger hash \
                                                      is recommended.'
                            -e --ext [EXT]...        'Search for filenames with the given \
                                                      extension.\n\
                                                      Defaults are jpeg, jpg, png, and gif.'
                            -o --outfile [FILE]      'Output to the given file. If omitted, will \
                                                      print to stdout.\n\
                                                      If not absolute, it will be relative to the \
                                                      search directory.'
                            -u --dup-only            'Only output images with similars or \
                                                      duplicates.'
                            -l --limit [LIMIT]       'Only process the given number of images.'
                            -j --json [SPACES]       'Output the results in JSON format.\n\
                                                      If outputting to stdout, normal output is \
                                                      suppressed.\n\
                                                      SPACES indicates the number of spaces \
                                                      to indent per level. If 0, the JSON \
                                                      will be in compact format.\n\
                                                      See the README for details.'
                         -g --gui                     'Open the GUI. Given command-line flags \
                                                       will be set in the configuration dialog.'")
                        .get_matches()
    }

    pub fn hash_settings(&self) -> HashSettings {
        HashSettings {
            hash_size: self.hash_size,
            fast: self.fast,
        }
    }

    pub fn silent_stdout(&self) -> bool {
        self.outfile.is_none() && self.json.is_json()
    }
}

impl Show for ProgramSettings {
    fn fmt(&self, fmt: &mut Formatter) -> FormatResult {
        try!(writeln!(fmt, "Threads: {}", self.threads));
        try!(writeln!(fmt, "Directory: {}", &self.dir.display()));
        try!(writeln!(fmt, "Recursive: {}", self.recurse));
        try!(writeln!(fmt, "Extensions: {}", self.exts.as_slice()));
        try!(writeln!(fmt, "Hash size: {}", self.hash_size));
        try!(writeln!(fmt, "Threshold: {0:.2}%", self.threshold * 100f32));
        writeln!(fmt, "Fast: {}", self.fast)
    }
}

impl ToJson for ProgramSettings {

    fn to_json(&self) -> Json {
        let mut my_json = BTreeMap::new();
        json_insert!(my_json, "threads", self.threads);
        json_insert!(my_json, "dir", self.dir.display().to_string());
        json_insert!(my_json, "recurse", self.recurse);
        json_insert!(my_json, "exts", self.exts.as_slice());
        json_insert!(my_json, "hash_size", self.hash_size);
        json_insert!(my_json, "threshold", self.threshold);
        json_insert!(my_json, "fast", self.fast);
        json_insert!(my_json, "limit", self.limit);

        Json::Object(my_json)
    }
}

#[derive(Copy, Clone)]
pub struct HashSettings {
    pub hash_size: u32,
    pub fast: bool,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum JsonSettings {
    NoJson,
    CompactJson,
    PrettyJson(uint),
}

impl JsonSettings {
    pub fn is_json(&self) -> bool {
        *self != JsonSettings::NoJson
    }
}

impl FromStr for JsonSettings {
    type Err = JsonSettings;

    fn from_str(&self) -> Result<Self, Self::Err> {
        match self.parse::<u32>() {
            Ok(spaces) if spaces > 0 => Ok(JsonSettings::PrettyJson(spaces)),
            Ok(_) => Ok(JsonSettings::CompactJson),
            Err(_)     => Err(JsonSettings::NoJson)
        }
    }
}

pub fn parse_args() -> ProgramSettings {
    let matches = ProgramSettings::new();

    let exts_default = vec!("jpeg", "jpg", "png");

    let dir = dir_arg(matches.value_of("DIR"));

    ProgramSettings {
        threads: value_t!(matches.value_of("THREADS"), u32).unwrap_or(os::num_cpus()),
        dir: dir.clone(),
        recurse: matches.is_present("recursive"),
        hash_size: value_t!(matches.value_of("hash-size"), u32).unwrap_or(8u32),
        threshold: value_t!(matches.value_of("THRESHOLD"), f32).unwrap_or(3f32).abs() / 100f32,
        fast: matches.is_present("fast"),
        exts: matches.values_of("EXT")
                     .unwrap_or(exts_default)
                     .iter()
                     .map(ToOwned::to_owned)
                     .collect(),
        outfile: outfile_arg(matches.value_of("FILE"), &dir),
        dup_only: matches.is_present("dup-only"),
        limit: value_t!(matches.value_of("LIMIT"), u32).unwrap_or(0u32),
        json: value_t!(matches.value_of("SPACES"), JsonSettings).unwrap(),
		gui: matches.is_present("gui"),
    }
}

fn dir_arg(arg: Option<&str>) -> Path {
    let dir = arg.map_or( os::get_cwd(), |path| Path::new(path) );

    if !dir.is_dir() {
        println!("'{}' is not a valid directory", arg.unwrap_or(""));
        std::process::exit(1);
    }

    dir
}

fn outfile_arg(arg: Option<&str>, dir: &Path) -> Option<Path> {
    arg.map(|path| {
        let path = Path::new(path);
        if path.is_relative() {
            dir.join(path)
        } else {
            path
        }
    })
}
