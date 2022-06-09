use psqs::queue::local::LocalQueue;
use rust_semp::{
    optimize::{frequency::Frequency, Optimize},
    *,
};

fn main() {
    let freq = Frequency::new(
        rust_pbqff::config::Config::load("test_files/pbqff.toml"),
        rust_pbqff::Intder::load_file("test_files/intder.in"),
        rust_pbqff::Spectro::load("test_files/spectro.in"),
        vec![],
    );
    setup();
    let queue = LocalQueue::new("inp", 128);
    freq.num_jac(
        &Vec::new(),
        &"USS            H    -11.246958000000
    ZS             H      1.268641000000
    BETAS          H     -8.352984000000
    GSS            H     14.448686000000
    USS            C    -51.089653000000
    UPP            C    -39.937920000000
    ZS             C      2.047558000000
    ZP             C      1.702841000000
    BETAS          C    -15.385236000000
    BETAP          C     -7.471929000000
    GSS            C     13.335519000000
    GPP            C     10.778326000000
    GSP            C     11.528134000000
    GP2            C      9.486212000000
    HSP            C      0.717322000000
    FN11           C      0.046302000000"
            .parse()
            .unwrap(),
        &queue,
        0,
    );
}
