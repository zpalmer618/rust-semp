use nalgebra as na;
use psqs::geom::Geom;
use psqs::program::mopac::Params;
use std::cmp::Ordering;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::iter::zip;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Mutex, Once};
use std::thread::{self};
use symm::atom::Atom;
use symm::Irrep;

/// from [StackOverflow](https://stackoverflow.com/a/45145246)
#[macro_export]
macro_rules! string {
    // match a list of expressions separated by comma:
    ($($str:expr),*) => ({
        // create a Vec with this list of expressions,
        // calling String::from on each:
        vec![$(String::from($str),)*] as Vec<String>
    });
}

/// Take an INTDER-style `file07` file and parse it into a Vec of geometries
pub fn load_geoms(filename: &str, atom_names: &[String]) -> Vec<Geom> {
    let f = match File::open(filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to open {filename} with {e}");
            std::process::exit(1);
        }
    };
    let lines = BufReader::new(f).lines();
    let mut ret = Vec::new();
    let mut buf = Vec::new();
    let mut idx: usize = 0;
    for (i, line) in lines.map(|x| x.unwrap()).enumerate() {
        if line.contains("# GEOM") {
            if i > 0 {
                let mut mol = symm::Molecule::new(buf);
                mol.to_angstrom();
                ret.push(Geom::Xyz(mol.atoms));
                buf = Vec::new();
                idx = 0;
            }
            continue;
        }
        let fields: Vec<_> = line
            .split_whitespace()
            .map(|x| x.parse::<f64>().unwrap())
            .collect();
        buf.push(Atom::new_from_label(
            &atom_names[idx],
            fields[0],
            fields[1],
            fields[2],
        ));
        idx += 1;
    }
    let mut mol = symm::Molecule::new(buf);
    mol.to_angstrom();
    ret.push(Geom::Xyz(mol.atoms));
    ret
}

pub fn load_energies(filename: &str) -> na::DVector<f64> {
    let mut ret = Vec::new();
    let f = match File::open(filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "failed to open {filename} for reading energies with {e}"
            );
            std::process::exit(1);
        }
    };
    let lines = BufReader::new(f).lines();
    for line in lines.map(|x| x.unwrap()) {
        ret.push(line.trim().parse().unwrap());
    }
    na::DVector::from(ret)
}

/// Read `filename` into a string and then call [`parse_params`]
pub fn load_params(filename: &str) -> Params {
    let params = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {filename} with {e}");
            std::process::exit(1);
        }
    };
    parse_params(&params)
}

pub(crate) static DIRS: &[&str] = &["inp", "tmparam", "opt"];

/// set up the directories needed for the program after deleting existing ones
pub fn setup() {
    takedown();
    for dir in DIRS {
        match fs::create_dir(dir) {
            Ok(_) => (),
            Err(e) => {
                panic!("can't create '{dir}' for '{e}'");
            }
        }
    }
}

/// a struct for taking down in the background. the directory names sent should
/// have been moved to the trash directory but just the original names should be
/// sent
struct Takedown {
    /// channel for sending filenames to be deleted
    sender: Sender<String>,
}

lazy_static::lazy_static! {
    static ref DUMP_DEBUG: bool = std::env::var("DUMP_DEBUG").is_ok();
}

impl Takedown {
    fn new() -> Mutex<Takedown> {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            for dir in receiver {
                let root = Path::new("trash");
                let d = root.join(&dir);
                if let Err(e) = std::fs::remove_dir_all(&d) {
                    if *DUMP_DEBUG {
                        eprintln!("failed to remove {d:#?} with {e}");
                    }
                }
            }
        });

        Mutex::new(Takedown { sender })
    }

    pub(crate) fn send(&self, s: String) {
        self.sender.send(s).unwrap();
    }
}

lazy_static::lazy_static! {
    static ref DUMPER: Mutex<Takedown> = Takedown::new();
}

/// on first call, create the "trash" directory and/or move the existing trash
/// directory to it. on future calls, send DIRS to the global Takedown thread
pub fn takedown() {
    static INIT: Once = Once::new();
    let path = Path::new("trash");
    INIT.call_once(|| {
        let exists = path.exists();
        if exists {
            if let Err(e) = std::fs::rename(path, "trash1") {
                eprintln!("failed to rename trash with {e}");
            }
        }
        std::fs::create_dir("trash").expect("failed to initialize trash dir");
        if exists {
            if let Err(e) = std::fs::rename("trash1", "trash/trash1") {
                eprintln!("failed to relocate trash with {e}");
            }
            DUMPER.lock().unwrap().send("trash1".to_owned());
        }
    });
    let now = std::time::Instant::now();
    eprint!("starting takedown, ");
    for dir in DIRS {
        let path = Path::new(dir);
        if path.exists() {
            let to = format!(
                "{dir}{}",
                std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos()
            );
            if let Err(e) = std::fs::rename(path, Path::new("trash").join(&to))
            {
                eprintln!("failed to move {dir} to {to} with {e}");
            } else {
                DUMPER.lock().unwrap().send(to);
            }
        }
    }
    eprintln!(
        "finished after {:.1} s",
        now.elapsed().as_millis() as f64 / 1000.0
    );
}

/// parse a string containing lines like:
///
/// ```text
///   USS            H    -11.246958000000
///   ZS             H      1.268641000000
/// ```
/// into a vec of Params
pub fn parse_params(params: &str) -> Params {
    let lines = params.split('\n');
    let mut names = Vec::new();
    let mut atoms = Vec::new();
    let mut values = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() == 3 {
            names.push(fields[0].to_string());
            atoms.push(fields[1].to_string());
            values.push(fields[2].parse().unwrap());
        }
    }
    Params::from(names, atoms, values)
}

pub fn dump_vec<W: Write>(w: &mut W, vec: &na::DVector<f64>) {
    for (i, v) in vec.iter().enumerate() {
        writeln!(w, "{i:>5}{v:>20.12}").unwrap();
    }
}

pub fn dump_mat<W: Write>(w: &mut W, mat: &na::DMatrix<f64>) {
    let (rows, cols) = mat.shape();
    writeln!(w).unwrap();
    for i in 0..rows {
        write!(w, "{i:>5}").unwrap();
        for j in 0..cols {
            write!(w, "{:>12.8}", mat[(i, j)]).unwrap();
        }
        writeln!(w).unwrap();
    }
}

/// return `energies` relative to its minimum element
pub fn relative(energies: &na::DVector<f64>) -> na::DVector<f64> {
    let min = energies.min();
    let min = na::DVector::from(vec![min; energies.len()]);
    let ret = energies.clone();
    ret - min
}

pub(crate) fn log_params<W: Write>(w: &mut W, iter: usize, params: &Params) {
    let _ = writeln!(w, "Iter {iter}");
    let _ = writeln!(w, "{params}");
}

/// sort freqs by the irrep in the same position and then by frequency. if
/// `freqs` and `irreps` are not the same length, just return the sorted
/// frequencies.
pub fn sort_irreps(freqs: &[f64], irreps: &[Irrep]) -> Vec<f64> {
    if freqs.len() != irreps.len() {
        eprintln!("length mismatch {freqs:#?} vs {irreps:#?}");
        let mut ret = Vec::from(freqs);
        ret.sort_by(|a, b| a.partial_cmp(b).unwrap());
        return ret;
    }
    let mut pairs: Vec<_> = zip(irreps, freqs).collect();
    pairs.sort_by(|a, b| match a.0.cmp(b.0) {
        Ordering::Equal => a.1.partial_cmp(b.1).unwrap(),
        other => other,
    });
    pairs.iter().map(|x| *x.1).collect()
}
