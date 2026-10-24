//! Command-line access to Looping documents.

use loo_format::Document;
use looping_core::{render, sql, Dbms, Mcd, Mld};

const USAGE: &str = "usage: looping-cli <json|mcd|mld|sql|xml|script|svg|png|resave> FILE [OUT] [--dbms N] [--set OFFSET=VALUE]... [--set-str OFFSET=TEXT]...";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
    let dbms = args
        .iter()
        .position(|a| a == "--dbms")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .map(Dbms::from_index);
    let out = args.get(3).filter(|a| !a.starts_with("--"));
    let mut doc = Document::open(&args[2]).unwrap_or_else(|e| {
        eprintln!("{}: {e}", args[2]);
        std::process::exit(1)
    });
    // --set 0x52c=3: overwrite an i32 of the settings block (CLoopingDoc offset)
    for (i, _) in args.iter().enumerate().filter(|(_, a)| *a == "--set") {
        let (off, val) = args.get(i + 1).and_then(|kv| kv.split_once('=')).expect(USAGE);
        let num = |s: &str| match s.strip_prefix("0x") {
            Some(h) => i64::from_str_radix(h, 16),
            None => s.parse(),
        };
        doc.settings.set_i32(num(off).expect(USAGE) as usize, num(val).expect(USAGE) as i32);
    }
    // --set-str 0x7cc=[Classe]_[FK]: overwrite a UTF-16 string of the settings block
    for (i, _) in args.iter().enumerate().filter(|(_, a)| *a == "--set-str") {
        let (off, val) = args.get(i + 1).and_then(|kv| kv.split_once('=')).expect(USAGE);
        let off = usize::from_str_radix(off.trim_start_matches("0x"), 16).expect(USAGE);
        doc.settings.set_wstr(off, val, 16);
    }
    let dbms = dbms.unwrap_or_else(|| Dbms::from_index(doc.settings.i32(0x52c)));
    let mcd = Mcd::from_document(&doc);
    match args[1].as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&doc).unwrap()),
        "mcd" => println!("{}", serde_json::to_string_pretty(&mcd).unwrap()),
        "mld" => {
            let mld = Mld::from_mcd(&mcd);
            print!("{}", mld.to_text(&sql::SqlOptions { dbms, ..sql::SqlOptions::from_settings(&doc.settings) }));
            for w in &mld.warnings {
                eprintln!("warning: {w}");
            }
        }
        "sql" => {
            let opts = sql::SqlOptions { dbms, ..sql::SqlOptions::from_settings(&doc.settings) };
            print!("{}", sql::generate_with(&Mld::from_mcd(&mcd), &opts))
        }
        // export files as Looping writes them (ISO-8859-1, CRLF)
        "xml" | "script" => {
            let opts = sql::SqlOptions { dbms, ..sql::SqlOptions::from_settings(&doc.settings) };
            let mld = Mld::from_mcd(&mcd);
            let text = if args[1] == "xml" {
                let title = std::path::Path::new(&args[2]).file_stem().map(|s| s.to_string_lossy().into_owned());
                sql::xml(&mld, &opts, &title.unwrap_or_default())
            } else {
                sql::script_file(&mld, &opts)
            };
            let text = text.unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1)
            });
            let bytes = sql::to_latin1_crlf(&text);
            match out {
                Some(o) => std::fs::write(o, bytes).unwrap(),
                None => std::io::Write::write_all(&mut std::io::stdout(), &bytes).unwrap(),
            }
        }
        // boxes grown to their content, as Looping does when it opens a document
        "svg" => {
            looping_core::edit::fit_all(&mut doc);
            print!("{}", render::build(&doc).to_svg())
        }
        "png" => {
            looping_core::edit::fit_all(&mut doc);
            let png = render::build(&doc).to_png().expect("png");
            std::fs::write(out.expect(USAGE), png).unwrap();
        }
        "resave" => std::fs::write(out.expect(USAGE), doc.save().unwrap()).unwrap(),
        _ => eprintln!("{USAGE}"),
    }
}
