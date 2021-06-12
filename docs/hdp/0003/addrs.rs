const GIB: u64 = 1u64 << 30;
const TIB: u64 = 1u64 << 40;

fn main() {
    println!("{:#016X} -512 GiB", 0_u64.wrapping_sub(512 * GIB));
    println!("{:#016X} -1 TiB ", 0_u64.wrapping_sub(TIB));
    println!("{:#016X} -1 TiB - 512 GiB", 0_u64.wrapping_sub(3 * 512 * GIB));
    for k in 2..=16 {
        println!("{:#016X} -{} TiB", 0_u64.wrapping_sub(k * TIB), k);
    }
}
