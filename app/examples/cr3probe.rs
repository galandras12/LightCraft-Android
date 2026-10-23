//! `cargo run --release --example cr3probe -- file.CR3`: what the raw decoder says about a file.
fn main() {
    let path = std::env::args().nth(1).expect("file");
    let bytes = std::fs::read(&path).expect("read");
    println!("{} bytes", bytes.len());
    println!("probe: {:?}", lightcraft_raw::probe_info(&bytes).map(|i| format!("{i:?}")));
    match lightcraft_raw::decode(&bytes) {
        Ok(img) => {
            println!("decode ok: {}x{} black={:?} white={:?}", img.width, img.height, img.black, img.white);
            if let lightcraft_raw::RawData::U16(v) = &img.data {
                let (mn, mx) = v.iter().fold((u16::MAX, 0u16), |(a, b), &x| (a.min(x), b.max(x)));
                let mean = v.iter().map(|&x| x as f64).sum::<f64>() / v.len() as f64;
                let at_max = v.iter().filter(|&&x| x >= mx.saturating_sub(2)).count();
                println!("samples min={mn} max={mx} mean={mean:.0} near-max={:.1}%", 100.0 * at_max as f64 / v.len() as f64);
            }
        }
        Err(e) => println!("decode error: {e}"),
    }
}
