#[macro_use] extern crate errln;
#[macro_use] extern crate error_chain;
extern crate clap;
extern crate hex;
extern crate lalrpop_util;
extern crate parser_haskell;
extern crate regex;
extern crate tempdir;
extern crate walkdir;
extern crate corollary;

use parser_haskell::util::{print_parse_error, simplify_parse_error};

use clap::{Arg, App};
use regex::Regex;
use std::fmt::Write;
use std::fs::{File, create_dir_all};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempdir::TempDir;
use walkdir::WalkDir;

use corollary::print_item_list;
use corollary::ir::PrintState;

// Define error chain.
mod errors {
    error_chain! {
        foreign_links {
            Walkdir(::walkdir::Error);
            Io(::std::io::Error);
        }
    }
}
use errors::*;

#[test] #[ignore]
fn test_single_file() {
    use std::io::Write;

    let a = "./corrode/src/Language/Rust/Corrode/C.lhs";
    // let a = "./corrode/src/Language/Rust/Corrode/C.hs";
    // let a = "./test/input.hs";
    println!("file: {}", a);
    let mut file = File::open(a).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    if a.ends_with(".lhs") {
        contents = strip_lhs(&contents);
    }
    let contents = parser_haskell::preprocess(&contents);

    // let mut a = ::std::fs::File::create("temp.txt").unwrap();
    // a.write_all(contents.as_bytes());

    let mut errors = Vec::new();
    match parser_haskell::parse(&mut errors, &contents) {
        Ok(okay) => println!("{:#?}", okay),
        Err(e) => {
            let e = simplify_parse_error(e);
            print_parse_error(&contents, &e);
            panic!(e);
        }
    }
}

#[test]
fn test_no_regressions() {
    let a = vec![
        "./corrode/src/Language/Rust/AST.hs",
        "./corrode/src/Language/Rust/Corrode/C.lhs",
        "./corrode/src/Language/Rust/Corrode/CFG.lhs",
        "./corrode/src/Language/Rust/Corrode/CrateMap.hs",
        "./corrode/src/Language/Rust/Idiomatic.hs",
        "./corrode/src/Language/Rust.hs",

        "./language-c/src/Language/C/Analysis/AstAnalysis.hs",
        "./language-c/src/Language/C/Analysis/Builtins.hs",
        "./language-c/src/Language/C/Analysis/ConstEval.hs",
        "./language-c/src/Language/C/Analysis/Debug.hs",
        "./language-c/src/Language/C/Analysis/DeclAnalysis.hs",
        "./language-c/src/Language/C/Analysis/DefTable.hs",
        "./language-c/src/Language/C/Analysis/Export.hs",
        "./language-c/src/Language/C/Analysis/NameSpaceMap.hs",
        "./language-c/src/Language/C/Analysis/SemError.hs",
        "./language-c/src/Language/C/Analysis/SemRep.hs",
        "./language-c/src/Language/C/Analysis/TravMonad.hs",
        "./language-c/src/Language/C/Analysis/TypeCheck.hs",
        "./language-c/src/Language/C/Analysis/TypeConversions.hs",
        "./language-c/src/Language/C/Analysis/TypeUtils.hs",
        "./language-c/src/Language/C/Analysis.hs",
        "./language-c/src/Language/C/Data/Error.hs",
        "./language-c/src/Language/C/Data/Ident.hs",
        "./language-c/src/Language/C/Data/InputStream.hs",
        "./language-c/src/Language/C/Data/Name.hs",
        "./language-c/src/Language/C/Data/Node.hs",
        "./language-c/src/Language/C/Data/Position.hs",
        "./language-c/src/Language/C/Data/RList.hs",
        "./language-c/src/Language/C/Data.hs",
        "./language-c/src/Language/C/Parser/Builtin.hs",
        "./language-c/src/Language/C/Parser/ParserMonad.hs",
        "./language-c/src/Language/C/Parser/Tokens.hs",
        "./language-c/src/Language/C/Parser.hs",
        "./language-c/src/Language/C/Pretty.hs",
        "./language-c/src/Language/C/Syntax/AST.hs",
        "./language-c/src/Language/C/Syntax/Constants.hs",
        "./language-c/src/Language/C/Syntax/Ops.hs",
        "./language-c/src/Language/C/Syntax/Utils.hs",
        "./language-c/src/Language/C/Syntax.hs",
        "./language-c/src/Language/C/System/GCC.hs",
        "./language-c/src/Language/C/System/Preprocess.hs",

        "./gen/Lexer.hs",
        "./gen/Parser.hs",

        "./test/input.hs",
    ];

    for path in a {
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        if path.ends_with(".lhs") {
            contents = strip_lhs(&contents);
        }
        let contents = parser_haskell::preprocess(&contents);

        // Do not output preprocessed data temp.txt
        //println!("{:?}", path);
        use ::std::io::Write;
        // let mut a = ::std::fs::File::create("temp.txt").unwrap();
        // a.write_all(contents.as_bytes());

        let mut errors = Vec::new();
        match parser_haskell::parse(&mut errors, &contents) {
            Ok(v) => {
            }
            Err(e) => {
                //TODO print_parse_error return string, feed to panic
                print_parse_error(&contents, &simplify_parse_error(e));
                panic!("cannot convert file {:?}", path);
            }
        }
    }
}

