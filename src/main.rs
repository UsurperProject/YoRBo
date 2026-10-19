mod config;
mod partitions;
mod toyboot;
mod vlnk;

use std::path::PathBuf;
use std::env::current_exe;

use anyhow::{Context, Result};
use input_macro::input;
use config::Config;
use clap::{Parser, ValueEnum};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Boot{
    Ventoy,
    Iso,
}

#[derive(Parser, Debug)]
struct Args{
    #[arg(short, long)]
    boot: Boot,
    #[arg(short, long)]
    file: Option<PathBuf>,
    #[arg(short, long)]
    reboot: bool
}

fn main() -> Result<()> {
    let args = Args::parse();
    if matches!(args.boot, Boot::Iso) && args.file.is_none() {
            eprintln!("Error: The --file or -f argument is required when --boot is set to 'iso'.");
            std::process::exit(1);
        }

    if !matches!(args.boot, Boot::Iso) && args.file.is_some() {
        eprintln!("Error: The --file or -f argument is only valid when --boot is set to 'iso'.");
        std::process::exit(1);
    }

    let mut cfg_path = current_exe().context("i got alzheimers")?;
    cfg_path.pop();
    cfg_path.push("config");
    let mut cfg: Config = confy::load_path(&cfg_path).unwrap_or_else(|_|{
        reconfigure()
    });
    println!("Config loaded");
    let mut toyboot_partition = partitions::find(cfg.partition.0, cfg.partition.1);
    if toyboot_partition.is_none() {
        cfg = reconfigure();
        toyboot_partition = partitions::find(cfg.partition.0, cfg.partition.1);
    }

    let toyboot_partition = toyboot_partition.unwrap();
    if !toyboot_partition.verify_format() {
        cfg = reconfigure();
    }
    println!("boot partition loaded"); 
    confy::store_path(&cfg_path, &cfg);
    let partition_letter = toyboot_partition.ensure_mounted().expect("Failed to mount partition");
    if args.boot == Boot::Iso {
        toyboot::set_iso_boot(partition_letter, args.file.unwrap());
    }

    else if args.boot == Boot::Ventoy {
        toyboot::set_ventoy_boot(partition_letter);
    }

    if args.reboot {
        toyboot::toyboot(partition_letter)?;
    }
    Ok(())
    
}

fn reconfigure() -> Config {
    let partitions = partitions::get_windows_partitions().expect("Failed to fetch partitions");
    println!("Failed to find toyboot partition. Please select the partition or reinstall:");
    for (i, partition) in partitions.iter().enumerate() {
        let mount_point = match partition.drive_letter {
            Some(c) => c.to_string(),
            None => String::from("Unmounted")
        };

        println!("{}. {} : {} mb {} (disk {})", 
        i+1, mount_point, partition.size / (1024 * 1024), partition.partition_type, partition.disk_number);
    }
    let selected_partition_num: usize = input!("").parse().expect("Please enter a valid number. quitting");
    let selected_partition = partitions.get(selected_partition_num -1).expect("Please enter a valid number. quitting");
    if selected_partition.verify_format() {
        return Config{
            partition: (selected_partition.disk_number, selected_partition.partition_number)
        };
    }
    return reconfigure();    

}