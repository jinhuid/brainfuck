use std::env;
use std::process::Command;
use std::time::Instant;

fn run_binary(
    binary_name: &str,
    bf_file: &str,
    iterations: usize,
) -> Result<u128, Box<dyn std::error::Error>> {
    let mut total_time = 0u128;

    for _ in 0..iterations {
        let start = Instant::now();

        let output = Command::new(format!("./target/release/{}", binary_name))
            .arg(bf_file)
            .output()?;

        let elapsed = start.elapsed().as_nanos();
        total_time += elapsed;

        if !output.status.success() {
            eprintln!(
                "Error running {}: {}",
                binary_name,
                String::from_utf8_lossy(&output.stderr)
            );
            return Err("Binary execution failed".into());
        }
    }

    Ok(total_time / iterations as u128)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <brainfuck_file> [iterations]", args[0]);
        eprintln!("Example: {} hello.bf 100", args[0]);
        std::process::exit(1);
    }

    let bf_file = &args[1];
    let iterations = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);

    println!("Benchmarking Brainfuck Interpreter Implementations");
    println!("=================================================");
    println!("File: {}", bf_file);
    println!("Iterations: {}", iterations);
    println!();

    // Build both binaries first
    println!("Building binaries...");
    let build_output = Command::new("cargo")
        .args(&["build", "--release", "--bin", "enum_version"])
        .output()?;

    if !build_output.status.success() {
        eprintln!(
            "Failed to build enum_version: {}",
            String::from_utf8_lossy(&build_output.stderr)
        );
        return Err("Build failed".into());
    }

    let build_output = Command::new("cargo")
        .args(&["build", "--release", "--bin", "trait_version"])
        .output()?;

    if !build_output.status.success() {
        eprintln!(
            "Failed to build trait_version: {}",
            String::from_utf8_lossy(&build_output.stderr)
        );
        return Err("Build failed".into());
    }

    println!("Build completed successfully!");
    println!();

    // Warm up runs
    println!("Performing warm-up runs...");
    for _ in 0..5 {
        let _ = Command::new("./target/release/enum_version")
            .arg(bf_file)
            .output();
        let _ = Command::new("./target/release/trait_version")
            .arg(bf_file)
            .output();
    }

    println!("Running benchmarks...");
    println!();

    // Benchmark enum version
    print!("Testing enum implementation... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    let enum_time = run_binary("enum_version", bf_file, iterations)?;
    println!("Done");

    // Benchmark trait version
    print!("Testing trait implementation... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    let trait_time = run_binary("trait_version", bf_file, iterations)?;
    println!("Done");

    println!();
    println!("Results");
    println!("=======");
    println!(
        "Enum implementation:  {:>10} ns ({:>8.3} μs)",
        enum_time,
        enum_time as f64 / 1_000.0
    );
    println!(
        "Trait implementation: {:>10} ns ({:>8.3} μs)",
        trait_time,
        trait_time as f64 / 1_000.0
    );
    println!();

    let ratio = trait_time as f64 / enum_time as f64;
    if ratio > 1.0 {
        println!("📊 Enum is {:.2}x faster than trait", ratio);
        println!("⚡ Performance improvement: {:.1}%", (ratio - 1.0) * 100.0);
    } else {
        println!("📊 Trait is {:.2}x faster than enum", 1.0 / ratio);
        println!(
            "⚡ Performance improvement: {:.1}%",
            (1.0 / ratio - 1.0) * 100.0
        );
    }

    println!();
    println!("Summary");
    println!("=======");
    if ratio > 1.05 {
        println!("✅ Enum implementation shows significant performance advantage");
    } else if ratio < 0.95 {
        println!("✅ Trait implementation shows significant performance advantage");
    } else {
        println!("⚖️  Both implementations have similar performance");
    }

    Ok(())
}