fn strip_lhs(s: &str) -> String {
    let re = Regex::new(r"([ \t]*)```haskell([\s\S]*?)```").unwrap();
    let mut out = vec![];
    for cap in re.captures_iter(&s) {
        let indent = cap[1].to_string().len();
        let group = cap[2].to_string()
            .lines()
            .map(|x| {
                x.chars().skip(indent).collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        out.push(group);
    }

    out.join("\n\n")
}

/// Converts a Haskell file by its path into a Rust module.
fn convert_file(input: &str, p: &Path, inline_mod: bool) -> (String, String) {
    let mut contents = input.to_string();
    let mut file_out = String::new();
    let mut rust_out = String::new();

    // Parse out HASKELL /HASKELL RUST /RUST sections.
    let re = Regex::new(r#"HASKELL[\s\S]*?/HASKELL"#).unwrap();
    contents = re.replace(&contents, "").to_string();
    let re = Regex::new(r#"RUST([\s\S]*?)/RUST"#).unwrap();
    if let Some(cap) = re.captures(&contents) {
        rust_out.push_str(&cap.get(1).unwrap().as_str().to_string());
    }
    contents = re.replace(&contents, "").to_string();

    // Preprocess the file.
    let contents = parser_haskell::preprocess(&contents);

    // Parse the file.
    let mut errors = Vec::new();
    match parser_haskell::parse(&mut errors, &contents) {
        Ok(v) => {
            //errln!("{:?}", v);

            if inline_mod {
                let _ = writeln!(file_out, "pub mod {} {{", v.name.0.replace(".", "_"));
                let _ = writeln!(file_out, "    use haskell_support::*;");
                let _ = writeln!(file_out, "");
                let state = PrintState::new();
                let _ = writeln!(file_out, "{}", print_item_list(state.tab(), &v.items));
                let _ = writeln!(file_out, "}}\n");
            } else {
                let _ = writeln!(file_out, "use haskell_support::*;");
                let _ = writeln!(file_out, "");
                let state = PrintState::new();
                let _ = writeln!(file_out, "{}", print_item_list(state, &v.items));
            }
        }
        Err(e) => {
            errln!("/* ERROR: cannot convert file {:?}" ,p);
            // TODO have this write to Format
            print_parse_error(&contents, &simplify_parse_error(e));
            errln!("*/");
            panic!("COULDN'T PARSE");
        }
    }

    (file_out, rust_out)
}

quick_main!(run);

fn run() -> Result<()> {
    use std::io::Write;

    let matches = App::new("corollary")
        .version("0.1")
        .about("Converts Haskell to Rust")
        .arg(Arg::with_name("run")
            .short("r")
            .long("run")
            .help("Runs the file"))
        .arg(Arg::with_name("out")
            .short("o")
            .long("out")
            .help("Output path")
            .takes_value(true))
        .arg(Arg::with_name("recursive")
            .short("R")
            .long("recursive")
            .help("Recursively recreate folder structure, not a single file"))
        .arg(Arg::with_name("INPUT")
            .help("Sets the input file to use")
            .required(true)
            .index(1))
        .arg(Arg::with_name("alias")
            .long("alias")
            .help("Alias a file for a recursive output (virtual=actual)")
            .multiple(true)
            .takes_value(true))
        .get_matches();

    let arg_input = matches.value_of("INPUT").unwrap();
    let arg_run = matches.is_present("run");
    let arg_recursive = matches.is_present("recursive");
    let arg_out: Option<_> = matches.value_of("out");
    let arg_alias = matches.values_of("alias");

    if arg_run && arg_recursive {
        bail!("Cannot use --run and --recursive at the same time.");
    }
    if arg_alias.is_some() && !arg_recursive {
        bail!("Cannot use --alias without --recursive.")
    }
    if arg_recursive && arg_out.is_none() {
        bail!("Please specify an --out path to use --recursive.");
    }

    // Starting message.
    if arg_run {
        errln!("running {:?}...", arg_input);
    } else {
        errln!("cross-compiling {:?}...", arg_input);
    }

    let mut rust_section = "".to_string();
    let mut file_section = "".to_string();

    let _ = writeln!(file_section, "{}", include_str!("haskell_support.txt"));
    let _ = writeln!(file_section, "");

    let mut inputs: Vec<(PathBuf, PathBuf)> = WalkDir::new(arg_input).into_iter()
        .map(|entry| {
            let path_buf = entry.unwrap().path().to_owned();
            (path_buf.clone(), path_buf.clone())
        })
        .collect();

    if let Some(aliases) = arg_alias {
        for item in aliases {
            let mut item_parts = item.split("=");
            let value = item_parts.next().unwrap().to_owned();
            let key = item_parts.next().unwrap().to_owned();
            inputs.push((PathBuf::from(key), PathBuf::from(value)));
        }
    }

    for (source_path, virtual_path) in inputs {
        // Check filetype. Allow .lhs and .hs, ignore all else.
        let mut do_strip_lhs = false;
        if virtual_path.display().to_string().ends_with(".lhs") {
            do_strip_lhs = true;
        } else if !virtual_path.display().to_string().ends_with(".hs") {
            continue;
        }

        // Read file contents.
        let mut file = File::open(source_path.as_path())
            .chain_err(|| format!("Could not open {:?}", source_path))?;
        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Ok(..) => (),
            _ => continue,
        };

        // Preprocess the file.
        if do_strip_lhs {
            contents = strip_lhs(&contents);
        }
        let (file_out, rust_out) = convert_file(&contents, source_path.as_path(), !arg_recursive);

        // Switch on recursive switch.
        if arg_recursive {
            // Trim initial components.
            //TODO why three segments?
            let mut a = virtual_path.components();
            a.next();
            a.next();
            a.next();

            // Write out file.
            let t = format!("{}/{}", arg_out.unwrap(), a.as_path().display()).to_lowercase();
            let t = t.replace(".lhs", ".rs");
            let t = t.replace(".hs", ".rs");

            // Create directory.
            let _ = create_dir_all(&Path::new(&t).parent().unwrap());

            // Write out file.
            let mut f = File::create(&t)?;
            let _ = f.write_all(file_out.as_bytes());
            let _ = f.write_all(rust_out.as_bytes());
            drop(f);
        } else {
            // Accumulate file output.
            let _ = writeln!(file_section, "{}", file_out);
            rust_section.push_str(&rust_out);
        }
    }

    // If we have an output directory, we've already finished writing it.
    if !arg_recursive {
        // Add Rust segments RUST ... /RUST and Haskell support code.
        let _ = writeln!(file_section, "");
        let _ = writeln!(file_section, "");
        if rust_section.len() > 0 {
            let _ = writeln!(file_section, "{}", include_str!("haskell_support.txt"));
            let _ = writeln!(file_section, "/* RUST ... /RUST */");
            let _ = writeln!(file_section, "{}", rust_section);
        }

        if let Some(out_path) = arg_out {
            let mut f = File::create(&out_path)?;
            let _ = f.write_all(file_section.as_bytes());
        } else if !arg_run {
            // Print file to stdout.
            print!("{}", file_section);
        }

        // Evaluate --run
        if arg_run {
            // Run the file.
            let dir = TempDir::new("corollary")?;
            let file_path = dir.path().join("script.rs");

            let mut f = File::create(&file_path)?;
            let _ = f.write_all(file_section.as_bytes());
            drop(f);

            let output = Command::new("cargo")
                        .args(&["script", &file_path.display().to_string()])
                        .output()
                        .expect("failed to execute process");

            if !output.status.success() {
                err!("{}", String::from_utf8_lossy(&output.stderr));
            }
            err!("{}", String::from_utf8_lossy(&output.stdout));
            ::std::process::exit(output.status.code().unwrap());
        }
    }

    Ok(())
}
