use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
/// Build System for Quantum OS
pub struct CommandLine {
    /// Build Option
    #[command(subcommand)]
    pub option: Option<TaskOption>,

    /// Enable Qemu's KVM
    #[arg(short, long, default_value_t = false)]
    pub enable_kvm: bool,

    /// Print all interrupts to std out
    #[arg(short, long = "log-int", default_value_t = false)]
    pub log_interrupts: bool,

    /// Print std out to command-line
    #[arg(short, long = "nographic", default_value_t = false)]
    pub no_graphic: bool,

    /// Slow down the emulator with a modifier
    #[arg(short, long = "slow")]
    pub slow_emulator: Option<usize>,

    /// Use the bochs emulator
    #[arg(short, long = "bochs", default_value_t = false)]
    pub use_bochs: bool,

    /// Enable GDB for qemu
    #[arg(long = "gdb", default_value_t = false)]
    pub use_gdb: bool,

    /// Run clippy durning build
    #[arg(long = "clippy", default_value_t = false)]
    pub enable_clippy: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum TaskOption {
    /// Build Quantum OS
    Build,
    /// Run CI/CD Actions
    Actions,
    /// Run + Build Quantum OS
    Run,
    /// Run + Build Quantum OS (with multiboot support from qemu)
    RunQuick,
    /// Clean up all build artifacts
    Clean,
    /// Build QMK Disk Image
    BuildDisk,
    /// Emit asm for this crate
    AsmAt { file: String, ip: String },
}
